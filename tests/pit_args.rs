use std::{fs, path::PathBuf};

use umm::java::{
    Project,
    grade::tests::{MutationInputs, UnitTestGrader},
    paths::ProjectPaths,
};
use uuid::Uuid;

fn temp_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!("umm-pit-{}", Uuid::new_v4()));
    // Prepare expected layout
    for dir in ["src", "target", "test", "lib", ".umm/test_reports"] {
        fs::create_dir_all(root.join(dir)).expect("create layout");
    }
    root
}

#[test]
fn pit_args_include_report_dir_under_umm() {
    let root = temp_root();
    let paths = ProjectPaths::from_parts(root.clone(), None, None, None, None, None, None);
    let project = Project::from_paths(paths.clone()).expect("build project");

    let inputs = MutationInputs::new(
        vec!["example.ExampleTest".into()],
        vec!["example.Example".into()],
        vec!["skip".into()],
        vec!["java.io".into()],
    );

    let args = UnitTestGrader::build_mutation_args(&project, &inputs).expect("mutation args");

    // Assert reportDir is set to .umm/test_reports
    let idx = args
        .iter()
        .position(|a| a == "--reportDir")
        .expect("reportDir flag");
    let report_arg = args
        .get(idx + 1)
        .and_then(|v: &std::ffi::OsString| v.to_str())
        .expect("reportDir value");
    assert_eq!(report_arg, paths.report_dir().to_str().unwrap());

    // Ensure classpath includes build then lib entries (order matters)
    let cp_idx = args
        .iter()
        .position(|a| a == "--class-path")
        .expect("class-path flag");
    let cp_val = args
        .get(cp_idx + 1)
        .and_then(|v: &std::ffi::OsString| v.to_str())
        .expect("classpath value");
    // Expected ordering: target, lib, lib/*
    let sep = if cfg!(windows) { ';' } else { ':' };
    let mut parts = cp_val.split(sep);
    assert_eq!(parts.next(), Some(paths.build_dir().to_str().unwrap()));
    assert_eq!(parts.next(), Some(paths.lib_dir().to_str().unwrap()));
    // wildcard entry should be third
    let expected_wildcard = paths.lib_dir().join("*");
    assert_eq!(parts.next(), Some(expected_wildcard.to_str().unwrap()));

    let _ = fs::remove_dir_all(root);
}
