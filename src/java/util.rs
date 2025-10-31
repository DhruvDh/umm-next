use std::{ffi::OsString, path::PathBuf};

use anyhow::{Context, Result};
use which::which;

use super::ProjectPaths;
use crate::util::find_files;

/// Finds and returns the path to the javac binary.
pub fn javac_path() -> Result<OsString> {
    which("javac")
        .map(PathBuf::into_os_string)
        .context("Cannot find a Java Compiler on path (javac)")
}

/// Finds and returns the path to the java binary.
pub fn java_path() -> Result<OsString> {
    which("java")
        .map(PathBuf::into_os_string)
        .context("Cannot find a Java runtime on path (java)")
}

/// Find class and jar files to populate the classpath.
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

/// Find java files in source/test directories to populate the sourcepath.
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
