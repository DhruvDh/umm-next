//! Tests for Python query grader functionality.

use std::path::PathBuf;

use umm::python::{
    Parser, Project,
    grade::query::{Query, QueryConstraint, QueryGrader},
    paths::ProjectPaths,
};

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

fn parse_fixture(path: &str) -> Parser {
    let full = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    let code = std::fs::read_to_string(full).expect("read fixture");
    Parser::new(code).expect("parse fixture")
}

#[test]
fn parser_finds_function_definition() {
    let parser = parse_fixture("fixtures/python/query-cases/example.py");
    let query = "(function_definition name: (identifier) @name)";
    let matches = parser.query(query).expect("run query");
    assert!(!matches.is_empty(), "should find function definitions");

    // Check that we found expected functions
    let names: Vec<_> = matches.iter().filter_map(|m| m.get("name")).collect();
    assert!(names.iter().any(|n| n.contains("main")));
    assert!(names.iter().any(|n| n.contains("sum_with_loop")));
    assert!(names.iter().any(|n| n.contains("squares_comprehension")));
}

#[test]
fn parser_finds_class_definition() {
    let parser = parse_fixture("fixtures/python/query-cases/example.py");
    let query = "(class_definition name: (identifier) @name)";
    let matches = parser.query(query).expect("run query");
    assert_eq!(matches.len(), 1, "should find one class");

    let name = matches[0].get("name").expect("name capture");
    assert!(name.contains("Calculator"));
}

#[test]
fn parser_finds_for_statement() {
    let parser = parse_fixture("fixtures/python/query-cases/example.py");
    let query = "(for_statement) @for";
    let matches = parser.query(query).expect("run query");
    assert!(!matches.is_empty(), "should find for statements");
}

#[test]
fn parser_finds_while_statement() {
    let parser = parse_fixture("fixtures/python/query-cases/example.py");
    let query = "(while_statement) @while";
    let matches = parser.query(query).expect("run query");
    assert!(!matches.is_empty(), "should find while statements");
}

#[test]
fn parser_finds_if_statement() {
    let parser = parse_fixture("fixtures/python/query-cases/example.py");
    let query = "(if_statement) @if";
    let matches = parser.query(query).expect("run query");
    assert!(!matches.is_empty(), "should find if statements");
}

#[test]
fn parser_finds_list_comprehension() {
    let parser = parse_fixture("fixtures/python/query-cases/example.py");
    let query = "(list_comprehension) @comp";
    let matches = parser.query(query).expect("run query");
    assert!(!matches.is_empty(), "should find list comprehensions");
}

#[tokio::test]
async fn query_grader_at_least_once_succeeds() {
    let project = project_for("query-cases");
    let grader = QueryGrader::builder()
        .req_name("q1")
        .out_of(5.0)
        .project(project)
        .file("example")
        .queries(vec![
            Query::new()
                .set_query("(function_definition name: (identifier) @name)".into())
                .set_capture("name".into()),
        ])
        .constraint(QueryConstraint::MustMatchAtLeastOnce)
        .reason("expected function definitions")
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 5.0);
}

#[tokio::test]
async fn query_grader_exact_count_succeeds() {
    let project = project_for("query-cases");
    // Count classes - should be 1
    let grader = QueryGrader::builder()
        .req_name("q2")
        .out_of(5.0)
        .project(project)
        .file("example")
        .queries(vec![
            Query::new()
                .set_query("(class_definition name: (identifier) @name)".into())
                .set_capture("name".into()),
        ])
        .constraint(QueryConstraint::MustMatchExactlyNTimes(1))
        .reason("expected one class")
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 5.0);
}

#[tokio::test]
async fn query_grader_exact_count_fails() {
    let project = project_for("query-cases");
    // Expect 10 classes when only 1 exists
    let grader = QueryGrader::builder()
        .req_name("q3")
        .out_of(5.0)
        .project(project)
        .file("example")
        .queries(vec![
            Query::new()
                .set_query("(class_definition name: (identifier) @name)".into())
                .set_capture("name".into()),
        ])
        .constraint(QueryConstraint::MustMatchExactlyNTimes(10))
        .reason("expected ten classes")
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 0.0);
}

#[tokio::test]
async fn query_grader_must_not_match_passes_on_zero() {
    let project = project_for("query-cases");
    // Check for a nonexistent class
    let grader = QueryGrader::builder()
        .req_name("q4")
        .out_of(3.0)
        .project(project)
        .file("example")
        .queries(vec![
            Query::new()
                .set_query(
                    "(class_definition name: (identifier) @name (#eq? @name \"NonExistent\"))"
                        .into(),
                )
                .set_capture("name".into()),
        ])
        .constraint(QueryConstraint::MustNotMatch)
        .reason("NonExistent class should not exist")
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 3.0);
}

#[tokio::test]
async fn query_grader_must_not_match_fails_when_present() {
    let project = project_for("query-cases");
    // Check for Calculator class (which exists)
    let grader = QueryGrader::builder()
        .req_name("q5")
        .out_of(3.0)
        .project(project)
        .file("example")
        .queries(vec![
            Query::new()
                .set_query(
                    "(class_definition name: (identifier) @name (#eq? @name \"Calculator\"))"
                        .into(),
                )
                .set_capture("name".into()),
        ])
        .constraint(QueryConstraint::MustNotMatch)
        .reason("Calculator class should not exist")
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 0.0);
}

#[tokio::test]
async fn query_grader_filter_predicate_applies() {
    let project = project_for("query-cases");
    let filtered = Query::new()
        .set_query("(function_definition name: (identifier) @name)".into())
        .set_capture("name".into())
        .set_filter_fn(|v| v.contains("sum"));

    let grader = QueryGrader::builder()
        .req_name("q6")
        .out_of(4.0)
        .project(project)
        .file("example")
        .queries(vec![filtered])
        .constraint(QueryConstraint::MustMatchExactlyNTimes(2)) // sum_with_loop and sum_with_while
        .reason("should find two sum functions")
        .build()
        .run()
        .await
        .expect("grade");

    assert_eq!(grader.grade_value(), 4.0);
}

#[tokio::test]
async fn query_grader_rejects_unknown_file() {
    let project = project_for("query-cases");
    let result = QueryGrader::builder()
        .req_name("q7")
        .out_of(1.0)
        .project(project)
        .file("nonexistent")
        .queries(vec![
            Query::new()
                .set_query("(function_definition) @body".into())
                .set_capture("body".into()),
        ])
        .constraint(QueryConstraint::MustMatchAtLeastOnce)
        .reason("missing file should fail")
        .build();

    // Build succeeds but grade should error because file is absent
    let grade = result.run().await;
    assert!(grade.is_err(), "expected error for missing file");
}
