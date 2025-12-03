use umm::java::{Parser, queries::CLASSNAME_QUERY};

#[test]
fn query_returns_class_name_capture() {
    let parser = Parser::new("class Foo {}".to_string()).expect("parser should initialize");
    let captures = parser.query(CLASSNAME_QUERY).expect("query should succeed");
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].get("name").map(String::as_str), Some("Foo"));
}

#[test]
fn query_errors_on_invalid_query() {
    let parser = Parser::new("class Foo {}".to_string()).expect("parser should initialize");
    let err = parser.query("(invalid").unwrap_err();
    let message = err.to_string();
    assert!(message.contains("Failed to compile tree-sitter query"));
}

#[test]
fn query_capture_positions_errors_on_missing_capture() {
    let parser = Parser::new("class Foo {}".to_string()).expect("parser should initialize");
    let err = parser
        .query_capture_positions(CLASSNAME_QUERY, "not_a_capture")
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Capture name not_a_capture not present in query")
    );
}

#[test]
fn set_code_updates_tree() {
    let mut parser = Parser::new("class Foo {}".to_string()).expect("parser should initialize");
    parser
        .set_code("class Bar {}".to_string())
        .expect("set_code should succeed");

    let captures = parser.query(CLASSNAME_QUERY).expect("query should succeed");
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].get("name").map(String::as_str), Some("Bar"));
}
