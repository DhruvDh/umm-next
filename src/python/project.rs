#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Python project discovery and management.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::{
    file::{File, FileType},
    paths::ProjectPaths,
    util::{discover_data_files, discover_python_files, discover_test_files},
};

/// Represents a Python project with discovered files and metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Project {
    /// Collection of Python files in this project.
    files:      Vec<File>,
    /// Cached list of names kept in lockstep with `files` for quick lookups.
    names:      Vec<String>,
    /// Workspace paths associated with this project.
    paths:      ProjectPaths,
    /// Data files discovered in the project (.txt, .csv, .json).
    data_files: HashMap<String, PathBuf>,
}

impl Project {
    /// Creates a new project by discovering files in the current directory.
    pub fn new() -> Result<Self> {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        Self::from_root(cwd)
    }

    /// Creates a new project from a root directory.
    pub fn from_root(root: impl Into<PathBuf>) -> Result<Self> {
        let paths = ProjectPaths::new(root.into());
        Self::from_paths(paths)
    }

    /// Creates a new project from explicit paths.
    pub fn from_paths(paths: ProjectPaths) -> Result<Self> {
        let mut files = Vec::new();
        let mut names = Vec::new();

        // Discover Python files
        let py_files = discover_python_files(&paths)?;

        for path in py_files {
            let display_path = path.display().to_string();
            match File::new(&path, paths.clone()) {
                Ok(file) => {
                    names.push(file.name().to_string());
                    files.push(file);
                }
                Err(e) => {
                    tracing::warn!("Skipping file {} due to error: {}", display_path, e);
                }
            }
        }

        // Also discover test files if in a separate directory
        if paths.test_dir().exists() && paths.test_dir() != paths.source_dir() {
            let test_files = discover_test_files(&paths)?;
            for path in test_files {
                // Skip if already discovered
                if files.iter().any(|f| f.path() == path) {
                    continue;
                }

                let display_path = path.display().to_string();
                match File::new(&path, paths.clone()) {
                    Ok(file) => {
                        names.push(file.name().to_string());
                        files.push(file);
                    }
                    Err(e) => {
                        tracing::warn!("Skipping test file {} due to error: {}", display_path, e);
                    }
                }
            }
        }

        // Discover data files
        let data_file_paths = discover_data_files(&paths)?;
        let data_files: HashMap<String, PathBuf> = data_file_paths
            .into_iter()
            .filter_map(|p| {
                p.file_name()
                    .map(|n| (n.to_string_lossy().to_string(), p.clone()))
            })
            .collect();

        Ok(Self {
            files,
            names,
            paths,
            data_files,
        })
    }

    /// Identifies a file by name.
    ///
    /// The name can be:
    /// - Simple name without extension: `hello`
    /// - File name with extension: `hello.py`
    /// - Module name: `package.hello`
    /// - Path-prefixed filename from tracebacks: `/tmp/project/hello.py`
    pub fn identify(&self, name: &str) -> Result<File> {
        let mut candidates: Vec<String> = Vec::new();

        let mut push_candidate = |s: &str| {
            if !s.is_empty() {
                candidates.push(s.to_string());
            }
        };

        // Raw input and stripped `.py`
        push_candidate(name);
        let stripped = name.strip_suffix(".py").unwrap_or(name);
        push_candidate(stripped);

        // Path-aware candidates (handles absolute/relative traceback paths)
        let path = Path::new(name);
        if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
            push_candidate(file_name);
            let base = file_name.strip_suffix(".py").unwrap_or(file_name);
            push_candidate(base);
        }

        // Try path relative to the project root, if applicable
        if let Ok(rel) = path.strip_prefix(self.paths.root_dir()) {
            if let Some(rel_str) = rel.to_str() {
                push_candidate(rel_str);
            }
            if let Some(rel_file) = rel.file_name().and_then(|s| s.to_str()) {
                push_candidate(rel_file);
                let base = rel_file.strip_suffix(".py").unwrap_or(rel_file);
                push_candidate(base);
            }
        }

        // Deduplicate to avoid repeated comparisons
        candidates.sort();
        candidates.dedup();

        // Try exact matches against stored simple names
        for cand in &candidates {
            if let Some(idx) = self.names.iter().position(|n| n == cand) {
                return Ok(self.files[idx].clone());
            }
        }

        // Try matching file names and module names
        for file in &self.files {
            if candidates
                .iter()
                .any(|cand| cand == file.file_name() || cand == file.module_name())
            {
                return Ok(file.clone());
            }

            // Also try matching the last part of the module name
            if let Some(last) = file.module_name().rsplit('.').next()
                && candidates.iter().any(|cand| cand == last)
            {
                return Ok(file.clone());
            }
        }

        bail!("Could not find Python file '{}'. Available files: {:?}", name, self.names)
    }

    /// Returns the number of files in the project.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Returns an iterator over all files.
    pub fn files(&self) -> impl Iterator<Item = &File> {
        self.files.iter()
    }

    /// Returns all source files (non-test, non-package).
    pub fn source_files(&self) -> Vec<&File> {
        self.files
            .iter()
            .filter(|f| matches!(f.kind(), FileType::Script | FileType::Module))
            .collect()
    }

    /// Returns all test files.
    pub fn test_files(&self) -> Vec<&File> {
        self.files
            .iter()
            .filter(|f| matches!(f.kind(), FileType::Test))
            .collect()
    }

    /// Returns the data files in the project.
    pub fn data_files(&self) -> &HashMap<String, PathBuf> {
        &self.data_files
    }

    /// Returns the project paths.
    pub fn paths(&self) -> &ProjectPaths {
        &self.paths
    }

    /// Returns a JSON description of the project.
    pub fn describe(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize project description")
    }

    /// Prints project info to stdout.
    pub fn info(&self) {
        eprintln!("Python Project");
        eprintln!("==============");
        eprintln!("Root: {:?}", self.paths.root_dir());
        eprintln!("Files: {}", self.files.len());
        eprintln!();

        for file in &self.files {
            eprintln!("  {} ({})", file.name(), file.kind());
            if !file.functions().is_empty() {
                eprintln!("    Functions: {}", file.functions().join(", "));
            }
            if !file.classes().is_empty() {
                eprintln!("    Classes: {}", file.classes().join(", "));
            }
        }

        if !self.data_files.is_empty() {
            eprintln!();
            eprintln!("Data files:");
            for name in self.data_files.keys() {
                eprintln!("  {}", name);
            }
        }
    }
}
