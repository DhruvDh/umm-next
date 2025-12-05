#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::Stdio,
    time::Duration,
};

use anyhow::{Context, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    time::timeout,
};

/// Drop guard that terminates a spawned child process if callers forget to
/// await it.
struct ChildDropGuard(Option<Child>);

impl ChildDropGuard {
    /// Wraps the provided child process with the drop guard.
    fn new(child: Child) -> Self {
        Self(Some(child))
    }

    /// Returns a mutable reference to the underlying child process.
    fn child_mut(&mut self) -> anyhow::Result<&mut Child> {
        self.0
            .as_mut()
            .context("child process already taken from guard")
    }

    /// Prevents the guard from killing the process on drop.
    fn disarm(mut self) {
        self.0 = None;
    }
}

impl Drop for ChildDropGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.start_kill();
        }
    }
}

/// Captured result of a finished subprocess.
#[derive(Debug)]
pub struct Collected {
    /// Exit status returned by the process.
    pub status: std::process::ExitStatus,
    /// Contents written to stdout.
    pub stdout: Vec<u8>,
    /// Contents written to stderr.
    pub stderr: Vec<u8>,
}

/// Describes how stdin should be wired for the spawned process.
#[derive(Debug)]
pub enum StdinSource {
    /// Inherit the parent's stdin.
    Inherit,
    /// Attach nothing to stdin.
    Null,
    /// Write the provided bytes, then close stdin.
    Bytes(Vec<u8>),
}

/// Spawns a command, optionally feeds stdin, and collects stdout/stderr.
pub async fn run_collect(
    program: impl AsRef<OsStr>,
    args: &[OsString],
    stdin: StdinSource,
    cwd: Option<&Path>,
    env: &[(OsString, OsString)],
    deadline: Option<Duration>,
) -> Result<Collected> {
    let mut cmd = Command::new(program);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    match &stdin {
        StdinSource::Inherit => {
            cmd.stdin(Stdio::inherit());
        }
        StdinSource::Null => {
            cmd.stdin(Stdio::null());
        }
        StdinSource::Bytes(_) => {
            cmd.stdin(Stdio::piped());
        }
    }

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    for (key, value) in env {
        cmd.env(key, value);
    }

    let mut guard = ChildDropGuard::new(cmd.spawn().context("failed to spawn process")?);
    let stdin_payload = match stdin {
        StdinSource::Bytes(bytes) => Some(bytes),
        StdinSource::Inherit | StdinSource::Null => None,
    };

    if let Some(bytes) = stdin_payload
        && let Some(mut handle) = guard.child_mut()?.stdin.take()
    {
        tokio::spawn(async move {
            if !bytes.is_empty() {
                let _ = handle.write_all(&bytes).await;
            }
            let _ = handle.shutdown().await;
        });
    }

    let stdout = guard
        .child_mut()?
        .stdout
        .take()
        .context("missing stdout pipe")?;
    let stderr = guard
        .child_mut()?
        .stderr
        .take()
        .context("missing stderr pipe")?;

    let out_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        let mut buf = Vec::new();
        reader
            .read_to_end(&mut buf)
            .await
            .context("failed to read stdout")?;
        Ok::<Vec<u8>, anyhow::Error>(buf)
    });

    let err_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut buf = Vec::new();
        reader
            .read_to_end(&mut buf)
            .await
            .context("failed to read stderr")?;
        Ok::<Vec<u8>, anyhow::Error>(buf)
    });

    let wait_future = async move {
        let mut guard = guard;
        let status = guard
            .child_mut()?
            .wait()
            .await
            .context("failed to wait on process")?;
        let stdout = out_task.await.context("stdout task join error")??;
        let stderr = err_task.await.context("stderr task join error")??;
        guard.disarm();
        Ok(Collected {
            status,
            stdout,
            stderr,
        })
    };

    match deadline {
        Some(limit) => timeout(limit, wait_future)
            .await
            .context("subprocess timed out")?,
        None => wait_future.await,
    }
}
