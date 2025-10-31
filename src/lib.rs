//! # umm
//!
//! A scriptable build tool/grader/test runner for Java projects that don't use
//! package managers.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]
#![feature(iterator_try_collect)]
#![feature(stmt_expr_attributes)]
#![feature(iter_collect_into)]

/// Shared, runtime-initialized configuration (prompts, services, env)
pub mod config;
// For all things related to grading see `java::grade` module.
/// For discovering Java projects, analyzing them, and generating/executing
/// build tasks
pub mod java;
/// Async process helpers shared across modules.
pub mod process;
/// Retrieval-mode definitions shared across languages.
pub mod retrieval;
/// Shared data structures reused across modules.
pub mod types;
/// Utility functions for convenience
pub mod util;
use anyhow::Result;

/// Defined for convenience
type Dict = std::collections::HashMap<String, String>;

/// Prints the result of grading
pub fn grade(_name_or_path: &str) -> Result<()> {
    anyhow::bail!(
        "The grade command is temporarily unavailable while Rhai support is being removed."
    )
}
