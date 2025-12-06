//! Tests for Python parser functionality.

use std::path::PathBuf;

use umm::{
    python::{Parser, Project, grade::context::get_source_context, paths::ProjectPaths},
    types::LineRef,
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
fn parser_creates_successfully() {
    let code = r#"
def hello():
    print("Hello, World!")
"#;
    let parser = Parser::new(code.to_string());
    assert!(parser.is_ok());
}

#[test]
fn parser_extracts_functions() {
    let parser = parse_fixture("fixtures/python/query-cases/example.py");
    let query = "(function_definition name: (identifier) @name)";
    let matches = parser.query(query).expect("run query");

    let names: Vec<_> = matches.iter().filter_map(|m| m.get("name")).collect();

    // Should find these functions
    assert!(names.iter().any(|n| n.contains("main")));
    assert!(names.iter().any(|n| n.contains("sum_with_loop")));
    assert!(names.iter().any(|n| n.contains("sum_with_while")));
    assert!(names.iter().any(|n| n.contains("process_value")));
    assert!(names.iter().any(|n| n.contains("squares_comprehension")));
}

#[test]
fn parser_extracts_classes() {
    let parser = parse_fixture("fixtures/python/query-cases/example.py");
    let query = "(class_definition name: (identifier) @name body: (block) @body)";
    let matches = parser.query(query).expect("run query");

    assert_eq!(matches.len(), 1);
    let name = matches[0].get("name").expect("name capture");
    assert!(name.contains("Calculator"));
}

#[test]
fn parser_extracts_imports() {
    let code = r#"
import os
from pathlib import Path
from typing import List, Dict
"#;
    let parser = Parser::new(code.to_string()).expect("parse");
    let query = "(import_statement) @imp";
    let matches = parser.query(query).expect("run query");
    assert!(!matches.is_empty());
}

#[test]
fn parser_handles_docstrings() {
    let code = r#"
def greet(name):
    """Greet a person.
    
    Args:
        name: The name to greet.
        
    Returns:
        A greeting string.
    """
    return f"Hello, {name}!"
"#;
    let parser = Parser::new(code.to_string()).expect("parse");

    // Find string expressions (docstrings)
    let query = "(expression_statement (string)) @doc";
    let matches = parser.query(query).expect("run query");
    assert!(!matches.is_empty());
}

#[test]
fn project_discovers_files() {
    let project = project_for("query-cases");
    // Use count() instead of is_empty() since files() returns an iterator
    let file_count = project.files().count();

    assert!(file_count > 0, "should discover Python files");
}

#[test]
fn project_identifies_file_by_name() {
    let project = project_for("query-cases");
    let file = project.identify("example");

    assert!(file.is_ok(), "should identify example.py");
}

#[test]
fn project_file_has_main() {
    let project = project_for("query-cases");
    let file = project.identify("example").expect("identify file");

    assert!(file.has_main(), "example.py should have a main block");
}

#[test]
fn project_file_lists_functions() {
    let project = project_for("query-cases");
    let file = project.identify("example").expect("identify file");
    let functions = file.functions();

    assert!(functions.contains(&"main".to_string()));
    assert!(functions.contains(&"sum_with_loop".to_string()));
}

#[test]
fn project_file_lists_classes() {
    let project = project_for("query-cases");
    let file = project.identify("example").expect("identify file");
    let classes = file.classes();

    assert!(classes.contains(&"Calculator".to_string()));
}

#[test]
fn project_identifies_file_with_prefixed_path() {
    let project = project_for("query-cases");
    let root = fixture_root("query-cases");

    let abs_path = root.join("example.py");
    let rel_path = PathBuf::from("fixtures/python/query-cases/example.py");

    assert!(
        project
            .identify(abs_path.to_str().expect("abs path to str"))
            .is_ok(),
        "should resolve file when traceback carries an absolute path prefix"
    );
    assert!(
        project
            .identify(rel_path.to_str().expect("rel path to str"))
            .is_ok(),
        "should resolve file when traceback carries a relative path prefix"
    );
}

#[test]
fn get_source_context_handles_prefixed_line_refs() {
    let project = project_for("query-cases");
    let file = project.identify("example").expect("identify file");
    let abs_path = fixture_root("query-cases").join("example.py");

    let line_ref = LineRef {
        file_name:   abs_path.to_string_lossy().to_string(),
        line_number: 5,
    };

    let context = get_source_context(&file, &[line_ref], 2);

    assert!(
        context.contains("example.py"),
        "context should include the filename for readability"
    );
    assert!(
        context.contains(">>>"),
        "context should mark the target line even when the LineRef filename has a prefix"
    );
}
