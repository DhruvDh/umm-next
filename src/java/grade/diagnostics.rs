use std::{
    fmt::{self, Display},
    path::Path,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use tabled::Tabled;
use typed_builder::TypedBuilder;

use crate::types::LineRef;

#[derive(Tabled, Serialize, Deserialize, TypedBuilder, Clone, Debug)]
#[builder(field_defaults(setter(into)))]
#[builder(doc)]
/// A struct representing a javac diagnostic message
pub struct JavacDiagnostic {
    /// * `path`: path to the file diagnostic is referring to
    #[tabled(rename = "File")]
    path:        String,
    /// * `file_name`: name of the file the diagnostic is about
    #[tabled(skip)]
    file_name:   String,
    /// Type of diagnostic (error or warning).
    #[tabled(skip)]
    severity:    DiagnosticSeverity,
    /// * `line_number`: line number
    #[tabled(rename = "Line")]
    line_number: u32,
    /// * `message`: the diagnostic message
    #[tabled(rename = "Message")]
    message:     String,
}

impl JavacDiagnostic {
    /// Returns the file name
    pub fn file_name(&self) -> &str {
        self.file_name.as_ref()
    }

    /// Returns the path to the diagnostic’s file.
    pub fn path(&self) -> &Path {
        Path::new(&self.path)
    }

    /// Returns the severity of the diagnostic.
    pub fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }
}

impl From<JavacDiagnostic> for LineRef {
    /// Converts a JavacDiagnostic to a LineRef
    fn from(val: JavacDiagnostic) -> Self {
        LineRef {
            file_name:   val.file_name,
            line_number: val.line_number as usize,
        }
    }
}

#[derive(Tabled, Serialize, Deserialize, TypedBuilder, Clone)]
#[builder(field_defaults(setter(into)))]
#[builder(doc)]
/// A struct representing a PIT diagnostic message
pub struct MutationDiagnostic {
    /// * `mutator`: name of the mutator in question
    #[tabled(rename = "Mutation type")]
    mutator:          String,
    /// * `source_method`: name of the source method being mutated
    #[tabled(rename = "Source method mutated")]
    source_method:    String,
    /// * `line_number`: source line number where mutation occurred
    #[tabled(rename = "Line no. of mutation")]
    line_number:      u32,
    /// * `test_method`: name of the test examined
    #[tabled(rename = "Test examined")]
    test_method:      String,
    /// * `result`: result of mutation testing
    #[tabled(rename = "Result")]
    result:           MutationTestResult,
    /// * `source_file_name`: name of the source file
    #[tabled(skip)]
    source_file_name: String,
    /// * `test_file_name`: name of the test file
    #[tabled(skip)]
    test_file_name:   String,
}

impl From<MutationDiagnostic> for LineRef {
    /// Converts a MutationDiagnostic to a LineRef
    fn from(val: MutationDiagnostic) -> Self {
        LineRef {
            file_name:   val.source_file_name,
            line_number: val.line_number as usize,
        }
    }
}

impl MutationDiagnostic {
    /// Returns the mutation result status.
    pub fn result(&self) -> &str {
        self.result.as_str()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// Severity of a diagnostic emitted by `javac`.
pub enum DiagnosticSeverity {
    /// Diagnostic raised as an error.
    Error,
    /// Diagnostic raised as a warning.
    Warning,
}

impl DiagnosticSeverity {
    fn as_str(self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "ERROR",
            DiagnosticSeverity::Warning => "WARNING",
        }
    }

    /// Indicates whether the severity represents an error.
    pub fn is_error(self) -> bool {
        matches!(self, DiagnosticSeverity::Error)
    }
}

impl From<bool> for DiagnosticSeverity {
    fn from(value: bool) -> Self {
        if value {
            DiagnosticSeverity::Error
        } else {
            DiagnosticSeverity::Warning
        }
    }
}

impl Serialize for DiagnosticSeverity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DiagnosticSeverity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "ERROR" => Ok(DiagnosticSeverity::Error),
            "WARNING" => Ok(DiagnosticSeverity::Warning),
            other => Err(de::Error::custom(format!("Unknown diagnostic severity: {other}"))),
        }
    }
}

impl Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Result of a PIT mutation.
pub enum MutationTestResult {
    /// Mutation survived.
    Survived,
    /// Mutation was killed.
    Killed,
    /// Mutation returned a different status (e.g., NO_COVERAGE, TIMED_OUT).
    Other(String),
}

impl MutationTestResult {
    /// Returns the canonical string representation used in reports.
    pub fn as_str(&self) -> &str {
        match self {
            MutationTestResult::Survived => "SURVIVED",
            MutationTestResult::Killed => "KILLED",
            MutationTestResult::Other(value) => value.as_str(),
        }
    }
}

impl From<String> for MutationTestResult {
    fn from(value: String) -> Self {
        match value.as_str() {
            "SURVIVED" => MutationTestResult::Survived,
            "KILLED" => MutationTestResult::Killed,
            _ => MutationTestResult::Other(value),
        }
    }
}

impl Serialize for MutationTestResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MutationTestResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(MutationTestResult::from(value))
    }
}

impl Display for MutationTestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
