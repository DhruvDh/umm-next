/// Represents a source location identified by file name and line number.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct LineRef {
    /// The line number within the file.
    pub line_number: usize,
    /// The file name associated with the diagnostic.
    pub file_name:   String,
}

impl LineRef {
    /// Returns the file name for this reference.
    pub fn file_name(&self) -> &str {
        self.file_name.as_ref()
    }
}
