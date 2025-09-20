//! # umm
//! ## Introduction

//! A java build tool for novices.

//! ## Installation

//! You would need rust installed, ideally the nightly toolchain. You can visit https://rustup.rs/ to find out how to install this on your computer, just make sure you install the "nightly" toolchain instead of stable.

//! On Linux, Windows Subsystem for Linux (WSL), and Mac you should be able to run `curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly` on a terminal to install the nightly toolchain for rust.

//! Once you are done, just type `cargo install --git=https://github.com/DhruvDh/umm.git` and it should compile and install it on your system.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    collections::HashSet,
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::Result;
use bpaf::*;
use dotenvy::dotenv;
use self_update::cargo_crate_version;
use tracing::{Level, metadata::LevelFilter};
use tracing_subscriber::{fmt, prelude::*, util::SubscriberInitExt};
use umm::{
    clean, grade,
    java::{Project, ProjectPaths},
};
use walkdir::WalkDir;

/// Updates binary based on github releases
fn update() -> Result<()> {
    self_update::backends::github::Update::configure()
        .repo_owner("dhruvdh")
        .repo_name("umm")
        .bin_name("umm")
        .no_confirm(true)
        .target_version_tag("spring_24")
        .show_download_progress(true)
        .show_output(false)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;

    eprintln!("Update done!");
    Ok(())
}

/// Enum to represent different commands
#[derive(Debug, Clone)]
enum Cmd {
    /// Run a file
    Run(String),
    /// Check a file
    Check(String),
    /// Test a file
    Test(String, Vec<String>),
    /// Check a files documentation
    DocCheck(String),
    /// Grade a file
    Grade(String),
    /// Create a submission zip
    CreateSubmission(String),
    /// Clean the project artifacts
    Clean,
    /// Print information about the project
    Info,
    /// Update the command
    Update,
    /// Checks project health
    CheckHealth,
    /// Starts and serves a web server that serves the project code
    ServeProjectCode,
    /// Resets the project metadata, and re-downloads libraries
    Reset,
    /// Exit the program
    Exit,
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

    /// parses path to project root folder
    fn h() -> impl Parser<String> {
        positional("PATH")
            .help("Path to project root folder. Defaults to current directory")
            .fallback(String::from("."))
    }

    let run = construct!(Cmd::Run(f()))
        .to_options()
        .command("run")
        .help("Run a java file with a main method");

    let check = construct!(Cmd::Check(f()))
        .to_options()
        .command("check")
        .help("Check for syntax errors");

    let test = construct!(Cmd::Test(f(), t()))
        .to_options()
        .command("test")
        .help("Run JUnit tests");

    let doc_check = construct!(Cmd::DocCheck(f()))
        .to_options()
        .command("doc-check")
        .help("Check a file for missing javadoc");

    let grade = construct!(Cmd::Grade(g()))
        .to_options()
        .command("grade")
        .help("Grade your work");

    let create_submission = construct!(Cmd::CreateSubmission(h()))
        .to_options()
        .command("create-submission")
        .help("Create a submission zip");

    let clean = pure(Cmd::Clean)
        .to_options()
        .command("clean")
        .help("Cleans the build folder, library folder, and vscode settings");

    let info = pure(Cmd::Info)
        .to_options()
        .command("info")
        .help("Prints a JSON description of the project as parsed");

    let update = pure(Cmd::Update)
        .to_options()
        .command("update")
        .help("Update the umm command");

    let check_health = pure(Cmd::CheckHealth)
        .to_options()
        .command("check-health")
        .help("Checks the health of the project");

    let serve = pure(Cmd::ServeProjectCode)
        .to_options()
        .command("serve-project-code")
        .help("Starts and serves a web server that serves the project code");

    let reset = pure(Cmd::Reset)
        .to_options()
        .command("reset")
        .help("Reset the project metadata, and re-download libraries");

    let exit = pure(Cmd::Exit)
        .to_options()
        .command("exit")
        .help("Exit the program");

    let cmd = construct!([
        run,
        check,
        test,
        doc_check,
        grade,
        create_submission,
        clean,
        info,
        update,
        check_health,
        serve,
        reset,
        exit
    ])
    .fallback(Cmd::Exit);

    cmd.to_options().descr("Build tool for novices").run()
}

fn main() -> Result<()> {
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

    // TODO: move this to a separate method and call that method in shell()
    match cmd {
        Cmd::Run(f) => {
            let file = Project::new()?.identify(f.as_str())?;
            match file.run(None) {
                Ok(out) => println!("{out}"),
                Err(e) => eprintln!("{:#?}", e),
            }
        }
        Cmd::Check(f) => {
            let file = Project::new()?.identify(f.as_str())?;
            match file.check() {
                Ok(out) => println!("{out}"),
                Err(e) => eprintln!("{:#?}", e),
            }
        }
        Cmd::Test(f, t) => {
            let project = Project::new()?;
            let file = project.identify(f.as_str())?;
            let out = if t.is_empty() {
                file.test(Vec::<&str>::new(), Some(&project))?
            } else {
                let test_refs: Vec<&str> = t.iter().map(String::as_str).collect();
                file.test(test_refs, Some(&project))?
            };

            println!("{out}");
        }
        Cmd::DocCheck(f) => {
            let file = Project::new()?.identify(f.as_str())?;
            let out = file.doc_check()?;
            println!("{out}");
        }
        Cmd::Grade(g) => grade(&g)?,
        Cmd::CreateSubmission(p) => {
            let paths = ProjectPaths::default();
            let zip_file_name = format!(
                "submission-{}.zip",
                chrono::offset::Local::now().format("%Y-%m-%d-%H-%M-%S")
            );
            let zip_file = std::fs::File::create(PathBuf::from(zip_file_name.clone()))?;

            let all_files = {
                let source_walkdir: Vec<_> = WalkDir::new(paths.source_dir())
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .collect();
                let lib_walkdir: Vec<_> = WalkDir::new(paths.lib_dir())
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .collect();
                let test_walkdir: Vec<_> = WalkDir::new(paths.test_dir())
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .collect();
                let all_java_files: Vec<_> = WalkDir::new(PathBuf::from(p).as_path())
                    .into_iter()
                    .filter_map(|e| {
                        e.ok()
                            .filter(|x| x.path().extension().unwrap_or_default() == "java")
                    })
                    .collect();

                source_walkdir
                    .into_iter()
                    .chain(lib_walkdir)
                    .chain(test_walkdir)
                    .chain(all_java_files)
            };

            let mut zip = zip::ZipWriter::new(zip_file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o755);
            let mut buffer = Vec::new();
            let mut already_added = HashSet::<PathBuf>::new();

            for entry in all_files {
                let path = match entry.path().strip_prefix(paths.root_dir()) {
                    Ok(path) => path,
                    Err(_) => entry.path(),
                };

                if already_added.contains(path) {
                    continue;
                } else {
                    already_added.insert(path.to_path_buf());
                }

                let mut name = paths.root_dir().to_path_buf();
                name.push(path);

                if path.is_file() {
                    #[allow(deprecated)]
                    zip.start_file_from_path(name.as_path(), options)?;
                    let mut f = std::fs::File::open(path)?;

                    f.read_to_end(&mut buffer)?;
                    zip.write_all(&buffer)?;
                    buffer.clear();
                } else if !name.as_os_str().is_empty() {
                    // Only if not root! Avoids path spec / warning
                    // and mapname conversion failed error on unzip
                    #[allow(deprecated)]
                    zip.add_directory_from_path(name.as_path(), options)?;
                }
            }

            zip.finish()?;
            println!("Submission zip created - {}", zip_file_name);
        }
        Cmd::Clean => clean()?,
        Cmd::Info => Project::new()?.info()?,
        Cmd::Update => {
            match update() {
                Ok(_) => {}
                Err(e) => eprintln!("{e}"),
            };
        }
        Cmd::CheckHealth => Project::new()?.check_health()?,
        Cmd::ServeProjectCode => Project::new()?.serve_project_code()?,
        Cmd::Reset => {
            clean()?;
            Project::new()?;
        }
        Cmd::Exit => {}
    };

    Ok(())
}
