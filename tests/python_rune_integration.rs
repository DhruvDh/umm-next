//! Integration tests for Python Rune scripting.

use std::path::PathBuf;

use assert_cmd::cargo::cargo_bin_cmd;

#[path = "rune_support.rs"]
mod rune_support;

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn rune_script(name: &str) -> PathBuf {
    fixtures_root().join("rune").join(name)
}

fn python_project_dir(name: &str) -> PathBuf {
    fixtures_root().join("python").join(name)
}

fn run_python_script(script: &str, workdir: &str) -> (String, String) {
    let mut cmd = cargo_bin_cmd!("umm");
    let script_path = rune_script(script);
    cmd.current_dir(python_project_dir(workdir))
        .env("CLICOLOR", "0")
        .arg("python")
        .arg("grade")
        .arg(script_path);

    let assert = cmd.assert().success();
    let output = assert.get_output().clone();
    let stdout =
        rune_support::normalize_rune_output(String::from_utf8_lossy(&output.stdout).to_string());
    let stderr =
        rune_support::normalize_rune_output(String::from_utf8_lossy(&output.stderr).to_string());
    (stdout, stderr)
}

#[test]
fn python_rune_query_grader() {
    let (stdout, stderr) = run_python_script("python_query.rn", "query-cases");
    insta::assert_snapshot!("python_rune_query_stdout", stdout);
    insta::assert_snapshot!("python_rune_query_stderr", stderr);
}

#[test]
fn python_rune_query_constraints() {
    let (stdout, stderr) = run_python_script("python_query_constraints.rn", "query-cases");
    insta::assert_snapshot!("python_rune_query_constraints_stdout", stdout);
    insta::assert_snapshot!("python_rune_query_constraints_stderr", stderr);
}
