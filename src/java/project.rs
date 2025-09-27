use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use async_openai::types::{ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs};
use futures::{future::join_all, stream::FuturesUnordered};
use serde::{Deserialize, Serialize};

use super::{file::File, paths::ProjectPaths};
use crate::{
    config,
    java::grade::{
        LineRef,
        context::{build_active_retrieval_context, build_heuristic_context},
    },
    retrieval::{HeuristicConfig, RetrievalFormatter},
    util::find_files,
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
    /// Initializes a Project by discovering Java files in the
    /// [struct@UMM_DIR] directory and preparing metadata for later operations.
    pub fn new() -> Result<Self> {
        Self::from_paths(ProjectPaths::default())
    }

    /// Initializes a project rooted at the provided directory path.
    pub fn from_root<P: Into<PathBuf>>(root: P) -> Result<Self> {
        Self::from_paths(ProjectPaths::new(root.into()))
    }

    /// Core implementation that discovers files for the provided paths.
    fn from_paths(paths: ProjectPaths) -> Result<Self> {
        let mut files = vec![];
        let mut names = vec![];

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

        Ok(Self {
            files,
            names,
            paths,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_paths_for_tests(paths: ProjectPaths) -> Result<Self> {
        Self::from_paths(paths)
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

impl RetrievalFormatter for Project {
    fn language(&self) -> &'static str {
        "java"
    }

    fn full_codebase(&self) -> Result<Vec<ChatCompletionRequestMessage>> {
        let mut blocks = Vec::new();
        for file in self.files.iter() {
            let language = "java";
            let header = format!(
                "<file name=\"{}\" path=\"{}\" language=\"{}\">",
                file.proper_name(),
                file.path().display(),
                language
            );
            let mut content = vec![header];
            content.push(String::from("```java"));
            content.push(file.code().to_string());
            content.push(String::from("```"));
            content.push(String::from("</file>"));

            blocks.push(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(content.join("\n"))
                    .name("Instructor".to_string())
                    .build()
                    .context("Failed to build full-codebase message")?
                    .into(),
            );
        }

        Ok(blocks)
    }

    fn heuristic_context(
        &self,
        line_refs: Vec<LineRef>,
        cfg: HeuristicConfig,
    ) -> Result<ChatCompletionRequestMessage> {
        build_heuristic_context(line_refs, self.clone(), cfg)
    }

    fn active_retrieval(&self, grader_output: String) -> Result<ChatCompletionRequestMessage> {
        build_active_retrieval_context(self, grader_output)
    }
}
