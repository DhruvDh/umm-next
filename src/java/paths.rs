#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::path::{Path, PathBuf};

use bon::builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Represents standard workspace paths for a Java project.
pub struct ProjectPaths {
    /// Root directory of the project workspace.
    root_dir:   PathBuf,
    /// `src/` directory containing production sources.
    source_dir: PathBuf,
    /// `target/` build output directory.
    build_dir:  PathBuf,
    /// `test/` directory containing student tests.
    test_dir:   PathBuf,
    /// `lib/` directory holding downloaded jars.
    lib_dir:    PathBuf,
    /// `.umm/` metadata directory maintained by the tool.
    umm_dir:    PathBuf,
    /// `test_reports/` directory where graders write reports (e.g., PIT).
    report_dir: PathBuf,
}

impl ProjectPaths {
    /// Creates a new set of workspace paths rooted at `root_dir`.
    pub fn new(root_dir: PathBuf) -> Self {
        Self::build_with_defaults(root_dir, None, None, None, None, None, None)
    }

    /// Returns the platform specific separator character for javac paths.
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
        build_dir: Option<PathBuf>,
        test_dir: Option<PathBuf>,
        lib_dir: Option<PathBuf>,
        umm_dir: Option<PathBuf>,
        report_dir: Option<PathBuf>,
    ) -> Self {
        Self::build_with_defaults(
            root_dir, source_dir, build_dir, test_dir, lib_dir, umm_dir, report_dir,
        )
    }

    /// Source directory for the project.
    pub fn source_dir(&self) -> &Path {
        self.source_dir.as_path()
    }

    /// Build directory for the project.
    pub fn build_dir(&self) -> &Path {
        self.build_dir.as_path()
    }

    /// Test directory for the project.
    pub fn test_dir(&self) -> &Path {
        self.test_dir.as_path()
    }

    /// Library directory for the project.
    pub fn lib_dir(&self) -> &Path {
        self.lib_dir.as_path()
    }

    /// Returns a copy of these paths with a different `lib` directory.
    pub fn with_lib_dir(mut self, lib_dir: impl Into<PathBuf>) -> Self {
        self.lib_dir = lib_dir.into();
        self
    }

    /// Directory for umm artefacts.
    pub fn umm_dir(&self) -> &Path {
        self.umm_dir.as_path()
    }

    /// Directory for grader reports (e.g., PIT mutation reports).
    pub fn report_dir(&self) -> &Path {
        self.report_dir.as_path()
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
        build_dir: Option<PathBuf>,
        test_dir: Option<PathBuf>,
        lib_dir: Option<PathBuf>,
        umm_dir: Option<PathBuf>,
        report_dir: Option<PathBuf>,
    ) -> Self {
        let source_dir = source_dir.unwrap_or_else(|| root_dir.join("src"));
        let build_dir = build_dir.unwrap_or_else(|| root_dir.join("target"));
        let test_dir = test_dir.unwrap_or_else(|| root_dir.join("test"));
        let lib_dir = lib_dir.unwrap_or_else(|| root_dir.join("lib"));
        let umm_dir = umm_dir.unwrap_or_else(|| root_dir.join(".umm"));
        let report_dir = report_dir.unwrap_or_else(|| umm_dir.join("test_reports"));

        Self {
            root_dir,
            source_dir,
            build_dir,
            test_dir,
            lib_dir,
            umm_dir,
            report_dir,
        }
    }
}

/// Builder-friendly constructor for `ProjectPaths` with optional overrides.
#[builder(finish_fn = build)]
pub fn project_paths(
    #[builder(into)] root_dir: PathBuf,
    source_dir: Option<PathBuf>,
    build_dir: Option<PathBuf>,
    test_dir: Option<PathBuf>,
    lib_dir: Option<PathBuf>,
    umm_dir: Option<PathBuf>,
    report_dir: Option<PathBuf>,
) -> ProjectPaths {
    ProjectPaths::build_with_defaults(
        root_dir, source_dir, build_dir, test_dir, lib_dir, umm_dir, report_dir,
    )
}
