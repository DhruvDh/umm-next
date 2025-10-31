#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

/// Java-specific configuration helpers.
pub mod config;
/// File type definitions and helpers.
pub mod file;
/// Java-specific grading utilities.
pub mod grade;
/// Tree-sitter parser wrapper.
pub mod parser;
/// Parsers for javac, JUnit, and mutation-testing outputs.
pub mod parsers;
/// Project path configuration helpers.
pub mod paths;
/// Java project discovery and operations.
pub mod project;
/// Tree-sitter query strings used by Java analysis.
pub mod queries;
/// Java-specific filesystem and toolchain helpers.
pub mod util;

pub use config::{JavaConfig, JavaPrompts};
pub use file::{File, FileType, JavaFileError};
pub use parser::Parser;
pub use paths::ProjectPaths;
pub use project::Project;
