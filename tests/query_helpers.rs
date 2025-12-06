use std::path::PathBuf;

use umm::java::{
    Parser, Project,
    grade::query::{Query, QueryConstraint, QueryGrader},
    paths::ProjectPaths,
};

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("java")
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
fn method_body_with_name_captures_expected() {
    let parser = parse_fixture("fixtures/java/query-cases/src/query/Example.java");
    let query = format!(include_str!("../src/java/queries/method_body_with_name.scm"), "foo");
    let matches = parser.query(&query).expect("run query");
    assert_eq!(matches.len(), 1);
    let body = matches[0].get("body").expect("body capture");
    assert!(body.contains("for (int i = 0; i < n; i++)"));
    assert!(body.contains("while (sum > 0)"));
}

#[test]
fn method_invocations_with_name_finds_multiple_calls() {
    let parser = parse_fixture("fixtures/java/query-cases/src/query/Example.java");
    let query =
        format!(include_str!("../src/java/queries/method_invocations_with_name.scm"), "println");
    let matches = parser.query(&query).expect("run query");
    assert_eq!(matches.len(), 2, "expected two println calls (if/else)");
    assert!(
        matches
            .iter()
            .all(|m| m.get("body").unwrap().contains("System.out"))
    );
}

#[test]
fn control_flow_queries_cover_if_for_while() {
    let parser = parse_fixture("fixtures/java/query-cases/src/query/Example.java");
    let ifs = parser.query("((if_statement) @if)").expect("if query");
    let fors = parser.query("((for_statement) @for)").expect("for query");
    let whiles = parser
        .query("((while_statement) @while)")
        .expect("while query");
    assert_eq!(ifs.len(), 1);
    assert_eq!(fors.len(), 1);
    assert_eq!(whiles.len(), 1);
}

#[test]
fn query_grader_exact_count_succeeds() {
    let project = project_for("query-cases");
    let parser = parse_fixture("fixtures/java/query-cases/src/query/Example.java");
    let invocation_query = "((method_invocation (identifier) @body))";
    let expected_count = parser
        .query(invocation_query)
        .expect("count invocations")
        .len();
    let grader = QueryGrader::builder()
        .req_name("q1")
        .out_of(5.0)
        .project(project.clone())
        .file("query.Example")
        .queries(vec![
            Query::new()
                .set_query(invocation_query.into())
                .set_capture("body".into()),
        ])
        .constraint(QueryConstraint::MustMatchExactlyNTimes(expected_count))
        .reason("expected three invocations")
        .build()
        .grade_by_query()
        .expect("grade");

    assert_eq!(grader.grade_value(), 5.0);
}

#[test]
fn query_grader_exact_count_fails_and_sets_prompt() {
    let project = project_for("query-cases");
    let grader = QueryGrader::builder()
        .req_name("q2")
        .out_of(5.0)
        .project(project.clone())
        .file("query.Example")
        .queries(vec![
            Query::new()
                .set_query("((method_invocation (identifier) @body))".into())
                .set_capture("body".into()),
        ])
        .constraint(QueryConstraint::MustMatchExactlyNTimes(5))
        .reason("expected five invocations")
        .build()
        .grade_by_query()
        .expect("grade");

    assert_eq!(grader.grade_value(), 0.0);
}

#[test]
fn query_grader_filter_predicate_applies() {
    let project = project_for("query-cases");
    let filtered = Query::new()
        .set_query(
            "((local_variable_declaration declarator: (variable_declarator name: (identifier) \
             @var)))"
                .into(),
        )
        .set_capture("var".into())
        .set_filter_fn(|v| v == "sum");

    let grader = QueryGrader::builder()
        .req_name("q3")
        .out_of(4.0)
        .project(project)
        .file("query.Example")
        .queries(vec![filtered])
        .constraint(QueryConstraint::MustMatchExactlyNTimes(1))
        .reason("should keep only sum")
        .build()
        .grade_by_query()
        .expect("grade");

    assert_eq!(grader.grade_value(), 4.0);
}

#[test]
fn query_grader_must_not_match_passes_on_zero() {
    let project = project_for("query-cases");
    let grader = QueryGrader::builder()
        .req_name("q4")
        .out_of(3.0)
        .project(project.clone())
        .file("query.Example")
        .queries(vec![
            Query::new()
                .set_query("((method_invocation (identifier) @body))".into())
                .set_capture("body".into())
                .set_filter_fn(|v| v == "nonexistent"),
        ])
        .constraint(QueryConstraint::MustNotMatch)
        .reason("should not find filtered calls")
        .build()
        .grade_by_query()
        .expect("grade");

    assert_eq!(grader.grade_value(), 3.0);
}

#[test]
fn query_grader_must_not_match_fails_when_present() {
    let project = project_for("query-cases");
    let grader = QueryGrader::builder()
        .req_name("q5")
        .out_of(3.0)
        .project(project.clone())
        .file("query.Example")
        .queries(vec![
            Query::new()
                .set_query("((if_statement) @if)".into())
                .set_capture("if".into()),
        ])
        .constraint(QueryConstraint::MustNotMatch)
        .reason("if should be present")
        .build()
        .grade_by_query()
        .expect("grade");

    assert_eq!(grader.grade_value(), 0.0);
}

#[test]
fn query_grader_at_least_once_fails_when_zero() {
    let project = project_for("query-cases");
    let grader = QueryGrader::builder()
        .req_name("q6")
        .out_of(2.0)
        .project(project.clone())
        .file("query.Example")
        .queries(vec![
            Query::new()
                .set_query("((class_declaration name: (identifier) @name))".into())
                .set_capture("name".into())
                .set_filter_fn(|v| v == "Nope"),
        ])
        .constraint(QueryConstraint::MustMatchAtLeastOnce)
        .reason("class Nope not present")
        .build()
        .grade_by_query()
        .expect("grade");

    assert_eq!(grader.grade_value(), 0.0);
}

#[test]
fn query_grader_chained_queries_operate_on_matches() {
    let project = project_for("query-cases");
    // First query: get the body of foo; second: find if_statement inside that body.
    let q1 = Query::new()
        .set_query(format!(include_str!("../src/java/queries/method_body_with_name.scm"), "foo"))
        .set_capture("body".into());
    let q2 = Query::new()
        .set_query("((if_statement) @if)".into())
        .set_capture("if".into());

    let grader = QueryGrader::builder()
        .req_name("q7")
        .out_of(2.5)
        .project(project.clone())
        .file("query.Example")
        .queries(vec![q1, q2])
        .constraint(QueryConstraint::MustMatchExactlyNTimes(1))
        .reason("one if inside foo")
        .build()
        .grade_by_query()
        .expect("grade");

    assert_eq!(grader.grade_value(), 2.5);
}

#[test]
fn query_grader_errors_when_capture_missing() {
    let project = project_for("query-cases");
    let grader = QueryGrader::builder()
        .req_name("q8")
        .out_of(1.0)
        .project(project.clone())
        .file("query.Example")
        // Deliberately omit capture
        .queries(vec![Query::new().set_query("((for_statement) @for)".into())])
        .constraint(QueryConstraint::MustMatchAtLeastOnce)
        .reason("missing capture should error")
        .build()
        .grade_by_query()
        .expect("grade");

    assert_eq!(grader.grade_value(), 0.0);
}

#[test]
fn query_grader_rejects_unknown_file() {
    let project = project_for("query-cases");
    let result = QueryGrader::builder()
        .req_name("q9")
        .out_of(1.0)
        .project(project)
        .file("query.DoesNotExist")
        .queries(vec![
            Query::new()
                .set_query("((class_declaration name: (identifier) @name))".into())
                .set_capture("name".into()),
        ])
        .constraint(QueryConstraint::MustMatchAtLeastOnce)
        .reason("missing file should fail")
        .build();

    // Build succeeds (bon validates required fields), but grade should error
    // because file is absent.
    let grade = result.grade_by_query().expect("grade");
    assert_eq!(grade.grade_value(), 0.0);
}

#[test]
fn query_grader_nested_queries_yield_no_matches_propagates_error() {
    let project = project_for("query-cases");
    // First query matches foo body; second intentionally looks for a capture that
    // won't exist.
    let q1 = Query::new()
        .set_query(format!(include_str!("../src/java/queries/method_body_with_name.scm"), "foo"))
        .set_capture("body".into());
    let q2 = Query::new()
        .set_query("((switch_expression) @switch)".into())
        .set_capture("switch".into());

    let grader = QueryGrader::builder()
        .req_name("q10")
        .out_of(2.0)
        .project(project)
        .file("query.Example")
        .queries(vec![q1, q2])
        .constraint(QueryConstraint::MustMatchAtLeastOnce)
        .reason("no switch inside foo")
        .build()
        .grade_by_query()
        .expect("grade");

    assert_eq!(grader.grade_value(), 0.0);
}

#[test]
fn query_grader_multiple_files_independent() {
    // Use the same project but switch files to confirm independent selection.
    let project = project_for("query-cases");
    // First grader on Example succeeds.
    let g1 = QueryGrader::builder()
        .req_name("g1")
        .out_of(1.0)
        .project(project.clone())
        .file("query.Example")
        .queries(vec![
            Query::new()
                .set_query("((for_statement) @for)".into())
                .set_capture("for".into()),
        ])
        .constraint(QueryConstraint::MustMatchExactlyNTimes(1))
        .build()
        .grade_by_query()
        .expect("grade");
    assert_eq!(g1.grade_value(), 1.0);

    // Second grader intentionally points to a non-existent helper file; should
    // error.
    let g2 = QueryGrader::builder()
        .req_name("g2")
        .out_of(1.0)
        .project(project)
        .file("query.Helper")
        .queries(vec![
            Query::new()
                .set_query("((method_declaration name: (identifier) @name))".into())
                .set_capture("name".into()),
        ])
        .constraint(QueryConstraint::MustMatchAtLeastOnce)
        .build();
    let g2 = g2.grade_by_query().expect("grade");
    assert_eq!(g2.grade_value(), 0.0);
}
