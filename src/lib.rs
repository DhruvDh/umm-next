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
use constants::{
    BUILD_DIR, COURSE, LIB_DIR, POSTGREST_CLIENT, ROOT_DIR, RUNTIME, SCRIPT_AST, TERM,
};
use grade::*;
use java::{File, FileType, Parser, Project};
use rhai::{Engine, EvalAltResult};
use umm_derive::generate_rhai_variant;
use util::{use_active_retrieval, use_heuristic_retrieval};

/// Defined for convenience
type Dict = std::collections::HashMap<String, String>;

/// Creates and returns a new `Engine` with all the types and functions
/// registered
pub fn create_engine() -> Engine {
    let mut engine = Engine::new();
    engine
        .register_type_with_name::<FileType>("JavaFileType")
        .build_type::<DocsGrader>()
        .build_type::<ByUnitTestGrader>()
        .build_type::<UnitTestGrader>()
        .build_type::<ByHiddenTestGrader>()
        .build_type::<DiffGrader>()
        .build_type::<Grade>()
        .build_type::<GradeResult>()
        .build_type::<Parser>()
        .build_type::<File>()
        .build_type::<Query>()
        .build_type::<QueryGrader>()
        .build_type::<Project>()
        .register_fn("clean", clean_script)
        .register_fn("show_results", show_result_script)
        .register_fn("generate_single_feedback", generate_single_feedback_script)
        .register_fn("generate_feedback", generate_feedback_script)
        .register_fn("use_active_retrieval", use_active_retrieval)
        .register_fn("use_heuristic_retrieval", use_heuristic_retrieval);
    engine
}

/// Prints the result of grading
pub fn grade(name_or_path: &str) -> Result<()> {
    let engine = create_engine();

    // println!("{}", engine.gen_fn_signatures(false).join("\n"));
    let script = match std::fs::read_to_string(name_or_path) {
        Ok(s) => s,
        Err(_) => {
            let assignment_name = name_or_path.to_string().replace(['\"', '\\'], "");
            let rt = RUNTIME.handle().clone();

            let resp = rt.block_on(async {
                POSTGREST_CLIENT
                    .from("grading_scripts")
                    .eq("course", COURSE)
                    .eq("term", TERM)
                    .eq("assignment", &assignment_name)
                    .select("url")
                    .single()
                    .execute()
                    .await?
                    .text()
                    .await
                    .context(format!("Could not get grading script for {assignment_name}"))
            });

            let resp: serde_json::Value = serde_json::from_str(resp?.as_str())?;
            let resp = resp.as_object().unwrap();

            if let Some(message) = resp.get("message") {
                anyhow::bail!("Error for {assignment_name}: {message}");
            }

            let script_url = resp.get("url").unwrap().as_str().unwrap();

            reqwest::blocking::get(script_url)
                .context(format!("Cannot get url: {script_url}"))?
                .text()
                .context(format!("Could not parse the response from {script_url} to text."))?
        }
    };
    let compiled_ast = engine.compile(script)?;
    {
        let ast_cell = std::sync::Arc::clone(&SCRIPT_AST);
        let mut ast_lock = ast_cell.lock().unwrap();
        *ast_lock = compiled_ast.clone();
    }

    // Run the script
    engine.run_ast(&compiled_ast)?;

    Ok(())
}

#[generate_rhai_variant(Fallible)]
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
