use anyhow::{Result, bail};
use futures::{future::join_all, stream::FuturesUnordered};
use serde::{Deserialize, Serialize};

use super::{file::File, paths::ProjectPaths};
use crate::{config, util::find_files};
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
    /// Initializes a Project by discovering Java files in the
    /// [struct@UMM_DIR] directory and preparing metadata for later operations.
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
        let mut lines = vec!["<project>".to_string()];

        for file in self.files.iter() {
            if file.proper_name().contains("Hidden") {
                continue;
            }
            lines.push(file.description());
        }

        lines.push("</project>".to_string());
        lines.join("\n")
    }
}
