#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Python diagnostic helper data structures.

use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::types::LineRef;

/// Represents a diagnostic message from Python (syntax error, runtime error,
/// lint).
#[derive(Tabled, Serialize, Deserialize, Clone, Debug)]
pub struct PythonDiagnostic {
    /// Path to the file.
    #[tabled(rename = "File")]
    path:        String,
    /// File name only.
    #[tabled(skip)]
    file_name:   String,
    /// Line number where the diagnostic occurred.
    #[tabled(rename = "Line")]
    line_number: u32,
    /// Column number (if available).
    #[tabled(skip)]
    column:      Option<u32>,
    /// The diagnostic message.
    #[tabled(rename = "Message")]
    message:     String,
    /// Severity of the diagnostic.
    #[tabled(rename = "Severity")]
    severity:    DiagnosticSeverity,
}

impl PythonDiagnostic {
    /// Creates a new diagnostic.
    pub fn new(
        path: impl Into<String>,
        file_name: impl Into<String>,
        line_number: u32,
        message: impl Into<String>,
        severity: DiagnosticSeverity,
    ) -> Self {
        Self {
            path: path.into(),
            file_name: file_name.into(),
            line_number,
            column: None,
            message: message.into(),
            severity,
        }
    }

    /// Returns the file name.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Returns the line number.
    pub fn line_number(&self) -> u32 {
        self.line_number
    }

    /// Returns the message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the severity.
    pub fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }
}

impl From<PythonDiagnostic> for LineRef {
    fn from(val: PythonDiagnostic) -> Self {
        LineRef {
            file_name:   val.file_name,
            line_number: val.line_number as usize,
        }
    }
}

/// Severity of a Python diagnostic.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DiagnosticSeverity {
    /// Syntax or runtime error.
    #[default]
    Error,
    /// Warning from linter.
    Warning,
    /// Informational note.
    Info,
}

impl DiagnosticSeverity {
    /// Returns the string representation.
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "ERROR",
            DiagnosticSeverity::Warning => "WARNING",
            DiagnosticSeverity::Info => "INFO",
        }
    }

    /// Returns true if this is an error.
    pub fn is_error(self) -> bool {
        matches!(self, DiagnosticSeverity::Error)
    }
}

impl std::fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
