#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Python project support for the umm autograder.
//!
//! This module provides functionality for discovering, analyzing, and grading
//! Python projects, mirroring the structure of the Java module.

/// Python-specific configuration helpers.
pub mod config;
/// File type definitions and helpers.
pub mod file;
/// Python-specific grading utilities.
pub mod grade;
/// Tree-sitter parser wrapper.
pub mod parser;
/// Project path configuration helpers.
pub mod paths;
/// Python project discovery and operations.
pub mod project;
/// Tree-sitter query strings used by Python analysis.
pub mod queries;
/// Python-specific filesystem and toolchain helpers.
pub mod util;

pub use config::{PythonConfig, PythonPrompts};
pub use file::{File, FileType, PythonFileError};
pub use parser::Parser;
pub use paths::ProjectPaths;
pub use project::Project;
