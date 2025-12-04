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
}

impl ProjectPaths {
    /// Creates a new set of workspace paths rooted at `root_dir`.
    pub fn new(root_dir: PathBuf) -> Self {
        let source_dir = root_dir.join("src");
        let build_dir = root_dir.join("target");
        let test_dir = root_dir.join("test");
        let lib_dir = root_dir.join("lib");
        let umm_dir = root_dir.join(".umm");
        // TODO: When Project introduces a typed builder, surface hooks to override
        // these defaults.

        Self {
            root_dir,
            source_dir,
            build_dir,
            test_dir,
            lib_dir,
            umm_dir,
        }
    }

    /// Returns the platform specific separator character for javac paths.
    pub fn separator(&self) -> &'static str {
        if cfg!(windows) { ";" } else { ":" }
    }

    /// Root directory for the project.
    pub fn root_dir(&self) -> &Path {
        self.root_dir.as_path()
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

    /// Directory for umm artefacts.
    pub fn umm_dir(&self) -> &Path {
        self.umm_dir.as_path()
    }
}

impl Default for ProjectPaths {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
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
) -> ProjectPaths {
    let source_dir = source_dir.unwrap_or_else(|| root_dir.join("src"));
    let build_dir = build_dir.unwrap_or_else(|| root_dir.join("target"));
    let test_dir = test_dir.unwrap_or_else(|| root_dir.join("test"));
    let lib_dir = lib_dir.unwrap_or_else(|| root_dir.join("lib"));
    let umm_dir = umm_dir.unwrap_or_else(|| root_dir.join(".umm"));

    ProjectPaths {
        root_dir,
        source_dir,
        build_dir,
        test_dir,
        lib_dir,
        umm_dir,
    }
}
