use std::path::PathBuf;

use umm::python::{Project, grade::docs::DocsGrader, paths::ProjectPaths};

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("python")
        .join(name)
}

fn project_for(name: &str) -> Project {
    let root = fixture_root(name);
    let paths = ProjectPaths::from_parts(root, None, None, None, None, None, None);
    Project::from_paths(paths).expect("build project")
}

#[tokio::test]
async fn docstring_must_be_leading_statement() {
    let project = project_for("docs-late-string");
    let grader = DocsGrader::builder()
        .project(project)
        .files(vec!["main.py"])
        .out_of(3.0)
        .req_name("docs")
        .build();

    let result = grader.run().await.expect("run grader");

    assert_eq!(result.grade_struct().grade, 0.0);
    assert!(
        result
            .reason()
            .contains("Missing docstring for function 'foo'")
    );
    assert!(
        result
            .reason()
            .contains("Missing docstring for class 'Bar'")
    );
}
