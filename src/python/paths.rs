#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Python-specific workspace path configuration.

use std::path::{Path, PathBuf};

use bon::builder;
use serde::{Deserialize, Serialize};

/// Represents standard workspace paths for a Python project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPaths {
    /// Root directory of the project workspace.
    root_dir:   PathBuf,
    /// Source directory containing production code.
    source_dir: PathBuf,
    /// Test directory containing test files.
    test_dir:   PathBuf,
    /// Virtual environment directory.
    venv_dir:   PathBuf,
    /// Data files directory (.txt, .csv, etc.).
    data_dir:   PathBuf,
    /// Directory for grader reports and artifacts.
    report_dir: PathBuf,
    /// `.umm/` metadata directory maintained by the tool.
    umm_dir:    PathBuf,
}

impl ProjectPaths {
    /// Creates a new set of workspace paths rooted at `root_dir`.
    pub fn new(root_dir: PathBuf) -> Self {
        Self::build_with_defaults(root_dir, None, None, None, None, None, None)
    }

    /// Returns the platform specific separator character for Python paths.
    pub fn separator(&self) -> &'static str {
        if cfg!(windows) { ";" } else { ":" }
    }

    /// Root directory for the project.
    pub fn root_dir(&self) -> &Path {
        self.root_dir.as_path()
    }

    /// Construct paths from optional overrides.
    pub fn from_parts(
        root_dir: PathBuf,
        source_dir: Option<PathBuf>,
        test_dir: Option<PathBuf>,
        venv_dir: Option<PathBuf>,
        data_dir: Option<PathBuf>,
        report_dir: Option<PathBuf>,
        umm_dir: Option<PathBuf>,
    ) -> Self {
        Self::build_with_defaults(
            root_dir, source_dir, test_dir, venv_dir, data_dir, report_dir, umm_dir,
        )
    }

    /// Source directory for the project.
    pub fn source_dir(&self) -> &Path {
        self.source_dir.as_path()
    }

    /// Test directory for the project.
    pub fn test_dir(&self) -> &Path {
        self.test_dir.as_path()
    }

    /// Virtual environment directory for the project.
    pub fn venv_dir(&self) -> &Path {
        self.venv_dir.as_path()
    }

    /// Data files directory for the project.
    pub fn data_dir(&self) -> &Path {
        self.data_dir.as_path()
    }

    /// Directory for grader reports.
    pub fn report_dir(&self) -> &Path {
        self.report_dir.as_path()
    }

    /// Directory for umm artifacts.
    pub fn umm_dir(&self) -> &Path {
        self.umm_dir.as_path()
    }

    /// Returns a copy of these paths with a different data directory.
    pub fn with_data_dir(mut self, data_dir: impl Into<PathBuf>) -> Self {
        self.data_dir = data_dir.into();
        self
    }
}

impl Default for ProjectPaths {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

impl ProjectPaths {
    /// Centralized constructor that applies standard defaults when overrides
    /// are absent.
    fn build_with_defaults(
        root_dir: PathBuf,
        source_dir: Option<PathBuf>,
        test_dir: Option<PathBuf>,
        venv_dir: Option<PathBuf>,
        data_dir: Option<PathBuf>,
        report_dir: Option<PathBuf>,
        umm_dir: Option<PathBuf>,
    ) -> Self {
        // Python projects often have source at root or in src/
        let source_dir = source_dir.unwrap_or_else(|| root_dir.clone());
        // Common test directory names
        let test_dir = test_dir.unwrap_or_else(|| {
            let tests = root_dir.join("tests");
            if tests.exists() {
                tests
            } else {
                root_dir.join("test")
            }
        });
        // Virtual environment locations
        let venv_dir = venv_dir.unwrap_or_else(|| {
            let venv = root_dir.join(".venv");
            if venv.exists() {
                venv
            } else {
                root_dir.join("venv")
            }
        });
        let data_dir = data_dir.unwrap_or_else(|| root_dir.clone());
        let umm_dir = umm_dir.unwrap_or_else(|| root_dir.join(".umm"));
        let report_dir = report_dir.unwrap_or_else(|| umm_dir.join("reports"));

        Self {
            root_dir,
            source_dir,
            test_dir,
            venv_dir,
            data_dir,
            report_dir,
            umm_dir,
        }
    }
}

/// Builder-friendly constructor for `ProjectPaths` with optional overrides.
#[builder(finish_fn = build)]
pub fn project_paths(
    #[builder(into)] root_dir: PathBuf,
    source_dir: Option<PathBuf>,
    test_dir: Option<PathBuf>,
    venv_dir: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    report_dir: Option<PathBuf>,
    umm_dir: Option<PathBuf>,
) -> ProjectPaths {
    ProjectPaths::build_with_defaults(
        root_dir, source_dir, test_dir, venv_dir, data_dir, report_dir, umm_dir,
    )
}
