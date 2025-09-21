#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

/// File type definitions and helpers.
pub mod file;
/// Java-specific grading utilities.
pub mod grade;
/// Tree-sitter parser wrapper.
pub mod parser;
/// Project path configuration helpers.
pub mod paths;
/// Java project discovery and operations.
pub mod project;

pub use file::{File, FileType, JavaFileError};
pub use parser::Parser;
pub use paths::ProjectPaths;
pub use project::Project;
