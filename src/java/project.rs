use anyhow::{Result, bail};
use futures::{future::join_all, stream::FuturesUnordered};
use serde::{Deserialize, Serialize};

use super::{file::File, paths::ProjectPaths};
use crate::{
    config,
    constants::JUNIT_PLATFORM,
    util::{download, find_files},
};
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
/// Struct representing a Java project.
/// Any index `i` in any collection in this struct always refers to the same
/// JavaFile.
pub struct Project {
    /// Collection of java files in this project
    files: Vec<File>,
    /// Names of java files in this project.
    names: Vec<String>,
    /// Workspace paths associated with this project
    paths: ProjectPaths,
}

impl Project {
    /// Initializes a Project, by discovering java files in the
    /// [struct@UMM_DIR] directory. Also downloads some `jar`
    /// files required for unit testing and mutation testing.
    pub fn new() -> Result<Self> {
        let mut files = vec![];
        let mut names = vec![];
        let paths = ProjectPaths::default();

        let runtime = config::runtime();
        let rt = runtime.handle().clone();
        let paths_for_search = paths.clone();
        let rt_for_spawn = rt.clone();

        let results = rt.block_on(async move {
            let found_files = match find_files("java", 15, paths_for_search.root_dir()) {
                Ok(f) => f,
                Err(e) => panic!("Could not find java files: {e}"),
            };

            let handles = FuturesUnordered::new();

            for path in found_files {
                let file_paths = paths_for_search.clone();
                let handle_clone = rt_for_spawn.clone();
                handles.push(handle_clone.spawn_blocking(move || File::new(path, file_paths)));
            }

            join_all(handles).await
        });

        for result in results {
            let file = result??;
            names.push(file.proper_name());
            files.push(file);
        }

        let proj = Self {
            files,
            names,
            paths: paths.clone(),
        };

        let proj_clone = proj.clone();
        let _guard = rt.enter();
        rt.block_on(async move { proj_clone.download_libraries_if_needed().await })?;

        Ok(proj)
    }

    /// Attempts to identify the correct file from the project from a partial or
    /// fully formed name as expected by a java compiler.
    ///
    /// Returns a reference to the identified file, if any.
    ///
    /// * `name`: partial/fully formed name of the Java file to look for.
    pub fn identify(&self, name: &str) -> Result<File> {
        if let Some(i) = self.names.iter().position(|n| n == name) {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files.iter().position(|n| n.file_name() == name) {
            Ok(self.files[i].clone())
        } else if let Some(i) = self
            .files
            .iter()
            .position(|n| n.file_name().trim_end_matches(".java") == name)
        {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files.iter().position(|n| n.simple_name() == name) {
            Ok(self.files[i].clone())
        } else if let Some(i) = self
            .files
            .iter()
            .position(|n| n.path().display().to_string() == name)
        {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files.iter().position(|n| n.proper_name() == name) {
            Ok(self.files[i].clone())
        } else {
            bail!("Could not find {} in the project", name)
        }
    }

    /// Returns true if project contains a file with the given name.
    pub fn contains(&self, name: &str) -> bool {
        self.identify(name).is_ok()
    }

    /// Returns the workspace paths associated with this project.
    pub fn paths(&self) -> &ProjectPaths {
        &self.paths
    }

    /// Downloads certain libraries like JUnit if found in imports.
    /// times out after 20 seconds.
    pub async fn download_libraries_if_needed(&self) -> Result<()> {
        let need_junit = 'outer: {
            for file in self.files.iter() {
                if let Some(imports) = file.imports() {
                    for import in imports {
                        if let Some(path) = import.get("path")
                            && path.starts_with("org.junit")
                        {
                            break 'outer true;
                        }
                    }
                }
            }
            false
        };

        if need_junit {
            let lib_dir = self.paths.lib_dir().to_path_buf();

            if !lib_dir.is_dir() {
                std::fs::create_dir(lib_dir.as_path()).unwrap();
            }

            let handle1 = {
                let lib_dir = lib_dir.clone();
                tokio::spawn(async move {
                    download(
                    "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/junit-platform-console-standalone-1.10.2.jar",
                    &lib_dir.join(JUNIT_PLATFORM),
                false
                        )
                        .await
                })
            };

            let handle2 = {
                let lib_dir = lib_dir.clone();
                tokio::spawn(async move {
                    download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/junit-4.13.2.jar",
                        &lib_dir.join("junit-4.13.2.jar"),
                        false,
                    )
                    .await
                })
            };

            let handle3 = {
                let lib_dir = lib_dir.clone();
                tokio::spawn(async move {
                    download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-1.16.1.jar",
                        &lib_dir.join("pitest.jar"),
                        false,
                    )
                    .await
                })
            };

            let handle4 = {
                let lib_dir = lib_dir.clone();
                tokio::spawn(async move {
                    download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-command-line-1.16.1.jar",
                        &lib_dir.join("pitest-command-line.jar"),
                        false,
                    )
                    .await
                })
            };

            let handle5 = {
                let lib_dir = lib_dir.clone();
                tokio::spawn(async move {
                    download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-entry-1.16.1.jar",
                        &lib_dir.join("pitest-entry.jar"),
                        false,
                    )
                    .await
                })
            };

            let handle6 = {
                let lib_dir = lib_dir.clone();
                tokio::spawn(async move {
                    download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-junit5-plugin-1.2.1.jar",
                        &lib_dir.join("pitest-junit5-plugin.jar"),
                        false,
                    )
                    .await
                })
            };

            let handle7 = {
                let lib_dir = lib_dir.clone();
                tokio::spawn(async move {
                    download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/commons-text-1.12.0.jar",
                        &lib_dir.join("commons-text-1.12.0.jar"),
                        false,
                    )
                    .await
                })
            };

            let handle8 = tokio::spawn(async move {
                download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/commons-lang3-3.14.0.jar",
                        &lib_dir.join("commons-lang3-3.14.0.jar"),
                        false,
                    )
                    .await
            });

            let handles = FuturesUnordered::from_iter([
                handle1, handle2, handle3, handle4, handle5, handle6, handle7, handle8,
            ]);

            futures::future::try_join_all(handles).await?;
        }
        Ok(())
    }

    /// Get a reference to the project's files.
    pub fn files(&self) -> &[File] {
        self.files.as_ref()
    }

    /// Prints project struct as a json
    pub fn info(&self) -> Result<()> {
        println!("{}", serde_json::to_string(&self)?);
        Ok(())
    }

    /// Returns a short summary of the project, it's files, their fields and
    /// methods.
    pub fn describe(&self) -> String {
        let mut result = String::new();
        result.push_str(
            "> What follows is a summary of the student's submission's files, their fields and \
             methods generated via treesitter queries.\n\n",
        );

        for f in self.files.iter() {
            if f.proper_name().contains("Hidden") {
                continue;
            }
            result.push_str(f.description().as_str());
            result.push_str("\n\n");
        }

        result
    }
}
