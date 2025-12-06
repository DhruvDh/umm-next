#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Python file representation and operations.

use std::{
    fmt::Display,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    parser::Parser,
    paths::ProjectPaths,
    queries::{CLASS_DEF_QUERY, FUNCTION_DEF_QUERY, IMPORT_QUERY, MAIN_BLOCK_QUERY},
};
use crate::{
    Dict, config,
    process::{self, StdinSource},
    types::LineRef,
};

/// Errors specific to Python file operations.
#[derive(Error, Debug)]
pub enum PythonFileError {
    /// Syntax error in the Python file.
    #[error("Syntax error in {file_name}:\n{message}")]
    SyntaxError {
        /// Name of the file with the error.
        file_name: String,
        /// Error message from Python.
        message:   String,
    },

    /// Runtime error during execution.
    #[error("Runtime error in {file_name}:\n{stacktrace}")]
    RuntimeError {
        /// Name of the file with the error.
        file_name:  String,
        /// Stack trace from Python.
        stacktrace: String,
        /// Diagnostics extracted from the error.
        diags:      Vec<LineRef>,
    },

    /// Import error.
    #[error("Import error in {file_name}:\n{message}")]
    ImportError {
        /// Name of the file with the error.
        file_name: String,
        /// Error message.
        message:   String,
    },

    /// Test failure.
    #[error("Test failure in {file_name}:\n{test_results}")]
    FailedTests {
        /// Name of the file with the error.
        file_name:    String,
        /// Test output.
        test_results: String,
        /// Diagnostics extracted from the failure.
        diags:        Vec<LineRef>,
    },

    /// Execution timeout.
    #[error("Execution timed out after {timeout:?}")]
    Timeout {
        /// How long we waited before timing out.
        timeout: Duration,
    },

    /// Unknown error.
    #[error("Unknown error: {0}")]
    Unknown(#[from] anyhow::Error),
}

/// Classification of Python source files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum FileType {
    /// Script with `if __name__ == "__main__":` block.
    Script,
    /// Regular Python module.
    #[default]
    Module,
    /// Test file (test_*.py or *_test.py).
    Test,
    /// Package init file (__init__.py).
    Package,
}

impl Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileType::Script => write!(f, "Script"),
            FileType::Module => write!(f, "Module"),
            FileType::Test => write!(f, "Test"),
            FileType::Package => write!(f, "Package"),
        }
    }
}

/// Represents a Python source file with parsed metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    /// Path to the Python file.
    path:        PathBuf,
    /// Filesystem name including `.py` extension.
    file_name:   String,
    /// Module name (dotted notation from package structure).
    module_name: String,
    /// Simple name without extension.
    name:        String,
    /// Classification of the file.
    kind:        FileType,
    /// Discovered imports.
    imports:     Vec<String>,
    /// Discovered function names.
    functions:   Vec<String>,
    /// Discovered class names.
    classes:     Vec<String>,
    /// Concise description of the file.
    description: String,
    /// Whether the file has a main block.
    has_main:    bool,
    #[serde(skip)]
    /// The parser for this file.
    parser:      Parser,
    /// Workspace paths associated with this file.
    paths:       ProjectPaths,
    /// Execution context used when running tools/scripts for this file.
    #[serde(skip)]
    context:     super::util::UvRunContext,
}

impl File {
    /// Creates a new File from a path.
    pub fn new(
        path: impl Into<PathBuf>,
        paths: ProjectPaths,
        context: super::util::UvRunContext,
    ) -> Result<Self> {
        let path = path.into();
        let file_name = path
            .file_name()
            .ok_or_else(|| anyhow!("Invalid file path: {:?}", path))?
            .to_string_lossy()
            .to_string();

        let name = file_name
            .strip_suffix(".py")
            .unwrap_or(&file_name)
            .to_string();

        // Read and parse the file
        let code = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read Python file: {:?}", path))?;
        let parser = Parser::new(code)?;

        // Determine module name from path relative to source directory
        let module_name = Self::compute_module_name(&path, &paths);

        // Determine file type
        let kind = Self::determine_file_type(&file_name, &parser);

        // Extract imports
        let imports = Self::extract_imports(&parser)?;

        // Extract functions
        let functions = Self::extract_functions(&parser)?;

        // Extract classes
        let classes = Self::extract_classes(&parser)?;

        // Check for main block
        let has_main = Self::has_main_block(&parser);

        // Build description
        let description = Self::build_description(&file_name, &kind, &functions, &classes);

        Ok(Self {
            path,
            file_name,
            module_name,
            name,
            kind,
            imports,
            functions,
            classes,
            description,
            has_main,
            parser,
            paths,
            context,
        })
    }

    /// Computes the dotted module name from the file path.
    fn compute_module_name(path: &Path, paths: &ProjectPaths) -> String {
        if let Ok(rel) = path.strip_prefix(paths.source_dir()) {
            let components: Vec<_> = rel
                .components()
                .filter_map(|c| c.as_os_str().to_str())
                .collect();

            if components.is_empty() {
                return String::new();
            }

            let mut parts: Vec<String> = components.iter().map(|s| s.to_string()).collect();

            // Remove .py extension from the last component
            if let Some(last) = parts.last_mut()
                && let Some(stripped) = last.strip_suffix(".py")
            {
                *last = stripped.to_string();
            }

            parts.join(".")
        } else {
            path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        }
    }

    /// Determines the file type based on name and content.
    fn determine_file_type(file_name: &str, parser: &Parser) -> FileType {
        if file_name == "__init__.py" {
            FileType::Package
        } else if file_name.starts_with("test_") || file_name.ends_with("_test.py") {
            FileType::Test
        } else if Self::has_main_block(parser) {
            FileType::Script
        } else {
            FileType::Module
        }
    }

    /// Checks if the file has a `if __name__ == "__main__":` block.
    fn has_main_block(parser: &Parser) -> bool {
        parser
            .query(MAIN_BLOCK_QUERY)
            .map(|r| !r.is_empty())
            .unwrap_or(false)
    }

    /// Extracts import statements from the file.
    fn extract_imports(parser: &Parser) -> Result<Vec<String>> {
        let results = parser.query(IMPORT_QUERY)?;
        Ok(results
            .into_iter()
            .filter_map(|d| d.get("module").cloned())
            .collect())
    }

    /// Extracts function names from the file.
    fn extract_functions(parser: &Parser) -> Result<Vec<String>> {
        let results = parser.query(FUNCTION_DEF_QUERY)?;
        Ok(results
            .into_iter()
            .filter_map(|d| d.get("name").cloned())
            .collect())
    }

    /// Extracts class names from the file.
    fn extract_classes(parser: &Parser) -> Result<Vec<String>> {
        let results = parser.query(CLASS_DEF_QUERY)?;
        Ok(results
            .into_iter()
            .filter_map(|d| d.get("name").cloned())
            .collect())
    }

    /// Builds a concise description of the file.
    fn build_description(
        file_name: &str,
        kind: &FileType,
        functions: &[String],
        classes: &[String],
    ) -> String {
        let mut desc = format!("{} ({})", file_name, kind);

        if !classes.is_empty() {
            desc.push_str(&format!(" - classes: {}", classes.join(", ")));
        }

        if !functions.is_empty() {
            let top_funcs: Vec<_> = functions.iter().take(5).cloned().collect();
            desc.push_str(&format!(" - functions: {}", top_funcs.join(", ")));
            if functions.len() > 5 {
                desc.push_str(&format!(" (+{} more)", functions.len() - 5));
            }
        }

        desc
    }

    /// Checks the file for syntax errors using Python's compile.
    pub async fn check(&self) -> Result<String, PythonFileError> {
        let path_str = self.path.to_string_lossy();
        let spec = self
            .context
            .clone()
            .run_module_command("py_compile", &[path_str.as_ref()])
            .map_err(PythonFileError::Unknown)?;

        let collected = process::run_collect(
            &spec.program,
            &spec.args,
            StdinSource::Null,
            spec.cwd.as_deref(),
            &spec.env,
            Some(Duration::from_secs(30)),
        )
        .await
        .map_err(PythonFileError::Unknown)?;

        if collected.status.success() {
            Ok("Syntax check passed".to_string())
        } else {
            Err(PythonFileError::SyntaxError {
                file_name: self.file_name.clone(),
                message:   String::from_utf8_lossy(&collected.stderr).to_string(),
            })
        }
    }

    /// Runs the Python file with optional input.
    pub async fn run(&self, input: Option<String>) -> Result<String, PythonFileError> {
        self.run_with_timeout(input, config::python_timeout()).await
    }

    /// Runs the Python file with a custom timeout.
    pub async fn run_with_timeout(
        &self,
        input: Option<String>,
        timeout: Duration,
    ) -> Result<String, PythonFileError> {
        let use_module = !self.module_name.is_empty() && self.has_relative_imports();

        let spec = if use_module {
            self.context
                .run_module_command(&self.module_name, &[])
                .map_err(PythonFileError::Unknown)?
        } else {
            self.context
                .run_script_command(&self.path)
                .map_err(PythonFileError::Unknown)?
        };

        let stdin_source = match input {
            Some(ref s) => StdinSource::Bytes(s.clone().into_bytes()),
            None => StdinSource::Null,
        };

        let collected = process::run_collect(
            &spec.program,
            &spec.args,
            stdin_source,
            spec.cwd.as_deref(),
            &spec.env,
            Some(timeout),
        )
        .await
        .map_err(|e| {
            if e.to_string().contains("timeout") || e.to_string().contains("timed out") {
                PythonFileError::Timeout { timeout }
            } else {
                PythonFileError::Unknown(e)
            }
        })?;

        if collected.status.success() {
            Ok(String::from_utf8_lossy(&collected.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&collected.stderr).to_string();
            let diags = Self::extract_line_refs_from_traceback(&stderr, &self.file_name);

            Err(PythonFileError::RuntimeError {
                file_name: self.file_name.clone(),
                stacktrace: stderr,
                diags,
            })
        }
    }

    /// Returns true if the file contains obvious relative imports that require
    /// module execution semantics.
    fn has_relative_imports(&self) -> bool {
        let code = self.parser.code();
        code.contains("from .") || code.contains("from ..")
    }

    /// Runs pytest on this test file.
    pub async fn test(&self) -> Result<String, PythonFileError> {
        if self.kind != FileType::Test {
            return Err(PythonFileError::Unknown(anyhow!(
                "File {} is not a test file",
                self.file_name
            )));
        }

        let path_str = self.path.to_string_lossy();
        let spec = self
            .context
            .clone()
            .with_overlay("pytest")
            .run_module_command("pytest", &["-v", "--tb=short", &path_str])
            .map_err(PythonFileError::Unknown)?;

        let collected = process::run_collect(
            &spec.program,
            &spec.args,
            StdinSource::Null,
            spec.cwd.as_deref(),
            &spec.env,
            Some(Duration::from_secs(120)),
        )
        .await
        .map_err(PythonFileError::Unknown)?;

        let output = String::from_utf8_lossy(&collected.stdout).to_string();
        let stderr = String::from_utf8_lossy(&collected.stderr).to_string();
        let combined = format!("{}\n{}", output, stderr);

        if collected.status.success() {
            Ok(combined)
        } else {
            let diags = Self::extract_line_refs_from_traceback(&combined, &self.file_name);
            Err(PythonFileError::FailedTests {
                file_name: self.file_name.clone(),
                test_results: combined,
                diags,
            })
        }
    }

    /// Extracts line references from a Python traceback.
    fn extract_line_refs_from_traceback(traceback: &str, file_name: &str) -> Vec<LineRef> {
        let mut refs = Vec::new();
        let target_basename = Path::new(file_name)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(file_name);

        for line in traceback.lines() {
            // Match patterns like:
            // File "foo.py", line 42, in function_name
            if let Some(start) = line.find("File \"")
                && let Some(end) = line[start + 6..].find('"')
            {
                let fname = &line[start + 6..start + 6 + end];
                let fname_basename = Path::new(fname)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(fname);

                if fname_basename != target_basename {
                    continue;
                }

                if let Some(line_start) = line.find(", line ")
                    && let Some(comma_pos) = line[line_start + 7..].find(',')
                    && let Ok(line_num) =
                        line[line_start + 7..line_start + 7 + comma_pos].parse::<usize>()
                {
                    refs.push(LineRef {
                        file_name:   fname.to_string(),
                        line_number: line_num,
                    });
                }
            }
        }

        refs
    }

    /// Returns the file's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the file name.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Returns the module name.
    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    /// Returns the simple name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the file type.
    pub fn kind(&self) -> &FileType {
        &self.kind
    }

    /// Returns the imports.
    pub fn imports(&self) -> &[String] {
        &self.imports
    }

    /// Returns the function names.
    pub fn functions(&self) -> &[String] {
        &self.functions
    }

    /// Returns the class names.
    pub fn classes(&self) -> &[String] {
        &self.classes
    }

    /// Returns the description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns whether the file has a main block.
    pub fn has_main(&self) -> bool {
        self.has_main
    }

    /// Returns a reference to the parser.
    pub fn parser(&self) -> &Parser {
        &self.parser
    }

    /// Update the execution context used for this file.
    pub(crate) fn set_context(&mut self, ctx: super::util::UvRunContext) {
        self.context = ctx;
    }

    /// Returns the source code.
    pub fn code(&self) -> &str {
        self.parser.code()
    }

    /// Executes a tree-sitter query on this file.
    pub fn query(&self, q: &str) -> Result<Vec<Dict>> {
        self.parser.query(q)
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Write, path::PathBuf, time::SystemTime};

    use super::*;
    use crate::python::util::UvRunContext;

    fn temp_py_file(contents: &str) -> (PathBuf, PathBuf, ProjectPaths) {
        let nonce = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("umm_rel_import_{nonce}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");

        let path = dir.join("mod.py");
        let mut f = std::fs::File::create(&path).expect("create file");
        f.write_all(contents.as_bytes()).expect("write code");

        let paths = ProjectPaths::new(dir.to_path_buf());
        (dir, path, paths)
    }

    #[test]
    fn detects_relative_imports() {
        let (dir, path, paths) = temp_py_file("from .util import foo\n");
        let ctx = UvRunContext::for_paths(&paths);
        let file = File::new(&path, paths, ctx).expect("build file");
        assert!(file.has_relative_imports());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ignores_absolute_imports() {
        let (dir, path, paths) = temp_py_file("import math\nprint(math.sqrt(4))\n");
        let ctx = UvRunContext::for_paths(&paths);
        let file = File::new(&path, paths, ctx).expect("build file");
        assert!(!file.has_relative_imports());
        let _ = std::fs::remove_dir_all(dir);
    }
}
