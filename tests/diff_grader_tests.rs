use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use umm::java::{Project, grade::diff::DiffGrader, paths::ProjectPaths};

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("java")
        .join(name)
}

fn project(name: &str) -> Project {
    let root = fixture_root(name);
    let paths = ProjectPaths::from_parts(root, None, None, None, None, None, None);
    Project::from_paths(paths).expect("build project")
}

fn temp_root(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("umm-java-diff-{prefix}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(root.join("src/pkg1")).expect("create pkg1");
    fs::create_dir_all(root.join("src/pkg2")).expect("create pkg2");
    root
}

fn write_file(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write java file");
}

fn project_from_root(root: &Path) -> Project {
    let paths = ProjectPaths::from_parts(
        root.to_path_buf(),
        Some(root.join("src")),
        None,
        None,
        None,
        None,
        None,
    );
    Project::from_paths(paths).expect("build project")
}

#[tokio::test]
async fn diff_passes_on_exact_match_trimmed() {
    let proj = project("diff-ok");
    let grader = DiffGrader::builder()
        .req_name("ok")
        .out_of(2.0)
        .project(proj)
        .file("Main")
        .cases(vec![("hello world", None::<String>)])
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
        .file("Main")
        .cases(vec![("hello  world", None::<String>)])
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
        .file("Main")
        .cases(vec![("HELLO WORLD", None::<String>)])
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
        .file("Main")
        .cases(vec![("  hello world  \n", None::<String>)])
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
        .file("Main")
        .cases(vec![("ping\n", Some("ping".to_string()))])
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
        .file("Main")
        .cases(vec![("irrelevant", None::<String>)])
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 0.0);
}

#[tokio::test]
async fn diff_handles_compile_failure() {
    let proj = project("diff-compile");
    let grader = DiffGrader::builder()
        .req_name("compile")
        .out_of(3.0)
        .project(proj)
        .file("Main")
        .cases(vec![("irrelevant", None::<String>)])
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
        .file("Main")
        .cases(vec![
            ("hello world", None::<String>),
            ("another", None::<String>), // should fail here
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
        .file("Main")
        .build();

    let err = grader.run().await;
    assert!(err.is_err(), "expected missing cases error");
}

#[tokio::test]
async fn diff_compile_failure_with_ambiguous_simple_names_degrades_gracefully() {
    let root = temp_root("ambiguous-compile");
    write_file(
        &root.join("src/pkg1/Main.java"),
        r#"package pkg1;

public class Main {
    public static void main(String[] args) {
        System.out.println("broken")
    }
}
"#,
    );
    write_file(
        &root.join("src/pkg2/Main.java"),
        r#"package pkg2;

public class Main {
    public static void main(String[] args) {
        System.out.println("ok");
    }
}
"#,
    );

    let proj = project_from_root(&root);
    let grader = DiffGrader::builder()
        .req_name("ambiguous-compile")
        .out_of(1.0)
        .project(proj)
        .file("pkg1.Main")
        .cases(vec![("irrelevant", None::<String>)])
        .build()
        .run()
        .await
        .expect("compile failure should return a grade result");

    assert_eq!(grader.grade_value(), 0.0);
    assert!(
        grader
            .reason()
            .contains("Error compiling file for some cases.")
    );

    let _ = fs::remove_dir_all(root);
}
