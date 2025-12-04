use serde_json::Value;
use umm::java::{
    grade::{
        diff::DiffGrader,
        docs::DocsGrader,
        gradescope::{GradescopeSubmission, GradescopeTestCase},
    },
    project::Project,
};

#[test]
fn diff_builder_accepts_tuple_cases() {
    let grader = DiffGrader::builder()
        .req_name("req")
        .out_of(10.0)
        .project(Project::default())
        .file("Main.java")
        .cases([("expected output", Option::<String>::None)])
        .build();

    assert_eq!(grader.cases.len(), 1);
    assert_eq!(grader.cases[0].expected, "expected output");
    assert!(grader.cases[0].input.is_none());
}

#[test]
fn docs_builder_takes_iterables() {
    let grader = DocsGrader::builder()
        .project(Project::default())
        .files(["A.java", "B.java"])
        .out_of(5.0)
        .req_name("docs")
        .build();

    assert_eq!(grader.files.len(), 2);
}

#[test]
fn gradescope_builder_from_iter() {
    let test_case = GradescopeTestCase::builder().name("case").build();
    let submission = GradescopeSubmission::builder().tests([test_case]).build();

    let value: Value = serde_json::to_value(&submission).expect("serialize gradescope");
    assert!(value.get("tests").is_some());
    assert!(value["tests"].is_array());
}
