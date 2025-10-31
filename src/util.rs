#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use glob::glob;
use which::which;

/// Finds and returns the path to the `umm` binary (falls back to `./umm`).
pub fn umm_path() -> String {
    match which("umm") {
        Ok(path) => path.display().to_string(),
        Err(_) => "./umm".into(),
    }
}

/// Generic glob helper to discover files under `root_dir` matching `extension`.
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
