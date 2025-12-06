//! Tests for Python diff grader functionality.

use std::path::PathBuf;

use umm::python::{
    Project,
    grade::diff::{DiffCase, DiffGrader},
    paths::ProjectPaths,
};

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("python")
        .join(name)
}

fn project(name: &str) -> Project {
    let root = fixture_root(name);
    let paths = ProjectPaths::from_parts(root, None, None, None, None, None, None);
    Project::from_paths(paths).expect("build project")
}

#[tokio::test]
async fn diff_passes_on_exact_match_trimmed() {
    let proj = project("diff-ok");
    let grader = DiffGrader::builder()
        .req_name("ok")
        .out_of(2.0)
        .project(proj)
        .file("main")
        .cases(vec![DiffCase::new("hello world")])
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 2.0);
}

#[tokio::test]
async fn diff_detects_mismatch_with_preserve_whitespace() {
    let proj = project("diff-ok");
    let grader = DiffGrader::builder()
        .req_name("ws")
        .out_of(1.0)
        .project(proj)
        .file("main")
        .cases(vec![DiffCase::new("hello  world")])
        .preserve_whitespace(true)
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 0.0);
}

#[tokio::test]
async fn diff_ignores_case_when_configured() {
    let proj = project("diff-ok");
    let grader = DiffGrader::builder()
        .req_name("case")
        .out_of(1.5)
        .project(proj)
        .file("main")
        .cases(vec![DiffCase::new("HELLO WORLD")])
        .ignore_case(true)
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 1.5);
}

#[tokio::test]
async fn diff_trims_when_whitespace_not_preserved() {
    let proj = project("diff-ok");
    let grader = DiffGrader::builder()
        .req_name("trim")
        .out_of(1.0)
        .project(proj)
        .file("main")
        .cases(vec![DiffCase::new("  hello world  \n")])
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 1.0);
}

#[tokio::test]
async fn diff_runs_with_stdin_bytes() {
    let proj = project("diff-stdin");
    let grader = DiffGrader::builder()
        .req_name("stdin")
        .out_of(1.0)
        .project(proj)
        .file("main")
        .cases(vec![DiffCase::new("ping\n").with_input("ping")])
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 1.0);
}

#[tokio::test]
async fn diff_handles_runtime_failure() {
    let proj = project("diff-runtime");
    let grader = DiffGrader::builder()
        .req_name("rt")
        .out_of(3.0)
        .project(proj)
        .file("main")
        .cases(vec![DiffCase::new("irrelevant")])
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 0.0);
}

#[tokio::test]
async fn diff_handles_syntax_error() {
    let proj = project("diff-syntax-error");
    let grader = DiffGrader::builder()
        .req_name("syntax")
        .out_of(3.0)
        .project(proj)
        .file("main")
        .cases(vec![DiffCase::new("irrelevant")])
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 0.0);
}

#[tokio::test]
async fn diff_multiple_cases_stops_on_first_failure() {
    let proj = project("diff-ok");
    let grader = DiffGrader::builder()
        .req_name("multi")
        .out_of(2.0)
        .project(proj)
        .file("main")
        .cases(vec![
            DiffCase::new("hello world"),
            DiffCase::new("another"), // should fail here
        ])
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 0.0);
}

#[tokio::test]
async fn diff_errors_when_no_cases() {
    let proj = project("diff-ok");
    let grader = DiffGrader::builder()
        .req_name("empty")
        .out_of(1.0)
        .project(proj)
        .file("main")
        .build();

    let err = grader.run().await;
    assert!(err.is_err(), "expected missing cases error");
}
