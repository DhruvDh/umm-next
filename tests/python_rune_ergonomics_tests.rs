//! Snapshot tests for Python Rune module ergonomics (new convenience methods).
//!
//! These tests verify the improved ergonomics using insta snapshots:
//! - DiffGrader: expect(), expect_with_input()
//! - QueryGrader: defines_function(), defines_class(), uses_*, imports_*,
//!   must_not_*
//! - Default constraint behavior (MustMatchAtLeastOnce)

use std::path::PathBuf;

use assert_cmd::cargo::cargo_bin_cmd;
use insta::assert_snapshot;

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn python_fixture_dir(name: &str) -> PathBuf {
    fixtures_root().join("python").join(name)
}

fn rune_ergonomics_script(name: &str) -> PathBuf {
    fixtures_root()
        .join("rune")
        .join("ergonomics")
        .join(format!("{}.rn", name))
}

/// Run a Rune script from fixtures/rune/ergonomics/ against a Python project
fn run_ergonomics_script(script_name: &str, python_project: &str) -> (bool, String, String) {
    let script_path = rune_ergonomics_script(script_name);
    let mut cmd = cargo_bin_cmd!("umm");
    cmd.current_dir(python_fixture_dir(python_project))
        .env("CLICOLOR", "0")
        .arg("python")
        .arg("grade")
        .arg(&script_path);

    let output = cmd.output().expect("run command");
    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (success, stdout, stderr)
}

// ============================================================================
// DiffGrader Ergonomics Tests
// ============================================================================

#[test]
fn diff_expect_method() {
    let (success, _stdout, stderr) = run_ergonomics_script("diff_expect", "diff-ok");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn diff_expect_with_input_method() {
    let (success, _stdout, stderr) = run_ergonomics_script("diff_expect_with_input", "diff-stdin");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn diff_expect_chain() {
    let (success, _stdout, stderr) = run_ergonomics_script("diff_expect_chain", "diff-ok");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

// ============================================================================
// QueryGrader Default Constraint Tests
// ============================================================================

#[test]
fn query_default_constraint() {
    let (success, _stdout, stderr) =
        run_ergonomics_script("query_default_constraint", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

// ============================================================================
// QueryGrader Convenience Method Tests
// ============================================================================

#[test]
fn query_defines_function() {
    let (success, _stdout, stderr) = run_ergonomics_script("query_defines_function", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_defines_class() {
    let (success, _stdout, stderr) = run_ergonomics_script("query_defines_class", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_uses_try_except() {
    let (success, _stdout, stderr) = run_ergonomics_script("query_uses_try_except", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_uses_lambda() {
    let (success, _stdout, stderr) = run_ergonomics_script("query_uses_lambda", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_uses_decorator() {
    let (success, _stdout, stderr) = run_ergonomics_script("query_uses_decorator", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_uses_with_statement() {
    let (success, _stdout, stderr) =
        run_ergonomics_script("query_uses_with_statement", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_uses_yield() {
    let (success, _stdout, stderr) = run_ergonomics_script("query_uses_yield", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_uses_dict_comprehension() {
    let (success, _stdout, stderr) =
        run_ergonomics_script("query_uses_dict_comprehension", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_uses_set_comprehension() {
    let (success, _stdout, stderr) =
        run_ergonomics_script("query_uses_set_comprehension", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_uses_generator_expression() {
    let (success, _stdout, stderr) =
        run_ergonomics_script("query_uses_generator_expression", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_uses_assert() {
    let (success, _stdout, stderr) = run_ergonomics_script("query_uses_assert", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_uses_raise() {
    let (success, _stdout, stderr) = run_ergonomics_script("query_uses_raise", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_imports_module() {
    let (success, _stdout, stderr) = run_ergonomics_script("query_imports_module", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_imports_from() {
    let (success, _stdout, stderr) = run_ergonomics_script("query_imports_from", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

// ============================================================================
// Negated Convenience Methods Tests
// ============================================================================

#[test]
fn query_must_not_for_loop_passes_when_absent() {
    let (success, _stdout, stderr) =
        run_ergonomics_script("query_must_not_for_loop_pass", "hello-with-docstrings");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_must_not_for_loop_fails_when_present() {
    let (success, _stdout, stderr) =
        run_ergonomics_script("query_must_not_for_loop_fail", "query-cases");
    assert!(success, "script should succeed (grade 0)");
    assert_snapshot!(stderr);
}

#[test]
fn query_must_not_while_loop_passes_when_absent() {
    let (success, _stdout, stderr) =
        run_ergonomics_script("query_must_not_while_loop_pass", "hello-with-docstrings");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_must_not_recursion_passes_when_absent() {
    let (success, _stdout, stderr) =
        run_ergonomics_script("query_must_not_recursion_pass", "query-cases");
    assert!(success, "script should succeed");
    assert_snapshot!(stderr);
}

#[test]
fn query_must_not_recursion_fails_when_present() {
    let (success, _stdout, stderr) =
        run_ergonomics_script("query_must_not_recursion_fail", "query-cases");
    assert!(success, "script should succeed (grade 0)");
    assert_snapshot!(stderr);
}
