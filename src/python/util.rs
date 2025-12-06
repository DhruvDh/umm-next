#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Python-specific utility functions for toolchain and filesystem operations.
//!
//! This module now treats `uv` as the primary execution policy engine: every
//! run is scoped to an explicit project root, working directory, virtual
//! environment location, and set of overlays. When `uv` is unavailable, we
//! gracefully fall back to the system Python interpreter while keeping
//! PYTHONPATH consistent.

use std::{
    collections::HashSet,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow};
use which::which;

use super::paths::ProjectPaths;
use crate::util::find_files;

/// Captures a fully constructed command invocation.
#[derive(Debug, Clone)]
pub struct CommandSpec {
    /// Program to execute.
    pub program: OsString,
    /// Arguments passed to the program.
    pub args:    Vec<OsString>,
    /// Environment variables to set for the invocation.
    pub env:     Vec<(OsString, OsString)>,
    /// Working directory for the command.
    pub cwd:     Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_uv_args_respect_policy_flags() {
        let paths = ProjectPaths::default();
        let ctx = UvRunContext::for_paths(&paths)
            .no_project(true)
            .frozen(true)
            .no_sync(true);

        let args: Vec<String> = ctx
            .base_uv_args()
            .into_iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();

        assert!(args.contains(&"--no-project".to_string()));
        assert!(args.contains(&"--frozen".to_string()));
        assert!(args.contains(&"--no-sync".to_string()));
        assert!(!args.contains(&"--project".to_string()));
        assert_eq!(args.first().map(String::as_str), Some("run"));
    }

    #[test]
    fn tool_args_use_tool_run() {
        let paths = ProjectPaths::default();
        let ctx = UvRunContext::for_paths(&paths);
        let args: Vec<String> = ctx
            .base_uv_tool_args()
            .into_iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();

        assert_eq!(args.first().map(String::as_str), Some("tool"));
        assert_eq!(args.get(1).map(String::as_str), Some("run"));
        assert!(args.contains(&"--directory".to_string()));
        assert!(args.contains(&"--no-active".to_string()));
        assert!(!args.contains(&"--project".to_string()));
    }

    #[test]
    fn base_env_clears_virtual_env() {
        let paths = ProjectPaths::default();
        let env = UvRunContext::for_paths(&paths).base_env();

        assert!(env.iter().any(|(k, v)| k == "VIRTUAL_ENV" && v.is_empty()));
        assert!(env.iter().any(|(k, v)| k == "UV_NO_ACTIVE" && v == "1"));
    }
}

/// Execution context for `uv run` (or a Python fallback) with explicit policy
/// for project scope, working directory, overlays, and environment isolation.
#[derive(Debug, Clone)]
pub struct UvRunContext {
    /// Root of the project for uv discovery and locking.
    project_root: PathBuf,
    /// Working directory for the spawned process.
    working_dir:  PathBuf,
    /// Location of the managed virtual environment.
    env_path:     PathBuf,
    /// Overlay packages injected for a single run (e.g., pytest).
    overlays:     Vec<String>,
    /// Whether to pass `--locked` to uv.
    locked:       bool,
    /// Whether to forbid project auto-discovery/install for this run.
    no_project:   bool,
    /// Whether to skip sync of the environment.
    no_sync:      bool,
    /// Whether to run strictly from the lockfile without resolving.
    frozen:       bool,
    /// Whether to disable uv config discovery.
    no_config:    bool,
    /// Whether to disable dotenv loading for the run.
    no_env_file:  bool,
    /// PYTHONPATH composed from source/test/root directories.
    pythonpath:   OsString,
}

impl Default for UvRunContext {
    fn default() -> Self {
        Self::for_paths(&ProjectPaths::default())
    }
}

impl UvRunContext {
    /// Construct a context from project paths with sensible defaults:
    /// - project + working dir: root
    /// - env path: `<root>/.umm/venv`
    /// - overlays: none
    /// - `--no-config` and `--no-env-file` enabled
    pub fn for_paths(paths: &ProjectPaths) -> Self {
        Self {
            project_root: paths.root_dir().to_path_buf(),
            working_dir:  paths.root_dir().to_path_buf(),
            env_path:     paths.umm_dir().join("venv"),
            overlays:     Vec::new(),
            locked:       false,
            no_project:   false,
            no_sync:      false,
            frozen:       false,
            no_config:    true,
            no_env_file:  true,
            pythonpath:   python_path_env(paths),
        }
    }

    /// Override the working directory.
    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = dir.into();
        self
    }

    /// Override the environment location `uv` should use.
    pub fn env_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.env_path = path.into();
        self
    }

    /// Add a single overlay dependency for this run (e.g., `pytest`).
    pub fn with_overlay(mut self, overlay: impl Into<String>) -> Self {
        self.overlays.push(overlay.into());
        self
    }

    /// Add multiple overlay dependencies.
    pub fn with_overlays<I, S>(mut self, overlays: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.overlays.extend(overlays.into_iter().map(Into::into));
        self
    }

    /// Require `uv run --locked` (defaults to false to avoid breaking projects
    /// without a lock file).
    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Toggle whether uv should avoid installing/detecting the current project.
    pub fn no_project(mut self, no_project: bool) -> Self {
        self.no_project = no_project;
        self
    }

    /// Toggle whether uv should skip syncing environments.
    pub fn no_sync(mut self, no_sync: bool) -> Self {
        self.no_sync = no_sync;
        self
    }

    /// Toggle whether uv should run strictly from the lockfile (no resolution).
    pub fn frozen(mut self, frozen: bool) -> Self {
        self.frozen = frozen;
        self
    }

    /// Toggle whether `uv` should ignore user/system config.
    pub fn no_config(mut self, no_config: bool) -> Self {
        self.no_config = no_config;
        self
    }

    /// Toggle whether `uv` should ignore .env files.
    pub fn no_env_file(mut self, no_env_file: bool) -> Self {
        self.no_env_file = no_env_file;
        self
    }

    /// Build a `uv run` invocation that executes a script by path.
    pub fn run_script_command(&self, script_path: &Path) -> Result<CommandSpec> {
        let uv = uv_path()?;
        let mut args = self.base_uv_args();
        args.push(script_path.as_os_str().to_os_string());

        Ok(CommandSpec {
            program: uv.into_os_string(),
            args,
            env: self.base_env(),
            cwd: Some(self.working_dir.clone()),
        })
    }

    /// Build an invocation that executes a module (`uv run -m <module>`),
    /// preserving overlays and environment policy.
    pub fn run_module_command(&self, module: &str, extra_args: &[&str]) -> Result<CommandSpec> {
        let uv = uv_path()?;
        let mut args = self.base_uv_args();
        args.push("-m".into());
        args.push(module.into());
        args.extend(extra_args.iter().map(|s| OsString::from(*s)));

        Ok(CommandSpec {
            program: uv.into_os_string(),
            args,
            env: self.base_env(),
            cwd: Some(self.working_dir.clone()),
        })
    }

    /// Build an invocation that runs a CLI tool (e.g., ruff/black) in tool mode
    /// (`uv tool run`) so tool environments stay isolated from the project.
    pub fn run_tool_command(&self, tool: &str, extra_args: &[&str]) -> Result<CommandSpec> {
        let ctx = self.clone().no_project(true);
        let uv = uv_path()?;
        let mut args = ctx.base_uv_tool_args();
        args.push(tool.into());
        args.extend(extra_args.iter().map(|s| OsString::from(*s)));

        Ok(CommandSpec {
            program: uv.into_os_string(),
            args,
            env: ctx.base_env(),
            cwd: Some(ctx.working_dir),
        })
    }

    /// Replace the PYTHONPATH used for execution.
    pub fn with_pythonpath(mut self, pythonpath: OsString) -> Self {
        self.pythonpath = pythonpath;
        self
    }

    /// Base environment variables applied to every invocation.
    fn base_env(&self) -> Vec<(OsString, OsString)> {
        let mut env = vec![
            (OsString::from("PYTHONPATH"), self.pythonpath.clone()),
            (OsString::from("UV_PROJECT_ENVIRONMENT"), self.env_path.clone().into_os_string()),
            // Prevent uv from preferring an externally active virtualenv.
            (OsString::from("UV_NO_ACTIVE"), OsString::from("1")),
            (OsString::from("VIRTUAL_ENV"), OsString::from("")),
        ];

        if self.no_env_file {
            env.push((OsString::from("UV_NO_ENV_FILE"), OsString::from("1")));
        }
        if self.no_config {
            env.push((OsString::from("UV_NO_CONFIG"), OsString::from("1")));
        }

        env
    }

    /// Shared uv arguments representing project scope and policy toggles.
    fn base_uv_args(&self) -> Vec<OsString> {
        let mut args: Vec<OsString> = vec!["run".into()];

        if !self.no_project {
            args.push("--project".into());
            args.push(self.project_root.clone().into());
        }

        args.push("--directory".into());
        args.push(self.working_dir.clone().into());

        if self.locked {
            args.push("--locked".into());
        }
        if self.frozen {
            args.push("--frozen".into());
        }
        if self.no_sync {
            args.push("--no-sync".into());
        }
        if self.no_config {
            args.push("--no-config".into());
        }
        if self.no_env_file {
            args.push("--no-env-file".into());
        }
        if self.no_project {
            args.push("--no-project".into());
        }
        // Ensure uv ignores any active virtualenv.
        args.push("--no-active".into());

        for overlay in &self.overlays {
            args.push("--with".into());
            args.push(overlay.clone().into());
        }

        args
    }

    /// Global uv flags for `uv tool run` invocations.
    fn base_uv_tool_args(&self) -> Vec<OsString> {
        let mut args: Vec<OsString> = vec!["tool".into(), "run".into()];

        args.push("--directory".into());
        args.push(self.working_dir.clone().into());

        if self.locked {
            args.push("--locked".into());
        }
        if self.frozen {
            args.push("--frozen".into());
        }
        if self.no_sync {
            args.push("--no-sync".into());
        }
        if self.no_config {
            args.push("--no-config".into());
        }
        if self.no_env_file {
            args.push("--no-env-file".into());
        }
        args.push("--no-active".into());

        args
    }
}

/// Discovers the path to `uv` using the system PATH.
pub fn uv_path() -> Result<PathBuf> {
    which("uv").context(
        "Could not find uv. Please install it with: curl -LsSf https://astral.sh/uv/install.sh | \
         sh",
    )
}

/// Checks if `uv` is available on the system.
pub fn uv_available() -> bool {
    uv_path().is_ok()
}

/// Discovers the path to the Python interpreter (preferring `python3`).
pub fn python_path() -> Result<PathBuf> {
    if let Ok(path) = which("python3") {
        return Ok(path);
    }

    if let Ok(path) = which("python") {
        return Ok(path);
    }

    Err(anyhow!(
        "Could not find Python interpreter. Please ensure uv or python3 is installed."
    ))
}

/// Constructs the PYTHONPATH for the project (source, tests, then root).
pub fn python_path_env(paths: &ProjectPaths) -> OsString {
    let mut seen = HashSet::new();
    let mut parts: Vec<OsString> = Vec::new();
    let sep = OsString::from(paths.separator());

    let mut push_unique = |p: &Path| {
        if seen.insert(p.to_path_buf()) {
            parts.push(p.as_os_str().to_os_string());
        }
    };

    push_unique(paths.source_dir());
    if paths.test_dir() != paths.source_dir() {
        push_unique(paths.test_dir());
    }
    if paths.root_dir() != paths.source_dir() && paths.root_dir() != paths.test_dir() {
        push_unique(paths.root_dir());
    }

    let mut out = OsString::new();
    for (idx, part) in parts.iter().enumerate() {
        if idx > 0 {
            out.push(&sep);
        }
        out.push(part);
    }

    out
}

/// Discovers data files (.txt, .csv, .json) in the project, skipping common
/// virtualenv/cache directories.
pub fn discover_data_files(paths: &ProjectPaths) -> Result<Vec<PathBuf>> {
    let mut data_files = Vec::new();

    for ext in ["txt", "csv", "json"] {
        if let Ok(files) = find_files(ext, 3, paths.data_dir()) {
            data_files.extend(files.into_iter().filter(|p| !is_ignored_path(p)));
        }
    }

    Ok(data_files)
}

/// Discovers Python files in the source directory, ignoring virtualenv and
/// cache directories.
pub fn discover_python_files(paths: &ProjectPaths) -> Result<Vec<PathBuf>> {
    let files = find_files("py", 10, paths.source_dir())
        .context("Failed to discover Python files in source directory")?;

    Ok(files.into_iter().filter(|p| !is_ignored_path(p)).collect())
}

/// Discovers test files, preferring `tests/` and also picking up `test_*.py`
/// under the source tree, while skipping ignored directories.
pub fn discover_test_files(paths: &ProjectPaths) -> Result<Vec<PathBuf>> {
    let mut test_files = Vec::new();

    if paths.test_dir().exists()
        && let Ok(files) = find_files("py", 5, paths.test_dir())
    {
        test_files.extend(files.into_iter().filter(|p| !is_ignored_path(p)));
    }

    if let Ok(files) = find_files("py", 3, paths.source_dir()) {
        for file in files {
            if is_ignored_path(&file) {
                continue;
            }
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

/// Returns the Python version string if available.
pub fn python_version() -> Result<String> {
    if let Ok(uv) = uv_path()
        && let Ok(output) = Command::new(&uv)
            .args(["python", "find", "--show-version"])
            .output()
        && output.status.success()
    {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() {
            return Ok(version);
        }
    }

    // Fallback to direct python --version
    let python = python_path()?;
    let output = Command::new(&python)
        .arg("--version")
        .output()
        .context("Failed to get Python version")?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if version.is_empty() {
            Ok(String::from_utf8_lossy(&output.stderr).trim().to_string())
        } else {
            Ok(version)
        }
    } else {
        Err(anyhow!("Failed to get Python version"))
    }
}

/// Returns true when the path resides inside directories we intentionally skip
/// during discovery (virtualenvs, caches, VCS metadata, etc.).
fn is_ignored_path(path: &Path) -> bool {
    path.components().any(|c| {
        if let Some(s) = c.as_os_str().to_str() {
            matches!(
                s,
                ".venv"
                    | "venv"
                    | ".git"
                    | "__pycache__"
                    | ".tox"
                    | ".pytest_cache"
                    | "node_modules"
                    | ".umm"
            )
        } else {
            false
        }
    })
}

/// Convenience: build a ruff lint command scoped to the project, installing
/// ruff as an overlay for this run if it is not already present.
pub fn ruff_lint_command(paths: &ProjectPaths, targets: &[&str]) -> Result<CommandSpec> {
    let ctx = UvRunContext::for_paths(paths);
    ctx.run_tool_command("ruff", targets)
}

/// Convenience: build a black format command scoped to the project, installing
/// black as an overlay for this run if it is not already present.
pub fn black_format_command(paths: &ProjectPaths, targets: &[&str]) -> Result<CommandSpec> {
    let ctx = UvRunContext::for_paths(paths);
    ctx.run_tool_command("black", targets)
}
