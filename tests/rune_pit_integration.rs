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

fn project_dir(name: &str) -> PathBuf {
    fixtures_root().join("java").join(name)
}

fn run_script(script: &str, workdir: &str) -> (String, String) {
    let mut cmd = cargo_bin_cmd!("umm");
    let script_path = rune_script(script);
    cmd.current_dir(project_dir(workdir))
        .env("CLICOLOR", "0")
        .env("NO_COLOR", "1")
        .arg("java")
        .arg("grade")
        .arg(script_path);

    let output = cmd.output().expect("failed to run command");
    (
        rune_support::normalize_rune_output(String::from_utf8_lossy(&output.stdout).to_string()),
        rune_support::normalize_rune_output(String::from_utf8_lossy(&output.stderr).to_string()),
    )
}

#[test]
fn rune_arraylist_docs_example() {
    let (stdout, stderr) = run_script("arraylist_docs.rn", "arraylist-example");
    insta::assert_snapshot!("rune_arraylist_docs_example_stdout", stdout);
    insta::assert_snapshot!("rune_arraylist_docs_example_stderr", stderr);
}

#[test]
fn rune_arraylist_docs_solution() {
    let (stdout, stderr) = run_script("arraylist_docs.rn", "arraylist-example-solution");
    insta::assert_snapshot!("rune_arraylist_docs_solution_stdout", stdout);
    insta::assert_snapshot!("rune_arraylist_docs_solution_stderr", stderr);
}

#[test]
fn rune_arraylist_mutation_example() {
    let (stdout, stderr) = run_script("arraylist_mutation.rn", "arraylist-example");
    insta::assert_snapshot!("rune_arraylist_mutation_example_stdout", stdout);
    insta::assert_snapshot!("rune_arraylist_mutation_example_stderr", stderr);
}

#[test]
fn rune_arraylist_mutation_solution() {
    let (stdout, stderr) = run_script("arraylist_mutation.rn", "arraylist-example-solution");
    insta::assert_snapshot!("rune_arraylist_mutation_solution_stdout", stdout);
    insta::assert_snapshot!("rune_arraylist_mutation_solution_stderr", stderr);
}
