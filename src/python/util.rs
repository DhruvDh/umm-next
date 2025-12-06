#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Python-specific utility functions for toolchain and filesystem operations.
//!
//! This module uses `uv` as the primary tool for running Python scripts and
//! managing dependencies. `uv` is Astral's fast Python package manager that
//! automatically handles virtual environments and dependencies.
//!
//! Key patterns:
//! - Script execution: `uv run <script.py>`
//! - Module with deps: `uv run --with <pkg> -- python -m <module> <args>`
//! - Simple module: `uv run -- python -m <module> <args>`

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow};

use super::paths::ProjectPaths;
use crate::util::find_files;

/// Discovers the path to `uv`.
///
/// Returns the path to `uv` if found, or an error if not available.
/// `uv` is the preferred tool for running Python scripts as it handles
/// dependencies and virtual environments automatically.
pub fn uv_path() -> Result<PathBuf> {
    if let Ok(output) = Command::new("which").arg("uv").output()
        && output.status.success()
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    // Check common locations
    let mut common_paths: Vec<PathBuf> = vec![
        PathBuf::from("/usr/local/bin/uv"),
        PathBuf::from("/opt/homebrew/bin/uv"),
    ];

    // Cargo install location
    if let Ok(home) = std::env::var("HOME") {
        common_paths.push(PathBuf::from(home).join(".cargo/bin/uv"));
    }

    for path in common_paths {
        if path.exists() {
            return Ok(path);
        }
    }

    Err(anyhow!(
        "Could not find uv. Please install it with: curl -LsSf https://astral.sh/uv/install.sh | sh"
    ))
}

/// Checks if `uv` is available on the system.
pub fn uv_available() -> bool {
    uv_path().is_ok()
}

/// Discovers the path to the Python interpreter via `uv`.
///
/// Uses `uv python find` to locate the Python interpreter that `uv` would use.
/// Falls back to direct python3/python lookup if uv is not available.
pub fn python_path() -> Result<PathBuf> {
    // Try uv first
    if let Ok(uv) = uv_path() {
        if let Ok(output) = Command::new(&uv).args(["python", "find"]).output()
            && output.status.success()
        {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(PathBuf::from(path));
            }
        }
    }

    // Fallback to direct python lookup
    if let Ok(output) = Command::new("which").arg("python3").output()
        && output.status.success()
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    if let Ok(output) = Command::new("which").arg("python").output()
        && output.status.success()
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    // Check common locations
    let common_paths = [
        "/usr/bin/python3",
        "/usr/local/bin/python3",
        "/opt/homebrew/bin/python3",
    ];

    for path in common_paths {
        if Path::new(path).exists() {
            return Ok(PathBuf::from(path));
        }
    }

    Err(anyhow!(
        "Could not find Python interpreter. Please ensure uv or python3 is installed."
    ))
}

/// Returns the command and arguments to run a Python script.
///
/// Uses `uv run <script.py>` if available (preferred), otherwise falls back to direct python.
/// `uv run` is preferred because it:
/// - Automatically handles inline script dependencies
/// - Creates isolated environments
/// - Is significantly faster than pip
pub fn python_run_command(script_path: &Path) -> Result<(OsString, Vec<OsString>)> {
    if let Ok(uv) = uv_path() {
        // uv run <script.py> - this is the simplest form
        Ok((
            uv.into_os_string(),
            vec!["run".into(), script_path.as_os_str().to_os_string()],
        ))
    } else {
        // Fallback to direct python
        let python = python_path()?;
        Ok((
            python.into_os_string(),
            vec![script_path.as_os_str().to_os_string()],
        ))
    }
}

/// Returns the command and arguments to run a Python module.
///
/// Uses `uv run -- python -m <module>` if available, otherwise falls back to `python -m`.
/// This form doesn't inject extra dependencies.
pub fn python_module_command(module: &str, extra_args: &[&str]) -> Result<(OsString, Vec<OsString>)> {
    if let Ok(uv) = uv_path() {
        // uv run -- python -m <module> <args>
        let mut args: Vec<OsString> = vec![
            "run".into(),
            "--".into(),
            "python".into(),
            "-m".into(),
            module.into(),
        ];
        args.extend(extra_args.iter().map(|s| OsString::from(*s)));
        Ok((uv.into_os_string(), args))
    } else {
        let python = python_path()?;
        let mut args: Vec<OsString> = vec!["-m".into(), module.into()];
        args.extend(extra_args.iter().map(|s| OsString::from(*s)));
        Ok((python.into_os_string(), args))
    }
}

/// Returns the command and arguments to run a Python module with additional dependencies.
///
/// Uses `uv run --with <deps> -- python -m <module>` which injects the specified
/// packages for this run only. This is useful for running tools like pytest that
/// may not be installed in the project environment.
///
/// # Arguments
/// * `module` - The Python module to run (e.g., "pytest")
/// * `with_deps` - Dependencies to inject (e.g., &["pytest"])
/// * `extra_args` - Additional arguments to pass to the module
pub fn python_module_with_deps_command(
    module: &str,
    with_deps: &[&str],
    extra_args: &[&str],
) -> Result<(OsString, Vec<OsString>)> {
    if let Ok(uv) = uv_path() {
        // uv run --with <dep1> --with <dep2> -- python -m <module> <args>
        let mut args: Vec<OsString> = vec!["run".into()];
        
        // Add --with for each dependency
        for dep in with_deps {
            args.push("--with".into());
            args.push((*dep).into());
        }
        
        // Add separator and python command
        args.push("--".into());
        args.push("python".into());
        args.push("-m".into());
        args.push(module.into());
        
        // Add extra arguments
        args.extend(extra_args.iter().map(|s| OsString::from(*s)));
        
        Ok((uv.into_os_string(), args))
    } else {
        // Fallback to direct python (assumes deps are already installed)
        let python = python_path()?;
        let mut args: Vec<OsString> = vec!["-m".into(), module.into()];
        args.extend(extra_args.iter().map(|s| OsString::from(*s)));
        Ok((python.into_os_string(), args))
    }
}

/// Returns the command and arguments to install a package.
///
/// Uses `uv pip install` if available, otherwise falls back to pip.
pub fn pip_install_command(package: &str) -> Result<(OsString, Vec<OsString>)> {
    if let Ok(uv) = uv_path() {
        Ok((
            uv.into_os_string(),
            vec!["pip".into(), "install".into(), package.into()],
        ))
    } else {
        // Fallback to pip
        if let Ok(output) = Command::new("which").arg("pip3").output()
            && output.status.success()
        {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok((
                    PathBuf::from(path).into_os_string(),
                    vec!["install".into(), package.into()],
                ));
            }
        }

        if let Ok(output) = Command::new("which").arg("pip").output()
            && output.status.success()
        {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok((
                    PathBuf::from(path).into_os_string(),
                    vec!["install".into(), package.into()],
                ));
            }
        }

        Err(anyhow!(
            "Could not find pip. Please ensure uv or pip is installed."
        ))
    }
}

/// Discovers data files (.txt, .csv) in the project.
pub fn discover_data_files(paths: &ProjectPaths) -> Result<Vec<PathBuf>> {
    let mut data_files = Vec::new();

    // Find .txt files
    if let Ok(txt_files) = find_files("txt", 3, paths.data_dir()) {
        data_files.extend(txt_files);
    }

    // Find .csv files
    if let Ok(csv_files) = find_files("csv", 3, paths.data_dir()) {
        data_files.extend(csv_files);
    }

    // Find .json files
    if let Ok(json_files) = find_files("json", 3, paths.data_dir()) {
        data_files.extend(json_files);
    }

    Ok(data_files)
}

/// Discovers Python files in the source directory.
pub fn discover_python_files(paths: &ProjectPaths) -> Result<Vec<PathBuf>> {
    find_files("py", 10, paths.source_dir())
        .context("Failed to discover Python files in source directory")
}

/// Discovers test files in the test directory.
pub fn discover_test_files(paths: &ProjectPaths) -> Result<Vec<PathBuf>> {
    let mut test_files = Vec::new();

    // Look in test directory
    if paths.test_dir().exists()
        && let Ok(files) = find_files("py", 5, paths.test_dir())
    {
        test_files.extend(files);
    }

    // Also look for test_*.py in source directory
    if let Ok(files) = find_files("py", 3, paths.source_dir()) {
        for file in files {
            if let Some(name) = file.file_name() {
                let name = name.to_string_lossy();
                if name.starts_with("test_") || name.ends_with("_test.py") {
                    test_files.push(file);
                }
            }
        }
    }

    Ok(test_files)
}

/// Constructs the PYTHONPATH for the project.
pub fn python_path_env(paths: &ProjectPaths) -> OsString {
    let sep = paths.separator();
    let mut python_path = OsString::new();

    // Add source directory
    python_path.push(paths.source_dir());

    // Add root directory if different from source
    if paths.root_dir() != paths.source_dir() {
        python_path.push(sep);
        python_path.push(paths.root_dir());
    }

    python_path
}

/// Returns the Python version string if available.
///
/// Uses `uv run python --version` or falls back to direct python.
pub fn python_version() -> Result<String> {
    // Try uv first
    if let Ok(uv) = uv_path() {
        if let Ok(output) = Command::new(&uv)
            .args(["run", "--", "python", "--version"])
            .output()
            && output.status.success()
        {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !version.is_empty() {
                return Ok(version);
            }
        }
    }

    // Fallback to direct python
    let python = python_path()?;
    let output = Command::new(&python)
        .arg("--version")
        .output()
        .context("Failed to get Python version")?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if version.is_empty() {
            // Python 2 prints to stderr
            Ok(String::from_utf8_lossy(&output.stderr).trim().to_string())
        } else {
            Ok(version)
        }
    } else {
        Err(anyhow!("Failed to get Python version"))
    }
}
