#![allow(deprecated)]

use std::{fs, path::PathBuf};

use assert_cmd::Command;

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn rune_script(name: &str) -> PathBuf {
    fixtures_root().join("rune").join(name)
}

fn project_dir(name: &str) -> PathBuf {
    fixtures_root().join("java").join(name)
}

fn run_script(script: &str, workdir: &str) -> (String, String) {
    let mut cmd = Command::cargo_bin("umm").expect("binary exists");
    let script_path = rune_script(script);
    cmd.current_dir(project_dir(workdir))
        .env("CLICOLOR", "0")
        .arg("java")
        .arg("grade")
        .arg(script_path);

    let assert = cmd.assert().success();
    let output = assert.get_output().clone();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr)
}

#[test]
fn rune_happy_path() {
    let (stdout, stderr) = run_script("happy.rn", "rune-hello");
    insta::assert_snapshot!("rune_happy_stdout", stdout);
    insta::assert_snapshot!("rune_happy_stderr", stderr);
}

#[test]
fn rune_missing_required() {
    let mut cmd = Command::cargo_bin("umm").expect("binary exists");
    let script_path = rune_script("missing_required.rn");
    cmd.current_dir(project_dir("rune-hello"))
        .env("CLICOLOR", "0")
        .arg("java")
        .arg("grade")
        .arg(script_path);

    let assert = cmd.assert().failure();
    let output = assert.get_output().clone();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    insta::assert_snapshot!("rune_missing_required_stdout", stdout);
    insta::assert_snapshot!("rune_missing_required_stderr", stderr);
}

#[test]
fn rune_gradescope_json() {
    let workdir = project_dir("rune-hello");
    let results_path = workdir.join("results.json");
    let _ = fs::remove_file(&results_path);

    let mut cmd = Command::cargo_bin("umm").expect("binary exists");
    let script_path = rune_script("gradescope_json.rn");
    cmd.current_dir(&workdir)
        .env("CLICOLOR", "0")
        .arg("java")
        .arg("grade")
        .arg(script_path);

    cmd.assert().success();

    let contents = fs::read_to_string(&results_path).expect("results.json written");
    insta::assert_snapshot!("rune_gradescope_results_json", contents);

    let _ = fs::remove_file(results_path);
}

#[test]
fn rune_query_grader() {
    let (stdout, stderr) = run_script("query.rn", "rune-hello");
    insta::assert_snapshot!("rune_query_stdout", stdout);
    insta::assert_snapshot!("rune_query_stderr", stderr);
}
