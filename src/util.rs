#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use glob::glob;
use which::which;

use crate::java::ProjectPaths;

/// Finds and returns the path to javac binary
pub fn javac_path() -> Result<OsString> {
    which("javac")
        .map(PathBuf::into_os_string)
        .context("Cannot find a Java Compiler on path (javac)")
}

/// Finds and returns the path to java binary
pub fn java_path() -> Result<OsString> {
    which("java")
        .map(PathBuf::into_os_string)
        .context("Cannot find a Java runtime on path (java)")
}

/// Finds and returns the path to umm
/// If not found, returns "./umm"
pub fn umm_path() -> String {
    match which("umm") {
        Ok(path) => path.display().to_string(),
        Err(_) => "./umm".into(),
    }
}

/// A glob utility function to find paths to files with certain extension
///
/// * `extension`: the file extension to find paths for
/// * `search_depth`: how many folders deep to search for
/// * `root_dir`: the root directory where search starts
pub fn find_files(extension: &str, search_depth: i8, root_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut pattern = root_dir.to_path_buf();

    for _ in 0..search_depth {
        pattern.push("**");
    }

    pattern.push(format!("*.{extension}"));
    let pattern = pattern
        .to_str()
        .context("Could not convert root_dir to string")?
        .to_string();

    Ok(glob(&pattern)
        .context("Could not create glob")?
        .filter_map(Result::ok)
        .collect())
}

/// Find class, jar files in library path and build directory to populate
/// classpath and return it
pub fn classpath(paths: &ProjectPaths) -> Result<String> {
    let mut entries: Vec<String> = vec![
        paths.lib_dir().display().to_string(),
        paths.build_dir().display().to_string(),
    ];

    entries.append(
        &mut find_files("jar", 4, paths.root_dir())?
            .iter()
            .map(|p| p.as_path().display().to_string())
            .collect(),
    );

    Ok(entries.join(paths.separator()))
}

/// Find java files in source path and root directory to populate
/// sourcepath and return it
pub fn sourcepath(paths: &ProjectPaths) -> Result<String> {
    let mut entries: Vec<String> = vec![
        paths.source_dir().display().to_string(),
        paths.test_dir().display().to_string(),
        paths.root_dir().display().to_string(),
    ];

    entries.append(
        &mut find_files("java", 4, paths.root_dir())?
            .iter()
            .map(|p| p.as_path().display().to_string())
            .collect(),
    );

    Ok(entries.join(paths.separator()))
}
