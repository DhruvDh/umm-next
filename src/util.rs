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

use crate::constants::*;

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
    let mut root_dir = PathBuf::from(root_dir);

    for _ in 0..search_depth {
        root_dir.push("**");
    }

    root_dir.push(format!("*.{extension}"));
    let root_dir = root_dir
        .to_str()
        .context("Could not convert root_dir to string")?;

    Ok(glob(root_dir)
        .context("Could not create glob")?
        .filter_map(Result::ok)
        .map(|path| ROOT_DIR.join(path))
        .collect())
}

/// Find class, jar files in library path and build directory to populate
/// classpath and return it
pub fn classpath() -> Result<String> {
    let mut path: Vec<String> = vec![
        LIB_DIR.display().to_string(),
        BUILD_DIR.display().to_string(),
    ];

    path.append(
        &mut find_files("jar", 4, &ROOT_DIR)?
            .iter()
            .map(|p| p.as_path().display().to_string())
            .collect(),
    );

    Ok(path.join(&SEPARATOR))
}

/// Find java files in source path and root directory to populate
/// sourcepath and return it
pub fn sourcepath() -> Result<String> {
    let mut path: Vec<String> = vec![
        SOURCE_DIR.join("").display().to_string(),
        TEST_DIR.join("").display().to_string(),
        ROOT_DIR.join("").display().to_string(),
    ];

    path.append(
        &mut find_files("java", 4, &ROOT_DIR)?
            .iter()
            .map(|p| p.as_path().display().to_string())
            .collect(),
    );

    Ok(path.join(&SEPARATOR))
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
