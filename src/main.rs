#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! # umm
//! ## Introduction
//!
//! A java build tool for novices.
//!
//! ## Installation
//!
//! You would need rust installed, ideally the nightly toolchain. You can visit https://rustup.rs/ to find out how to install this on your computer, just make sure you install the "nightly" toolchain instead of stable.
//!
//! On Linux, Windows Subsystem for Linux (WSL), and Mac you should be able to run `curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly` on a terminal to install the nightly toolchain for rust.
//!
//! Once you are done, just type `cargo install --git=https://github.com/DhruvDh/umm.git` and it should compile and install it on your system.

use anyhow::{Context, Result};
use bpaf::*;
use dotenvy::dotenv;
use self_update::cargo_crate_version;
use tracing::{Level, metadata::LevelFilter};
use tracing_subscriber::{fmt, prelude::*, util::SubscriberInitExt};
use umm::{java::Project, scripting};

/// Updates binary based on github releases
fn update() -> Result<()> {
    self_update::backends::github::Update::configure()
        .repo_owner("dhruvdh")
        .repo_name("umm-next")
        .bin_name("umm")
        .no_confirm(true)
        .target_version_tag("spring_26")
        .show_download_progress(true)
        .show_output(false)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;

    eprintln!("Update done!");
    Ok(())
}

/// Java-specific subcommands.
#[derive(Debug, Clone)]
enum JavaCmd {
    /// Run a file
    Run(String),
    /// Check a file
    Check(String),
    /// Test a file
    Test(String, Vec<String>),
    /// Check a file's documentation
    DocCheck(String),
    /// Grade a file
    Grade(String),
    /// Print information about the project
    Info,
}

/// Top-level CLI commands.
#[derive(Debug, Clone)]
enum Cmd {
    /// Java-related operations
    Java(JavaCmd),
    /// Update the command
    Update,
}

/// Parse the command line arguments and return a `Cmd` enum
fn options() -> Cmd {
    /// parses test names
    fn t() -> impl Parser<Vec<String>> {
        positional("TESTNAME")
            .help("Name of JUnit test to run")
            .many()
    }

    /// parsers file name
    fn f() -> impl Parser<String> {
        positional("FILENAME").help("Name of java file")
    }

    /// parses Assignment name or path to grading script file
    fn g() -> impl Parser<String> {
        positional("NAME/PATH").help("Name of assignment in database or path to grading script")
    }

    let java_run = construct!(JavaCmd::Run(f()))
        .to_options()
        .command("run")
        .help("Run a java file with a main method");

    let java_check = construct!(JavaCmd::Check(f()))
        .to_options()
        .command("check")
        .help("Check for syntax errors");

    let java_test = construct!(JavaCmd::Test(f(), t()))
        .to_options()
        .command("test")
        .help("Run JUnit tests");

    let java_doc_check = construct!(JavaCmd::DocCheck(f()))
        .to_options()
        .command("doc-check")
        .help("Check a file for missing javadoc");

    let java_grade = construct!(JavaCmd::Grade(g()))
        .to_options()
        .command("grade")
        .help("Grade your work");

    let java_info = pure(JavaCmd::Info)
        .to_options()
        .command("info")
        .help("Prints a JSON description of the project as parsed");

    let java = construct!([
        java_run,
        java_check,
        java_test,
        java_doc_check,
        java_grade,
        java_info
    ])
    .to_options()
    .command("java")
    .help("Java project commands")
    .map(Cmd::Java);

    let update = pure(Cmd::Update)
        .to_options()
        .command("update")
        .help("Update the umm command");

    let cmd = construct!([java, update]);

    cmd.to_options().descr("Build tool for novices").run()
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let fmt = fmt::layer()
        .without_time()
        .with_file(false)
        .with_line_number(false);
    let filter_layer = LevelFilter::from_level(Level::INFO);
    tracing_subscriber::registry()
        .with(fmt)
        .with(filter_layer)
        .init();

    let cmd = options();

    match cmd {
        Cmd::Java(java_cmd) => match java_cmd {
            JavaCmd::Run(f) => {
                let file = Project::new()?.identify(f.as_str())?;
                match file.run(None).await {
                    Ok(out) => println!("{out}"),
                    Err(e) => eprintln!("{:#?}", e),
                }
            }
            JavaCmd::Check(f) => {
                let file = Project::new()?.identify(f.as_str())?;
                match file.check().await {
                    Ok(out) => println!("{out}"),
                    Err(e) => eprintln!("{:#?}", e),
                }
            }
            JavaCmd::Test(f, t) => {
                let project = Project::new()?;
                let file = project.identify(f.as_str())?;
                let out = if t.is_empty() {
                    file.test(Vec::<&str>::new(), Some(&project)).await?
                } else {
                    let test_refs: Vec<&str> = t.iter().map(String::as_str).collect();
                    file.test(test_refs, Some(&project)).await?
                };

                println!("{out}");
            }
            JavaCmd::DocCheck(f) => {
                let file = Project::new()?.identify(f.as_str())?;
                let out = file.doc_check().await?;
                println!("{out}");
            }
            JavaCmd::Grade(g) => {
                scripting::run_file(&g)
                    .await
                    .with_context(|| format!("Failed to execute Rune script `{}`", g))?;
            }
            JavaCmd::Info => Project::new()?.info()?,
        },
        Cmd::Update => {
            match update() {
                Ok(_) => {}
                Err(e) => eprintln!("{e}"),
            };
        }
    };

    Ok(())
}
