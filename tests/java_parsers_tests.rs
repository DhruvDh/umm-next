use umm::java::{
    grade::{DiagnosticSeverity, LineRef},
    parsers::parser,
};

#[test]
fn mutation_row_parses_none_marker() {
    let csv = "Foo,Bar,org.pitest.mutationtest.engine.gregor.mutators.MathMutator,method,42,\
               SURVIVED,none";
    let diag = parser::mutation_report_row(csv).expect("parse mutation row without test info");
    let lr: LineRef = diag.clone().into();
    assert_eq!(lr.line_number, 42);
    assert_eq!(lr.file_name(), "Bar");
    assert_eq!(diag.result(), "SURVIVED");
    let snapshot = serde_json::to_value(&diag).unwrap();
    assert_eq!(snapshot["test_file_name"], "NA");
    assert_eq!(snapshot["test_method"], "None");
}

#[test]
fn mutation_row_parses_class_and_method_hints() {
    let csv = "Foo,Source,org.pitest.mutationtest.engine.gregor.mutators.\
               ConditionalsBoundaryMutator,method,7,KILLED,a/[class:MyTest]/[method:testAdds()]";
    let diag = parser::mutation_report_row(csv).expect("parse mutation row with hints");
    assert_eq!(diag.result(), "KILLED");
    let snapshot = serde_json::to_value(&diag).unwrap();
    assert_eq!(snapshot["test_file_name"], "MyTest");
    assert_eq!(snapshot["test_method"], "testAdds");
}

#[test]
fn mutation_row_preserves_other_status() {
    let csv = "Foo,Source,org.pitest.mutationtest.engine.gregor.mutators.RemoveConditionalMutator,\
               method,3,TIMED_OUT,none";
    let diag = parser::mutation_report_row(csv).expect("parse mutation row with other status");
    assert_eq!(diag.result(), "TIMED_OUT");
}

#[test]
fn mutation_row_rejects_malformed_csv() {
    assert!(parser::mutation_report_row("too,few,columns").is_err());
}

#[test]
fn javac_diag_parses_error_and_warning() {
    let error_line = "./Foo.java:12: error: missing semicolon";
    let warning_line = "./Foo.java:15: warning: unchecked call";

    let error_diag = parser::parse_diag(error_line).expect("parse javac error");
    assert_eq!(error_diag.file_name(), "Foo.java");
    assert_eq!(error_diag.path().display().to_string(), "./Foo.java");
    assert!(error_diag.severity().is_error());

    let warning_diag = parser::parse_diag(warning_line).expect("parse javac warning");
    assert_eq!(warning_diag.severity(), DiagnosticSeverity::Warning);
    let snapshot = serde_json::to_value(&warning_diag).unwrap();
    assert!(
        snapshot["message"]
            .as_str()
            .unwrap()
            .contains("unchecked call")
    );
}

#[test]
fn javac_diag_rejects_invalid_line() {
    assert!(parser::parse_diag("not a diagnostic line").is_err());
}
