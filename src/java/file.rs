use std::{
    hash::{Hash, Hasher},
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use snailquote::unescape;

use super::{parser::Parser, paths::ProjectPaths, project::Project};
use crate::{
    Dict,
    constants::{
        CLASS_CONSTRUCTOR_QUERY, CLASS_DECLARATION_QUERY, CLASS_FIELDS_QUERY, CLASS_METHOD_QUERY,
        CLASSNAME_QUERY, IMPORT_QUERY, INTERFACE_CONSTANTS_QUERY, INTERFACE_DECLARATION_QUERY,
        INTERFACE_METHODS_QUERY, INTERFACENAME_QUERY, JUNIT_PLATFORM, MAIN_METHOD_QUERY,
        PACKAGE_QUERY, TEST_ANNOTATION_QUERY,
    },
    java::grade::{JavacDiagnostic, LineRef},
    parsers::parser,
    util::{classpath, java_path, javac_path, sourcepath},
};
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
pub struct File {
    /// path to java file.
    path:         PathBuf,
    /// name of file.
    file_name:    String,
    /// package the java file belongs to.
    package_name: Option<String>,
    /// imports made by the java file.
    imports:      Option<Vec<Dict>>,
    /// name of the file TODO: How does this differ from `file_name`?
    name:         String,
    /// colored terminal string representing java file name.
    proper_name:  String,
    /// Name of tests methods in this file, as understood by JUnit.
    test_methods: Vec<String>,
    /// Name of tests methods in this file, colored using terminal color codes.
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
        /// [fn@crate::parsers::parser::parse_diag]
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

impl File {
    /// Creates a new `File` from `path`
    ///
    /// * `path`: the path to read and try to create a File instance for.
    pub(super) fn new(path: PathBuf, paths: ProjectPaths) -> Result<Self> {
        let parser = {
            let source_code = std::fs::read_to_string(&path)
                .with_context(|| format!("Could not read file: {:?}", &path))?;
            Parser::new(source_code)?
        };

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
        let (kind, name) = {
            let interface = parser.query(INTERFACENAME_QUERY)?;
            if let Some(first) = interface.first() {
                let name = first
                    .get("name")
                    .ok_or_else(|| {
                        anyhow!(
                            "Could not find a valid interface declaration for {} (hashmap has no \
                             name key)",
                            path.display()
                        )
                    })?
                    .to_string();
                (FileType::Interface, name)
            } else {
                let classes = parser.query(CLASSNAME_QUERY)?;
                if let Some(first) = classes.first() {
                    let name = first
                        .get("name")
                        .ok_or_else(|| {
                            anyhow!(
                                "Could not find a valid class declaration for {} (hashmap has no \
                                 name key)",
                                path.display()
                            )
                        })?
                        .to_string();
                    let kind = if has_main {
                        FileType::ClassWithMain
                    } else {
                        FileType::Class
                    };
                    (kind, name)
                } else {
                    (FileType::Class, String::new())
                }
            }
        };

        let proper_name = if let Some(pkg) = package_name.as_ref() {
            format!("{pkg}.{name}")
        } else {
            name.clone()
        };

        let test_methods = {
            let test_methods = parser.query(TEST_ANNOTATION_QUERY)?;
            let mut tests = vec![];
            for t in test_methods {
                if let Some(t) = t.get("name") {
                    tests.push(format!("{}#{}", &proper_name, t));
                }
            }

            tests
        };

        let kind = if !test_methods.is_empty() {
            FileType::Test
        } else {
            kind
        };

        let description = match kind {
            FileType::Interface => {
                let empty_dict = Dict::new();
                let empty = String::new();
                let not_found = String::from("[NOT FOUND]");

                let query_result = parser
                    .query(INTERFACE_DECLARATION_QUERY)
                    .unwrap_or_default();
                let declaration = query_result.first().unwrap_or(&empty_dict);

                let parameters = declaration.get("parameters").unwrap_or(&empty).trim();
                let extends = declaration.get("extends").unwrap_or(&empty).trim();

                let consts = parser
                    .query(INTERFACE_CONSTANTS_QUERY)
                    .unwrap_or_default()
                    .iter()
                    .map(|c| c.get("constant").unwrap_or(&not_found).to_string())
                    .collect::<Vec<String>>()
                    .join("\n");

                let methods = parser
                    .query(INTERFACE_METHODS_QUERY)
                    .unwrap_or_default()
                    .iter()
                    .map(|m| m.get("signature").unwrap_or(&not_found).to_string())
                    .collect::<Vec<String>>()
                    .join("\n");

                let methods = if methods.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    methods.trim().to_string()
                };

                let consts = if consts.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    consts.trim().to_string()
                };

                let mut result = vec![];
                result.push(format!("Interface: {proper_name} {parameters} {extends}:\n"));

                if !consts.contains("NOT FOUND") {
                    result.push(String::from("Constants:"));
                    result.push(consts);
                }
                if !methods.contains("NOT FOUND") {
                    result.push(String::from("Methods:"));
                    result.push(methods);
                }

                format!("```\n{r}\n```", r = result.join("\n"))
            }
            _ => {
                let empty_dict = Dict::new();
                let empty = String::new();
                let not_found = String::from("[NOT FOUND]");

                let query_result = parser.query(CLASS_DECLARATION_QUERY).unwrap_or_default();
                let declaration = query_result.first().unwrap_or(&empty_dict);

                let parameters = declaration.get("typeParameters").unwrap_or(&empty).trim();
                let implements = declaration.get("interfaces").unwrap_or(&empty).trim();

                let fields = parser
                    .query(CLASS_FIELDS_QUERY)
                    .unwrap_or_default()
                    .iter()
                    .map(|f| f.get("field").unwrap_or(&not_found).trim().to_string())
                    .collect::<Vec<String>>()
                    .join(", ");

                let methods = parser
                    .query(CLASS_METHOD_QUERY)
                    .unwrap_or_default()
                    .iter()
                    .map(|m| {
                        let identifier = m.get("identifier").unwrap_or(&not_found).trim();
                        let parameters = m.get("parameters").unwrap_or(&empty);

                        if identifier == not_found.as_str() {
                            "[NOT FOUND]".to_string()
                        } else {
                            format!("{}{}", identifier.trim(), parameters.trim())
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(", ");

                let constructors = parser
                    .query(CLASS_CONSTRUCTOR_QUERY)
                    .unwrap_or_default()
                    .iter()
                    .map(|m| {
                        let identifier = m.get("identifier").unwrap_or(&not_found).trim();
                        let parameters = m.get("parameters").unwrap_or(&empty);

                        if identifier == not_found.as_str() {
                            "[NOT FOUND]".to_string()
                        } else {
                            format!("{}{}", identifier.trim(), parameters.trim())
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(", ");

                let fields = if fields.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    format!("\tFields: {}", fields)
                };

                let methods = if methods.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    format!("\tMethods: {}", methods)
                };

                let constructors = if constructors.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    format!("\tConstructors: {}", constructors)
                };

                let mut result = vec![];
                result.push(format!("Class: {proper_name} {parameters} {implements}:\n"));

                if !fields.contains("NOT FOUND") {
                    result.push(fields);
                }
                if !constructors.contains("NOT FOUND") {
                    result.push(constructors);
                }
                if !methods.contains("NOT FOUND") {
                    result.push(methods);
                }

                result.join("\n")
            }
        };

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

    /// Returns the inner doc check of this [`File`].
    fn inner_doc_check(&self, err: Stdio, out: Stdio, in_: Stdio) -> Result<Output> {
        let source_path = sourcepath(&self.paths)?;
        let class_path = classpath(&self.paths)?;
        let build_dir = self.paths.build_dir().to_str().unwrap_or(".").to_string();

        Command::new(javac_path()?)
            .stderr(err)
            .stdout(out)
            .stdin(in_)
            .arg("--source-path")
            .arg(source_path.as_str())
            .arg("-g")
            .arg("--class-path")
            .arg(class_path.as_str())
            .arg("-d")
            .arg(build_dir)
            .arg(self.path.as_path())
            .arg("-Xdiags:verbose")
            .arg("-Xdoclint")
            .output()
            .context("Failed to spawn javac process.")
    }

    /// Utility method to ask javac for documentation lints using the -Xdoclint
    /// flag.
    ///
    /// The method simply returns the output produced by javac as a String.
    /// There is a ['parse_diag method'][fn@crate::parsers::parser::parse_diag]
    /// that can parse these to yield useful information.
    pub fn doc_check(&self) -> Result<String, JavaFileError> {
        let child = self.inner_doc_check(Stdio::piped(), Stdio::piped(), Stdio::piped())?;

        let output = unescape(
            &[
                String::from_utf8(child.stderr).context("Error when parsing stderr as utf8")?,
                String::from_utf8(child.stdout).context("Error when parsing stdout as utf8")?,
            ]
            .concat(),
        )
        .context("Error when un-escaping javac output.")?;

        Ok(output)
    }

    /// Returns the inner check of this [`File`].
    fn inner_check(&self, err: Stdio, out: Stdio, in_: Stdio) -> Result<Output> {
        let source_path = sourcepath(&self.paths)?;
        let class_path = classpath(&self.paths)?;
        let build_dir = self.paths.build_dir().to_str().unwrap_or(".").to_string();

        Command::new(javac_path()?)
            .stderr(err)
            .stdout(out)
            .stdin(in_)
            .arg("--source-path")
            .arg(source_path.as_str())
            .arg("-g")
            .arg("--class-path")
            .arg(class_path.as_str())
            .arg("-d")
            .arg(build_dir)
            .arg(self.path.as_path())
            .arg("-Xdiags:verbose")
            .arg("-Xprefer:source")
            .output()
            .context("Failed to spawn javac process.")
    }

    /// Utility method to check for syntax errors using javac flag.
    pub fn check(&self) -> Result<String, JavaFileError> {
        match self.inner_check(Stdio::piped(), Stdio::piped(), Stdio::piped()) {
            Ok(out) => {
                let output = unescape(
                    &[
                        String::from_utf8(out.stderr).context("Error parsing stderr as utf8")?,
                        String::from_utf8(out.stdout).context("Error parsing stdout as utf8")?,
                    ]
                    .concat(),
                )
                .context("Error when un-escaping javac output.")?;

                if out.status.success() {
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
            Err(e) => Err(JavaFileError::Unknown(e)),
        }
    }

    /// Returns the inner run of this [`File`].
    fn inner_run(&self, input: Option<String>, err: Stdio, out: Stdio) -> Result<Output> {
        if self.kind != FileType::ClassWithMain {
            Err(JavaFileError::DuringCompilation {
                stacktrace: "The file you wish to run does not have a main method.".into(),
                diags:      vec![],
            })?;
        }

        let class_path = classpath(&self.paths)?;

        if let Some(input_str) = input {
            let mut child = Command::new(java_path()?)
                .arg("--class-path")
                .arg(class_path.as_str())
                .arg(self.proper_name.clone())
                .stdin(Stdio::piped())
                .stdout(out)
                .stderr(err)
                .spawn()
                .context("Failed to spawn javac process.")?;

            let input = format!("{}\r\n", input_str);

            let mut stdin = child.stdin.take().unwrap();

            stdin
                .write_all(input.as_bytes())
                .context("Error when trying to write input to stdin")?;
            stdin.flush().context("Error when trying to flush stdin")?;

            child
                .wait_with_output()
                .context("Error when waiting for child process to finish")
        } else {
            Command::new(java_path()?)
                .arg("--class-path")
                .arg(class_path.as_str())
                .arg(self.proper_name.clone())
                .stdin(Stdio::inherit())
                .stdout(out)
                .stderr(err)
                .spawn()?
                .wait_with_output()
                .context("Failed to spawn javac process.")
        }
    }

    /// Utility method to run a java file that has a main method.
    pub fn run(&self, input: Option<String>) -> Result<String, JavaFileError> {
        self.check()?;

        match self.inner_run(input, Stdio::piped(), Stdio::piped()) {
            Ok(out) => {
                let output = unescape(
                    &[
                        String::from_utf8(out.stderr)
                            .context("Error when parsing stderr as utf8")?,
                        String::from_utf8(out.stdout)
                            .context("Error when parsing stdout as utf8")?,
                    ]
                    .concat(),
                )
                .context("Error when escaping java output.")?;

                if out.status.success() {
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
            Err(e) => Err(anyhow!(e).into()),
        }
    }

    /// Inner method to run tests.
    fn inner_test(&self, tests: Vec<&str>, err: Stdio, out: Stdio, in_: Stdio) -> Result<Output> {
        let tests = {
            let mut new_tests = Vec::<String>::new();
            for t in tests {
                new_tests.push(format!("{}#{}", self.proper_name.clone(), t));
            }

            if new_tests.is_empty() {
                self.test_methods.clone()
            } else {
                new_tests
            }
        };

        let tests = tests
            .iter()
            .map(|s| format!("-m{s}"))
            .collect::<Vec<String>>();
        let methods: Vec<&str> = tests.iter().map(String::as_str).collect();

        let junit_jar = self.paths.lib_dir().join(JUNIT_PLATFORM);
        let class_path = classpath(&self.paths)?;

        let mut command =
            Command::new(java_path().context("Could not find `java` command on path.")?);
        command.stderr(err).stdout(out).stdin(in_);

        command
            .arg("-jar")
            .arg(junit_jar)
            .arg("--disable-banner")
            .arg("--disable-ansi-colors")
            .arg("--details-theme=unicode")
            .arg("--single-color")
            .arg("-cp")
            .arg(class_path.as_str());

        for method in methods {
            command.arg(method);
        }

        command.output().context("Failed to spawn javac process.")
    }

    /// A utility method that takes a list of strings (or types that implement
    /// `Into<String>`) meant to represent test method names, and runs those
    /// tests.
    ///
    /// Returns the output from JUnit as a string. There are parsers in
    /// ['parsers module'][crate::parsers::parser] that helps parse this output.
    ///
    /// * `tests`: list of strings (or types that implement `Into<String>`)
    ///   meant to represent test method names,
    pub fn test(
        &self,
        tests: Vec<&str>,
        project: Option<&Project>,
    ) -> Result<String, JavaFileError> {
        self.check()?;

        match self.inner_test(tests, Stdio::piped(), Stdio::piped(), Stdio::inherit()) {
            Ok(out) => {
                let output = unescape(
                    &[
                        String::from_utf8(out.stderr)
                            .context("Error when parsing stderr as utf8")?,
                        String::from_utf8(out.stdout)
                            .context("Error when parsing stdout as utf8")?,
                    ]
                    .concat(),
                )
                .context("Error when un-escaping JUnit output.")?;

                if out.status.success() {
                    Ok(output)
                } else {
                    let mut diags = Vec::new();
                    let mut new_output = Vec::new();

                    for line in output.lines() {
                        if line.contains("MethodSource") || line.contains("Native Method") {
                            continue;
                        }

                        // if line.contains("Test run finished after") {
                        //     break;
                        // }

                        if let Ok(diag) = parser::junit_stacktrace_line_ref(line) {
                            if let Some(proj) = project
                                && proj.identify(diag.file_name()).is_ok()
                            {
                                new_output.push(
                                    line.replace("\\\\", "\\").replace("\\\"", "\"").to_string(),
                                );
                            }
                            diags.push(diag);
                        } else if let Ok(diag) = parser::parse_diag(line) {
                            if let Some(proj) = project
                                && proj.identify(diag.file_name()).is_ok()
                            {
                                new_output.push(
                                    line.replace("\\\\", "\\").replace("\\\"", "\"").to_string(),
                                );
                            }
                            diags.push(diag.into());
                        } else {
                            new_output
                                .push(line.replace("\\\\", "\\").replace("\\\"", "\"").to_string());
                        }
                    }

                    Err(JavaFileError::FailedTests {
                        test_results: new_output.join("\n"),
                        diags,
                    })
                }
            }
            Err(e) => Err(anyhow!(e).into()),
        }
    }

    /// A utility method that takes a list of strings (or types that implement
    /// `Into<String>`) meant to represent test method names, and runs those
    /// tests.
    ///
    /// Returns the output from JUnit as a string. There are parsers in
    /// ['parsers module'][crate::parsers::parser] that helps parse this output.
    /// Get a reference to the file's kind.
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
