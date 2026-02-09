use std::time::Duration;

use umm::config::{
    DEFAULT_RETRIEVAL_ENDPOINT, parse_timeout_secs_value, resolve_retrieval_endpoint_value,
};

#[test]
fn empty_retrieval_endpoint_override_falls_back_to_default() {
    assert_eq!(resolve_retrieval_endpoint_value(Some("   ")), DEFAULT_RETRIEVAL_ENDPOINT);
    assert_eq!(resolve_retrieval_endpoint_value(None), DEFAULT_RETRIEVAL_ENDPOINT);
    assert_eq!(
        resolve_retrieval_endpoint_value(Some(" https://example.com/api ")),
        "https://example.com/api"
    );
}

#[test]
fn timeout_values_are_parsed_from_env_strings() {
    let python_timeout = parse_timeout_secs_value(Some("61"), 60);
    let lint_timeout = parse_timeout_secs_value(Some("31"), 30);
    let test_timeout = parse_timeout_secs_value(Some("121"), 120);

    assert_eq!(python_timeout, Duration::from_secs(61));
    assert_eq!(lint_timeout, Duration::from_secs(31));
    assert_eq!(test_timeout, Duration::from_secs(121));

    assert_eq!(parse_timeout_secs_value(Some("not-a-number"), 30), Duration::from_secs(30));
    assert_eq!(parse_timeout_secs_value(Some(""), 120), Duration::from_secs(120));
}
