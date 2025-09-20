//! # umm
//!
//! A scriptable build tool/grader/test runner for Java projects that don't use
//! package managers.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]
#![feature(iterator_try_collect)]
#![feature(stmt_expr_attributes)]
#![feature(iter_collect_into)]

/// A module defining a bunch of constant values to be used throughout
pub mod constants;
/// For all things related to grading
pub mod grade;
/// For all things related to project health
pub mod health;
/// For discovering Java projects, analyzing them, and generating/executing
/// build tasks
pub mod java;
/// For all parsers used
pub mod parsers;
/// Utility functions for convenience
pub mod util;
/// For structs and enums related to VSCode Tasks
pub mod vscode;

use anyhow::{Context, Result};
use constants::{BUILD_DIR, LIB_DIR, ROOT_DIR};
use rhai::Engine;

/// Defined for convenience
type Dict = std::collections::HashMap<String, String>;

/// Creates and returns a new bare `Engine` placeholder while Rhai support is
/// being phased out.
pub fn create_engine() -> Engine {
    Engine::new()
}

/// Prints the result of grading
pub fn grade(_name_or_path: &str) -> Result<()> {
    anyhow::bail!(
        "The grade command is temporarily unavailable while Rhai support is being removed."
    )
}

/// Deletes all java compiler artefacts
pub fn clean() -> Result<()> {
    if BUILD_DIR.as_path().exists() {
        std::fs::remove_dir_all(BUILD_DIR.as_path())
            .with_context(|| format!("Could not delete {}", BUILD_DIR.display()))?;
    }
    if LIB_DIR.as_path().exists() {
        std::fs::remove_dir_all(LIB_DIR.as_path())
            .with_context(|| format!("Could not delete {}", LIB_DIR.display()))?;
    }
    if ROOT_DIR.join(".vscode/settings.json").as_path().exists() {
        std::fs::remove_file(ROOT_DIR.join(".vscode/settings.json").as_path()).with_context(
            || format!("Could not delete {}", ROOT_DIR.join(".vscode/settings.json").display()),
        )?;
    }
    if ROOT_DIR.join(".vscode/tasks.json").as_path().exists() {
        std::fs::remove_file(ROOT_DIR.join(".vscode/tasks.json").as_path()).with_context(|| {
            format!("Could not delete {}", ROOT_DIR.join(".vscode/tasks.json").display())
        })?;
    }

    Ok(())
}

// TODO: replace std::Command with cmd_lib
// TODO: Lazily load all constants from rhai scripts instead
// TODO: Fix java mod impls
// TODO: update classpath when discovering project
// TODO: fix grading api
// TODO: add rhai scripting for grading
// TODO: find a way to generate a rhai wrapper for all methods
// TODO: add rhai scripting for project init
// TODO: update tabled to 0.6
// TODO: make reedline shell optional behind a feature
// TODO: Download jars only if required OR remove jar requirement altogether.
