use std::{
    ffi::{OsStr, OsString},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use snailquote::unescape;

use super::{parser::Parser, parsers::parser, paths::ProjectPaths, project::Project};
use crate::{
    Dict, config,
    java::{
        grade::{JavacDiagnostic, LineRef},
        queries::{
            CLASS_CONSTRUCTOR_QUERY, CLASS_DECLARATION_QUERY, CLASS_FIELDS_QUERY,
            CLASS_METHOD_QUERY, CLASSNAME_QUERY, IMPORT_QUERY, INTERFACE_CONSTANTS_QUERY,
            INTERFACE_DECLARATION_QUERY, INTERFACE_METHODS_QUERY, INTERFACENAME_QUERY,
            MAIN_METHOD_QUERY, METHOD_CALL_QUERY, PACKAGE_QUERY, TEST_ANNOTATION_QUERY,
        },
        util::{classpath, java_path, javac_path, sourcepath},
    },
    process::{self, StdinSource},
};

/// Normalizes captured snippets by trimming whitespace and flattening newlines.
fn normalize_entry(entry: &str) -> Option<String> {
    let trimmed = entry.replace('\n', " ").trim().to_string();
    if trimmed.is_empty() || trimmed == "[NOT FOUND]" {
        None
    } else {
        Some(trimmed)
    }
}

/// Pushes a simple bullet list wrapped in an XML-like tag onto `lines`.
fn push_block(lines: &mut Vec<String>, tag: &str, items: &[String]) {
    let entries: Vec<String> = items
        .iter()
        .filter_map(|item| {
            let trimmed = item.trim();
            if trimmed.is_empty() || trimmed == "[NOT FOUND]" {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect();

    if entries.is_empty() {
        return;
    }

    lines.push(format!("  <{}>", tag));
    lines.push(String::from("  ```"));
    for entry in entries {
        lines.push(format!("  {}", entry));
    }
    lines.push(String::from("  ```"));
    lines.push(format!("  </{}>", tag));
}

/// Adds a declaration element wrapped in a code fence to protect markup.
fn push_declaration(lines: &mut Vec<String>, decl: &str) {
    let trimmed = decl.trim();
    if trimmed.is_empty() {
        return;
    }
    lines.push(String::from("  <declaration>"));
    lines.push(String::from("  ```"));
    lines.push(format!("  {}", trimmed));
    lines.push(String::from("  ```"));
    lines.push(String::from("  </declaration>"));
}

/// Normalizes stacktrace lines by unescaping common escape sequences.
fn normalize_stacktrace_line(line: &str) -> String {
    line.replace("\\\\", "\\").replace("\\\"", "\"")
}

/// Possible failures while decoding collected process output.
#[derive(thiserror::Error, Debug)]
enum DecodeOutputError {
    /// The captured stderr stream could not be decoded as UTF-8.
    #[error("Error parsing stderr as utf8")]
    Stderr {
        /// The underlying UTF-8 decoding failure.
        #[source]
        source: std::string::FromUtf8Error,
    },
    /// The captured stdout stream could not be decoded as UTF-8.
    #[error("Error parsing stdout as utf8")]
    Stdout {
        /// The underlying UTF-8 decoding failure.
        #[source]
        source: std::string::FromUtf8Error,
    },
    /// The combined output could not be unescaped with `snailquote`.
    #[error("Error when un-escaping {phase} output.")]
    Unescape {
        /// Operation label identifying which phase failed (e.g., `javac`).
        phase:  &'static str,
        /// The upstream error raised by `snailquote::unescape`.
        #[source]
        source: snailquote::UnescapeError,
    },
}

/// Decodes combined stdout/stderr output into a UTF-8 string and removes escape
/// sequences.
fn decode_output(
    stderr: Vec<u8>,
    stdout: Vec<u8>,
    phase: &'static str,
) -> Result<String, DecodeOutputError> {
    let mut decoded =
        String::from_utf8(stderr).map_err(|source| DecodeOutputError::Stderr { source })?;
    let stdout =
        String::from_utf8(stdout).map_err(|source| DecodeOutputError::Stdout { source })?;
    decoded.push_str(&stdout);
    unescape(&decoded).map_err(|source| DecodeOutputError::Unescape { phase, source })
}

/// Loads source code from `path` and constructs a Java parser.
fn parse_source(path: &Path) -> Result<Parser> {
    let source_code = std::fs::read_to_string(path)
        .with_context(|| format!("Could not read file: {:?}", path))?;
    Parser::new(source_code)
}

/// Determines the initial file type and simple name based on parsed
/// declarations.
fn detect_file_identity(
    parser: &Parser,
    path: &Path,
    has_main: bool,
) -> Result<(FileType, String)> {
    let interface = parser.query(INTERFACENAME_QUERY)?;
    if let Some(first) = interface.first() {
        let name = first
            .get("name")
            .ok_or_else(|| {
                anyhow!(
                    "Could not find a valid interface declaration for {} (hashmap has no name key)",
                    path.display()
                )
            })?
            .to_string();
        return Ok((FileType::Interface, name));
    }

    let classes = parser.query(CLASSNAME_QUERY)?;
    if let Some(first) = classes.first() {
        let name = first
            .get("name")
            .ok_or_else(|| {
                anyhow!(
                    "Could not find a valid class declaration for {} (hashmap has no name key)",
                    path.display()
                )
            })?
            .to_string();
        let kind = if has_main {
            FileType::ClassWithMain
        } else {
            FileType::Class
        };
        return Ok((kind, name));
    }

    Ok((FileType::Class, String::new()))
}

/// Collects fully qualified test method names discovered via `@Test`
/// annotations.
fn collect_test_methods(parser: &Parser, proper_name: &str) -> Result<Vec<String>> {
    let mut tests = Vec::new();
    for entry in parser.query(TEST_ANNOTATION_QUERY)? {
        if let Some(method) = entry.get("name") {
            tests.push(format!("{}#{}", proper_name, method));
        }
    }
    Ok(tests)
}

/// Returns the XML type attribute corresponding to the file classification.
fn file_type_attr(kind: &FileType) -> &'static str {
    match kind {
        FileType::Interface => "interface",
        FileType::Class => "class",
        FileType::ClassWithMain => "class_with_main",
        FileType::Test => "test",
    }
}

/// Renders declaration and summary sections for interface files.
fn interface_sections(parser: &Parser, proper_name: &str) -> Vec<String> {
    let empty_dict = Dict::new();
    let empty = String::new();
    let mut lines = Vec::new();

    let declaration_data = parser
        .query(INTERFACE_DECLARATION_QUERY)
        .unwrap_or_default();
    let declaration = declaration_data.first().unwrap_or(&empty_dict);

    let parameters = declaration.get("parameters").unwrap_or(&empty).trim();
    let extends = declaration.get("extends").unwrap_or(&empty).trim();
    let mut decl = format!("interface {proper_name}");
    if !parameters.is_empty() {
        decl.push(' ');
        decl.push_str(parameters);
    }
    if !extends.is_empty() {
        decl.push(' ');
        decl.push_str(extends);
    }
    push_declaration(&mut lines, decl.trim());

    let constants = parser
        .query(INTERFACE_CONSTANTS_QUERY)
        .unwrap_or_default()
        .iter()
        .filter_map(|c| c.get("constant"))
        .filter_map(|s| normalize_entry(s))
        .collect::<Vec<_>>();

    let methods = parser
        .query(INTERFACE_METHODS_QUERY)
        .unwrap_or_default()
        .iter()
        .filter_map(|m| m.get("signature"))
        .filter_map(|s| normalize_entry(s))
        .collect::<Vec<_>>();

    push_block(&mut lines, "constants", &constants);
    push_block(&mut lines, "methods", &methods);
    lines
}

/// Renders declaration and summary sections for class-like files.
fn class_sections(parser: &Parser, proper_name: &str) -> Vec<String> {
    let empty_dict = Dict::new();
    let empty = String::new();
    let mut lines = Vec::new();

    let declaration_data = parser.query(CLASS_DECLARATION_QUERY).unwrap_or_default();
    let declaration = declaration_data.first().unwrap_or(&empty_dict);

    let parameters = declaration.get("typeParameters").unwrap_or(&empty).trim();
    let implements = declaration.get("interfaces").unwrap_or(&empty).trim();

    let mut decl = format!("class {proper_name}");
    if !parameters.is_empty() {
        decl.push(' ');
        decl.push_str(parameters);
    }
    if !implements.is_empty() {
        decl.push(' ');
        decl.push_str(implements);
    }
    push_declaration(&mut lines, decl.trim());

    let fields = parser
        .query(CLASS_FIELDS_QUERY)
        .unwrap_or_default()
        .iter()
        .filter_map(|f| f.get("field"))
        .filter_map(|s| normalize_entry(s))
        .collect::<Vec<_>>();

    let constructor_data = parser.query(CLASS_CONSTRUCTOR_QUERY).unwrap_or_default();
    let constructors = constructor_data
        .iter()
        .map(constructor_signature)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    let method_data = parser.query(CLASS_METHOD_QUERY).unwrap_or_default();
    let methods = method_data
        .iter()
        .map(method_signature)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    push_block(&mut lines, "fields", &fields);
    push_block(&mut lines, "constructors", &constructors);
    push_block(&mut lines, "methods", &methods);
    lines
}

/// Builds the XML-like description block used for retrieval context.
fn build_description(
    parser: &Parser,
    proper_name: &str,
    kind: FileType,
    test_methods: &[String],
    file_path: &str,
) -> String {
    let mut lines = vec![format!(
        "<file name=\"{proper_name}\" path=\"{file_path}\" type=\"{}\">",
        file_type_attr(&kind)
    )];

    match kind {
        FileType::Interface => lines.extend(interface_sections(parser, proper_name)),
        _ => lines.extend(class_sections(parser, proper_name)),
    }

    if !test_methods.is_empty() {
        push_block(&mut lines, "tests", test_methods);
    }

    lines.push(String::from("</file>"));
    lines.join("\n")
}

/// Builds a readable method signature from tree-sitter captures.
fn method_signature(data: &Dict) -> String {
    if data.get("identifier").is_none() {
        return String::new();
    }

    let mut parts = Vec::new();

    if let Some(annotation) = data.get("annotation").and_then(|s| normalize_entry(s)) {
        parts.push(annotation);
    }

    if let Some(modifier) = data.get("modifier").and_then(|s| normalize_entry(s)) {
        parts.push(modifier);
    }

    if let Some(return_type) = data.get("returnType").and_then(|s| normalize_entry(s)) {
        parts.push(return_type);
    }

    if let Some(identifier) = data.get("identifier") {
        let params = data
            .get("parameters")
            .map(|p| p.trim().to_string())
            .unwrap_or_else(|| "()".to_string());
        parts.push(format!("{}{}", identifier.trim(), params));
    }

    if let Some(throws) = data.get("throws").and_then(|s| normalize_entry(s)) {
        parts.push(throws);
    }

    parts
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Builds a readable constructor signature from tree-sitter captures.
fn constructor_signature(data: &Dict) -> String {
    if data.get("identifier").is_none() {
        return String::new();
    }

    let mut parts = Vec::new();

    if let Some(annotation) = data.get("annotation").and_then(|s| normalize_entry(s)) {
        parts.push(annotation);
    }

    if let Some(modifier) = data.get("modifier").and_then(|s| normalize_entry(s)) {
        parts.push(modifier);
    }

    if let Some(identifier) = data.get("identifier") {
        let params = data
            .get("parameters")
            .map(|p| p.trim().to_string())
            .unwrap_or_else(|| "()".to_string());
        parts.push(format!("{}{}", identifier.trim(), params));
    }

    if let Some(throws) = data.get("throws").and_then(|s| normalize_entry(s)) {
        parts.push(throws);
    }

    parts
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
/// Types of Java files -
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileType {
    /// - Interface
    Interface,
    /// - Class
    Class,
    /// - Class with a main method
    ClassWithMain,
    /// - JUnit test class
    Test,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Struct representing a java file
///
/// Captures the filesystem name (`file_name`), the simple Java identifier
/// (`name`), and the package-qualified form (`proper_name`) so callers can
/// choose whichever is most convenient.
pub struct File {
    /// path to java file.
    path:         PathBuf,
    /// Filesystem name (including the `.java` extension).
    file_name:    String,
    /// package the java file belongs to.
    package_name: Option<String>,
    /// imports made by the java file.
    imports:      Option<Vec<Dict>>,
    /// Simple, unqualified Java identifier extracted from the declaration.
    name:         String,
    /// Package-qualified Java name (no ANSI colors, just dotted notation).
    proper_name:  String,
    /// Fully qualified `Class#method` strings discovered via `@Test`
    /// annotations.
    test_methods: Vec<String>,
    /// Classification of the Java file (class, interface, test, etc.).
    kind:         FileType,
    #[serde(skip)]
    /// The parser for this file
    parser:       Parser,
    /// Concise description of the file
    description:  String,
    /// Workspace paths associated with this file
    paths:        ProjectPaths,
}

/// Two `File`s are equal if their paths are equal
impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

/// Based on PartialEq
impl Eq for File {}

/// Hash based on path
impl Hash for File {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

/// An enum to represent possible errors with a Java file
#[derive(thiserror::Error, Debug)]
pub enum JavaFileError {
    /// An error while compiling a Java file (running
    /// [fn@crate::java::File::check])
    #[error("Something went wrong while compiling the Java file")]
    DuringCompilation {
        /// javac stacktrace
        stacktrace: String,
        /// javac stacktrace, parsed with
        /// [fn@crate::java::parsers::parser::parse_diag]
        diags:      Vec<JavacDiagnostic>,
    },
    /// An error while running a Java file (running
    /// [fn@crate::java::File::run])
    #[error("Something went wrong while running the Java file")]
    AtRuntime {
        /// java output
        output: String,
        /// java stacktrace, parsed with [parser::junit_stacktrace_line_ref]
        diags:  Vec<LineRef>,
    },
    /// An error while testing a Java file (running
    /// [fn@crate::java::File::test])
    #[error("Something went wrong while testing the Java file")]
    FailedTests {
        /// junit test results
        test_results: String,
        /// junit stacktrace, parsed with [parser::junit_stacktrace_line_ref]
        diags:        Vec<LineRef>,
    },
    /// Unknown error
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

impl From<DecodeOutputError> for JavaFileError {
    fn from(err: DecodeOutputError) -> Self {
        JavaFileError::Unknown(err.into())
    }
}

impl File {
    /// Builds the standard set of `javac` arguments for this file.
    fn javac_args(&self, include_doclint: bool, prefer_source: bool) -> Result<Vec<OsString>> {
        let mut args = vec![
            OsString::from("--source-path"),
            OsString::from(sourcepath(&self.paths)?),
            OsString::from("-g"),
            OsString::from("--class-path"),
            OsString::from(classpath(&self.paths)?),
            OsString::from("-d"),
            OsString::from(self.paths.build_dir().to_str().unwrap_or(".").to_string()),
            self.path.as_os_str().to_os_string(),
            OsString::from("-Xdiags:verbose"),
        ];
        if include_doclint {
            args.push(OsString::from("-Xdoclint"));
        }
        if prefer_source {
            args.push(OsString::from("-Xprefer:source"));
        }
        Ok(args)
    }

    /// Constructs the `java` invocation for running a main class.
    fn java_run_args(&self) -> Result<Vec<OsString>> {
        Ok(vec![
            OsString::from("--class-path"),
            OsString::from(classpath(&self.paths)?),
            OsString::from(self.proper_name.clone()),
        ])
    }

    /// Constructs the `java` invocation for the JUnit console launcher.
    fn junit_args(&self, selectors: &[String]) -> Result<Vec<OsString>> {
        let mut args = vec![
            OsString::from("-cp"),
            OsString::from(classpath(&self.paths)?),
            OsString::from("org.junit.platform.console.ConsoleLauncher"),
            OsString::from("--disable-banner"),
            OsString::from("--disable-ansi-colors"),
            OsString::from("--details-theme=unicode"),
            OsString::from("--single-color"),
        ];

        if selectors.is_empty() {
            args.push(OsString::from("--scan-class-path"));
        } else {
            for selector in selectors {
                args.push(OsString::from(selector));
            }
        }

        Ok(args)
    }

    /// Spawns the given command, wiring stdin/stdout/stderr, and returns the
    /// collected output once the process completes.
    async fn collect_process(
        program: &OsStr,
        args: &[OsString],
        stdin: StdinSource,
        timeout: Duration,
    ) -> Result<process::Collected, JavaFileError> {
        process::run_collect(program, args, stdin, None, &[], Some(timeout))
            .await
            .map_err(JavaFileError::Unknown)
    }

    /// Shared helper to compile and run a main class with the provided stdin
    /// configuration.
    async fn exec_main(
        &self,
        stdin_mode: StdinSource,
        output_phase: &'static str,
    ) -> Result<String, JavaFileError> {
        if self.kind != FileType::ClassWithMain {
            return Err(JavaFileError::DuringCompilation {
                stacktrace: "The file you wish to run does not have a main method.".into(),
                diags:      vec![],
            });
        }

        self.check().await?;

        let java = java_path().map_err(JavaFileError::Unknown)?;
        let args = self.java_run_args().map_err(JavaFileError::Unknown)?;

        let process::Collected {
            status,
            stdout,
            stderr,
        } = Self::collect_process(java.as_os_str(), &args, stdin_mode, config::java_timeout())
            .await?;

        let output = decode_output(stderr, stdout, output_phase)?;

        if status.success() {
            Ok(output)
        } else {
            let mut diags = Vec::new();
            for line in output.lines() {
                if let Ok(diag) = parser::junit_stacktrace_line_ref(line) {
                    diags.push(diag);
                }
            }

            Err(JavaFileError::AtRuntime { output, diags })
        }
    }

    /// Creates a new `File` from `path`
    ///
    /// * `path`: the path to read and try to create a File instance for.
    pub(super) fn new(path: PathBuf, paths: ProjectPaths) -> Result<Self> {
        let parser = parse_source(path.as_path())?;

        let imports = {
            let imports = parser.query(IMPORT_QUERY)?;
            if imports.is_empty() {
                None
            } else {
                Some(imports)
            }
        };

        let package_name = {
            let package_name = parser.query(PACKAGE_QUERY)?;

            if package_name.is_empty() {
                None
            } else {
                package_name[0].get("name").map(String::to_owned)
            }
        };

        let has_main = !parser.query(MAIN_METHOD_QUERY)?.is_empty();
        let (kind, name) = detect_file_identity(&parser, path.as_path(), has_main)?;

        let proper_name = if let Some(pkg) = package_name.as_ref() {
            format!("{pkg}.{name}")
        } else {
            name.clone()
        };

        let test_methods = collect_test_methods(&parser, &proper_name)?;
        let kind = if !test_methods.is_empty() {
            FileType::Test
        } else {
            kind
        };

        let file_path = path.display().to_string();
        let description =
            build_description(&parser, &proper_name, kind.clone(), &test_methods, &file_path);

        Ok(Self {
            path: path.to_owned(),
            file_name: path.file_name().unwrap().to_str().unwrap().to_string(),
            package_name,
            imports,
            name,
            test_methods,
            kind,
            proper_name,
            parser,
            description,
            paths,
        })
    }

    /// Utility method to ask javac for documentation lints using the -Xdoclint
    /// flag.
    pub async fn doc_check(&self) -> Result<String, JavaFileError> {
        let javac = javac_path().map_err(JavaFileError::Unknown)?;
        let args = self
            .javac_args(true, false)
            .map_err(JavaFileError::Unknown)?;

        let collected = Self::collect_process(
            javac.as_os_str(),
            &args,
            StdinSource::Null,
            config::javac_timeout(),
        )
        .await?;

        let process::Collected { stdout, stderr, .. } = collected;
        let output = decode_output(stderr, stdout, "javac")?;

        Ok(output)
    }

    /// Utility method to check for syntax errors using javac.
    pub async fn check(&self) -> Result<String, JavaFileError> {
        let javac = javac_path().map_err(JavaFileError::Unknown)?;
        let args = self
            .javac_args(false, true)
            .map_err(JavaFileError::Unknown)?;

        let collected = Self::collect_process(
            javac.as_os_str(),
            &args,
            StdinSource::Null,
            config::javac_timeout(),
        )
        .await?;

        let process::Collected {
            status,
            stdout,
            stderr,
        } = collected;
        let output = decode_output(stderr, stdout, "javac")?;

        if status.success() {
            Ok(output)
        } else {
            let mut diags = Vec::new();
            for line in output.lines() {
                if let Ok(diag) = parser::parse_diag(line) {
                    diags.push(diag);
                }
            }

            Err(JavaFileError::DuringCompilation {
                stacktrace: output,
                diags,
            })
        }
    }

    /// Utility method to run a java file that has a main method.
    pub async fn run(&self, input: Option<String>) -> Result<String, JavaFileError> {
        let stdin_mode = match input {
            Some(mut value) => {
                value.push_str("\r\n");
                StdinSource::Bytes(value.into_bytes())
            }
            None => StdinSource::Inherit,
        };

        self.exec_main(stdin_mode, "java").await
    }

    /// Runs the java file while piping stdin even when no explicit input is
    /// supplied.
    pub async fn run_with_input(&self, input: Option<String>) -> Result<String, JavaFileError> {
        let stdin_mode = match input {
            Some(mut value) => {
                value.push_str("\r\n");
                StdinSource::Bytes(value.into_bytes())
            }
            None => StdinSource::Bytes(Vec::new()),
        };

        self.exec_main(stdin_mode, "java").await
    }

    /// A utility method that takes a list of strings (or types that implement
    /// `Into<String>`) meant to represent test method names, and runs those
    /// tests.
    ///
    /// Returns the output from JUnit as a string. There are parsers in
    /// ['parsers module'][crate::java::parsers::parser] that helps parse this
    /// output.
    ///
    /// * `tests`: list of strings (or types that implement `Into<String>`)
    ///   meant to represent test method names,
    pub async fn test(
        &self,
        tests: Vec<&str>,
        project: Option<&Project>,
    ) -> Result<String, JavaFileError> {
        self.check().await?;

        let java = java_path().map_err(JavaFileError::Unknown)?;

        let explicit_tests = {
            let mut mapped = Vec::<String>::new();
            for t in tests {
                mapped.push(format!("{}#{}", self.proper_name.clone(), t));
            }

            if mapped.is_empty() {
                self.test_methods.clone()
            } else {
                mapped
            }
        };

        let selectors: Vec<String> = explicit_tests
            .iter()
            .map(|s| format!("--select-method={s}"))
            .collect();

        let args = self
            .junit_args(&selectors)
            .map_err(JavaFileError::Unknown)?;

        let collected = Self::collect_process(
            java.as_os_str(),
            &args,
            StdinSource::Inherit,
            config::java_timeout(),
        )
        .await?;

        let process::Collected {
            status,
            stdout,
            stderr,
        } = collected;
        let output = decode_output(stderr, stdout, "JUnit")?;

        if status.success() {
            Ok(output)
        } else {
            let mut diags = Vec::new();
            let mut new_output = Vec::new();

            for line in output.lines() {
                if line.contains("MethodSource") || line.contains("Native Method") {
                    continue;
                }

                if let Ok(diag) = parser::junit_stacktrace_line_ref(line) {
                    if let Some(proj) = project
                        && proj.identify(diag.file_name()).is_ok()
                    {
                        new_output.push(normalize_stacktrace_line(line));
                    }
                    diags.push(diag);
                } else if let Ok(diag) = parser::parse_diag(line) {
                    if let Some(proj) = project
                        && proj.identify(diag.file_name()).is_ok()
                    {
                        new_output.push(normalize_stacktrace_line(line));
                    }
                    diags.push(diag.into());
                } else {
                    new_output.push(normalize_stacktrace_line(line));
                }
            }

            Err(JavaFileError::FailedTests {
                test_results: new_output.join("\n"),
                diags,
            })
        }
    }

    /// A utility method that takes a list of strings (or types that implement
    /// `Into<String>`) meant to represent test method names, and runs those
    /// tests.
    ///
    /// Returns the output from JUnit as a string. There are parsers in
    /// ['parsers module'][crate::java::parsers::parser] that helps parse this
    /// output. Get a reference to the file's kind.
    pub fn kind(&self) -> &FileType {
        &self.kind
    }

    /// Get a reference to the file's file name.
    pub fn file_name(&self) -> &str {
        self.file_name.as_ref()
    }

    /// Get a reference to the file's test methods.
    pub fn test_methods(&self) -> Vec<String> {
        self.test_methods.clone()
    }

    /// treesitter query for this file
    pub fn query(&self, q: &str) -> Result<Vec<Dict>> {
        self.parser.query(q)
    }

    /// Get a reference to the file's path.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get a reference to the file's proper name.
    pub fn package_name(&self) -> Option<&String> {
        self.package_name.as_ref()
    }

    /// Borrow the underlying parser.
    pub fn parser(&self) -> &Parser {
        &self.parser
    }

    /// Returns method invocation names with their 1-based starting line
    /// numbers.
    pub fn method_invocations(&self) -> Result<Vec<(String, usize)>> {
        self.parser
            .query_capture_positions(METHOD_CALL_QUERY, "name")
    }

    /// Returns full method bodies matching the provided name with their
    /// starting line numbers.
    pub fn method_bodies_named(&self, method_name: &str) -> Result<Vec<(String, usize)>> {
        let query = format!(include_str!("queries/method_body_with_name.scm"), method_name);
        self.parser.query_capture_positions(&query, "body")
    }

    /// Returns the source code associated with this file.
    pub fn code(&self) -> &str {
        self.parser.code()
    }

    /// Get a reference to the file's description.
    pub fn description(&self) -> String {
        self.description.clone()
    }

    /// Get the file's proper name.
    pub fn proper_name(&self) -> String {
        self.proper_name.clone()
    }

    /// Returns the simple, unqualified name of the file.
    pub fn simple_name(&self) -> &str {
        &self.name
    }

    /// Returns the parsed imports for this file, if any.
    pub fn imports(&self) -> Option<&[Dict]> {
        self.imports.as_deref()
    }
}
