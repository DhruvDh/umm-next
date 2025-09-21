use serde::{Deserialize, Serialize};
use tabled::Tabled;
use typed_builder::TypedBuilder;

#[derive(Debug, Hash, PartialEq, Eq)]
/// A struct representing a line in a stack trace
pub struct LineRef {
    /// The line number
    pub line_number: usize,
    /// The file name
    pub file_name:   String,
}

impl LineRef {
    /// Returns the file name
    pub fn file_name(&self) -> &str {
        self.file_name.as_ref()
    }
}

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
    /// * `line_number`: line number
    #[tabled(rename = "Line")]
    line_number: u32,
    /// * `is_error`: boolean value, is true if error or false if the diagnostic
    ///   is a warning
    #[tabled(skip)]
    is_error:    bool,
    /// * `message`: the diagnostic message
    #[tabled(rename = "Message")]
    message:     String,
}

impl JavacDiagnostic {
    /// Returns the file name
    pub fn file_name(&self) -> &str {
        self.file_name.as_ref()
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
    result:           String,
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
