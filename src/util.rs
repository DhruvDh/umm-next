#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use glob::glob;
use tokio::io::AsyncWriteExt;
use which::which;

use crate::{constants::USE_ACTIVE_RETRIEVAL, java::ProjectPaths};

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

/// TODO: Add docs
pub async fn download(url: &str, path: &PathBuf, replace: bool) -> Result<()> {
    if !replace && path.exists() {
        Ok(())
    } else {
        let bytes = reqwest::get(url)
            .await
            .context(format!("Failed to download url: {url}"))?
            .bytes()
            .await
            .context(format!("Failed to read response as bytes: {url}"))?;

        let name = path.file_name().unwrap().to_str().unwrap();

        let mut file = tokio::fs::File::create(path)
            .await
            .context(format!("Failed to create file at {name}"))?;

        file.write_all(&bytes)
            .await
            .context(format!("Failed to write to file at {name}"))
    }
}

/// Download a URL and return response as string
pub async fn download_to_string(url: &str) -> Result<String> {
    reqwest::get(url)
        .await
        .context(format!("Failed to download url: {url}"))?
        .text()
        .await
        .context(format!("Failed to read response as text: {url}"))
}

/// Download a URL and return response as JSON
pub async fn download_to_json(url: &str) -> Result<HashMap<String, String>> {
    reqwest::get(url)
        .await
        .context(format!("Failed to download url: {url}"))?
        .json()
        .await
        .context(format!("Failed to read response as json: {url}"))
}

/// Use active retrieval when retrieving context from student submission.
pub fn use_active_retrieval() {
    USE_ACTIVE_RETRIEVAL.set(true);
    dbg!(USE_ACTIVE_RETRIEVAL.get());
}

/// Use heuristic based retrieval when retrieving context from student
/// submission.
pub fn use_heuristic_retrieval() {
    USE_ACTIVE_RETRIEVAL.set(false);
}
