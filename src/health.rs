// TODO: make recommendations for the above

use std::sync::Arc;

use anyhow::{Context, Result};
use futures::{future::try_join_all, stream::FuturesUnordered};
use tokio::{fs::OpenOptions, task::JoinError};
use walkdir::WalkDir;

use crate::{
    config,
    java::{FileType, Project, ProjectPaths},
};

impl Project {
    /// Checks the project for common CodingRooms errors
    pub fn check_health(&self) -> Result<()> {
        tracing::info!("Checking Project Health...");
        let project = Project::new()?;
        let paths = Arc::new(ProjectPaths::default());

        let runtime = config::runtime();
        let rt = runtime.handle().clone();
        let _guard = rt.enter();

        let paths_for_walk = paths.clone();
        let handle1 = rt.spawn(async move {
            let root_dir = paths_for_walk.root_dir().to_path_buf();
            let files = WalkDir::new(root_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_path_buf())
                .map(|path| {
                    tokio::spawn(async move {
                        match tokio::fs::metadata(path.clone()).await {
                            Ok(m) => {
                                if m.len() == 0 {
                                    tracing::warn!("File {}\n\tis empty", &path.display())
                                }
                                if let Err(e) =
                                    OpenOptions::new().read(true).write(true).open(&path).await
                                {
                                    tracing::warn!(
                                        "File {}\n\tcould not be opened (read + write): {}",
                                        &path.display(),
                                        e
                                    )
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Could not read file {}: {}", path.display(), e)
                            }
                        };

                        if path.extension().unwrap_or_default() == "jar" {
                            let output = tokio::process::Command::new("zip")
                                .arg("-T")
                                .arg(&path)
                                .output()
                                .await
                                .unwrap_or_else(|_| {
                                    panic!("Could not run zip -T on {}", &path.display())
                                });

                            if !output.status.success() {
                                tracing::warn!(
                                    "File {}\n\tis not a valid zip file: {}",
                                    &path.display(),
                                    String::from_utf8_lossy(&output.stderr)
                                )
                            }
                        }
                    })
                })
                .collect::<FuturesUnordered<_>>();

            try_join_all(files).await
        });

        let paths_for_packages = paths.clone();
        let handle2 = rt.spawn(async move {
            let files = project
                .files()
                .iter()
                .map(|file| {
                    let file = file.clone();
                    let paths_for_packages = paths_for_packages.clone();
                    tokio::spawn(async move {
                        if file.package_name().is_none() {
                            tracing::warn!(
                                "File {}\n\tdoesn't belong to any package",
                                file.path().display()
                            );
                        } else {
                            let expected_path = if let FileType::Test = file.kind() {
                                paths_for_packages
                                    .test_dir()
                                    .join(file.package_name().unwrap())
                            } else {
                                paths_for_packages
                                    .source_dir()
                                    .join(file.package_name().unwrap())
                            };
                            if file
                                .path()
                                .parent()
                                .unwrap_or(paths_for_packages.root_dir())
                                != expected_path.as_path()
                            {
                                tracing::warn!(
                                    "File {}\n\tis in the wrong directory.\n\t\tExpected: \
                                     {}\n\t\tFound: {}",
                                    file.path().display(),
                                    expected_path.display(),
                                    file.path()
                                        .parent()
                                        .unwrap_or(paths_for_packages.root_dir())
                                        .to_string_lossy()
                                );
                            }
                        }
                    })
                })
                .collect::<FuturesUnordered<_>>();
            try_join_all(files).await
        });

        let paths_for_cleanup = paths.clone();
        rt.block_on(async move {
            let build_dir = paths_for_cleanup.build_dir().to_path_buf();
            if build_dir.join(".vscode").exists() {
                tokio::fs::remove_dir_all(build_dir.join(".vscode").as_path())
                    .await
                    .with_context(|| {
                        format!("Could not delete {}", build_dir.join(".vscode").display())
                    })
                    .unwrap();
            }

            if let Some(lib_folder_name) = paths_for_cleanup.lib_dir().file_name() {
                let build_lib_dir = build_dir.join(lib_folder_name);
                if build_lib_dir.exists() {
                    tokio::fs::remove_dir_all(build_lib_dir.as_path())
                        .await
                        .with_context(|| format!("Could not delete {}", build_lib_dir.display()))
                        .unwrap();
                }
            }
            let handles = FuturesUnordered::from_iter(vec![handle1, handle2]);
            try_join_all(handles).await
        })?
        .into_iter()
        .collect::<Result<Vec<Vec<()>>, JoinError>>()?;

        tracing::info!(
            "This is information an instructor can use to help you, please don't try to interpret \
             it yourself or make any changes to your submission based on it."
        );
        Ok(())
    }
}
