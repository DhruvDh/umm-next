#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use glob::glob;
use reqwest::Client;
use state::InitCell;
use tokio::io::AsyncWriteExt;
use which::which;

use crate::{config, java::ProjectPaths};

/// Shared reqwest client configured with CLI-friendly defaults.
static HTTP_CLIENT: InitCell<Client> = InitCell::new();

/// Returns the shared reqwest client, initializing it on demand.
fn http_client() -> &'static Client {
    if let Some(client) = HTTP_CLIENT.try_get() {
        client
    } else {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent(concat!("umm/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to construct HTTP client");
        HTTP_CLIENT.set(client);
        HTTP_CLIENT.get()
    }
}

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
        let response = http_client()
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to download url: {url}"))?
            .error_for_status()
            .with_context(|| format!("Request returned error status for url: {url}"))?;

        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("Failed to read response as bytes: {url}"))?;

        let display_name = path.display().to_string();

        let mut file = tokio::fs::File::create(path)
            .await
            .with_context(|| format!("Failed to create file at {display_name}"))?;

        file.write_all(&bytes)
            .await
            .with_context(|| format!("Failed to write to file at {display_name}"))
    }
}

/// Download a URL and return response as string
pub async fn download_to_string(url: &str) -> Result<String> {
    http_client()
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to download url: {url}"))?
        .error_for_status()
        .with_context(|| format!("Request returned error status for url: {url}"))?
        .text()
        .await
        .with_context(|| format!("Failed to read response as text: {url}"))
}

/// Download a URL and return response as JSON
pub async fn download_to_json(url: &str) -> Result<HashMap<String, String>> {
    http_client()
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to download url: {url}"))?
        .error_for_status()
        .with_context(|| format!("Request returned error status for url: {url}"))?
        .json()
        .await
        .with_context(|| format!("Failed to read response as json: {url}"))
}

/// Use active retrieval when retrieving context from student submission.
pub fn use_active_retrieval() {
    config::set_active_retrieval(true);
}

/// Use heuristic based retrieval when retrieving context from student
/// submission.
pub fn use_heuristic_retrieval() {
    config::set_active_retrieval(false);
}
