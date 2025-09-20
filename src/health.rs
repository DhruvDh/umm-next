// TODO: make recommendations for the above

use anyhow::{Context, Result};
use futures::{future::try_join_all, stream::FuturesUnordered};
use tokio::{fs::OpenOptions, task::JoinError};
use walkdir::WalkDir;

use crate::{
    constants::{BUILD_DIR, LIB_DIR, ROOT_DIR, RUNTIME, SOURCE_DIR, TEST_DIR},
    java::{FileType, Project},
};

impl Project {
    /// Checks the project for common CodingRooms errors
    pub fn check_health(&self) -> Result<()> {
        tracing::info!("Checking Project Health...");
        let project = Project::new()?;

        let rt = RUNTIME.handle().clone();
        let _guard = rt.enter();

        let handle1 = rt.spawn(async {
            let files = WalkDir::new(ROOT_DIR.as_path())
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

        let handle2 = rt.spawn(async move {
            let files = project
                .files()
                .iter()
                .map(|file| {
                    let file = file.clone();
                    tokio::spawn(async move {
                        if file.package_name().is_none() {
                            tracing::warn!(
                                "File {}\n\tdoesn't belong to any package",
                                file.path().display()
                            );
                        } else {
                            let expected_path = if let FileType::Test = file.kind() {
                                TEST_DIR.join(file.package_name().unwrap())
                            } else {
                                SOURCE_DIR.join(file.package_name().unwrap())
                            };
                            if file.path().parent().unwrap_or(&ROOT_DIR) != expected_path.as_path()
                            {
                                tracing::warn!(
                                    "File {}\n\tis in the wrong directory.\n\t\tExpected: \
                                     {}\n\t\tFound: {}",
                                    file.path().display(),
                                    expected_path.display(),
                                    file.path().parent().unwrap_or(&ROOT_DIR).to_string_lossy()
                                );
                            }
                        }
                    })
                })
                .collect::<FuturesUnordered<_>>();
            try_join_all(files).await
        });

        rt.block_on(async {
            if BUILD_DIR.join(".vscode").exists() {
                tokio::fs::remove_dir_all(BUILD_DIR.join(".vscode").as_path())
                    .await
                    .with_context(|| {
                        format!("Could not delete {}", BUILD_DIR.join(".vscode").display())
                    })
                    .unwrap();
            }

            if BUILD_DIR.join(LIB_DIR.display().to_string()).exists() {
                tokio::fs::remove_dir_all(BUILD_DIR.join(LIB_DIR.display().to_string()).as_path())
                    .await
                    .with_context(|| {
                        format!(
                            "Could not delete {}",
                            BUILD_DIR.join(LIB_DIR.display().to_string()).display()
                        )
                    })
                    .unwrap();
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
