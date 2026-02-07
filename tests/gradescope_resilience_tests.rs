use std::{fs, path::PathBuf};

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn rune_script(name: &str) -> PathBuf {
    fixtures_root().join("rune").join(name)
}

fn java_project_dir(name: &str) -> PathBuf {
    fixtures_root().join("java").join(name)
}

fn run_java_grading_script(
    script: &str,
    workdir: &str,
    env: &[(&str, &str)],
    remove_env: &[&str],
) -> Value {
    let workdir = java_project_dir(workdir);
    let results_path = workdir.join("results.json");

    let mut cmd = cargo_bin_cmd!("umm");
    cmd.current_dir(&workdir)
        .env("CLICOLOR", "0")
        .arg("java")
        .arg("grade")
        .arg(rune_script(script));

    for (key, value) in env {
        cmd.env(key, value);
    }
    for key in remove_env {
        cmd.env_remove(key);
    }

    cmd.assert().success();
    let raw = fs::read_to_string(&results_path).expect("results.json written");
    serde_json::from_str(&raw).expect("valid results json")
}

fn warning_codes(json: &Value) -> Vec<String> {
    json["extra_data"]["warnings"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| entry["code"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn supabase_feedback_failure_is_non_fatal_and_reported() {
    let json = run_java_grading_script(
        "gradescope_feedback_warning.rn",
        "rune-hello",
        &[],
        &["SUPABASE_URL", "SUPABASE_ANON_KEY"],
    );

    let output = json["tests"][0]["output"].as_str().unwrap_or_default();
    assert!(output.contains("SUPABASE_FEEDBACK_FAILED"));

    let submission_output = json["output"].as_str().unwrap_or_default();
    assert!(submission_output.contains("Autograder completed with warnings"));
    assert!(submission_output.contains("SUPABASE_FEEDBACK_FAILED"));
    assert!(warning_codes(&json).contains(&"SUPABASE_FEEDBACK_FAILED".to_string()));
}

#[test]
fn missing_openai_config_is_non_fatal_and_preserves_scores() {
    let json = run_java_grading_script(
        "gradescope_slo_missing_openai.rn",
        "rune-hello",
        &[],
        &["OPENAI_ENDPOINT", "OPENAI_API_KEY_SLO", "OPENAI_MODEL"],
    );

    assert_eq!(json["tests"][0]["score"].as_f64(), Some(2.0));
    let submission_output = json["output"].as_str().unwrap_or_default();
    assert!(submission_output.contains("SLO_OPENAI_CONFIG_MISSING"));
    assert!(warning_codes(&json).contains(&"SLO_OPENAI_CONFIG_MISSING".to_string()));
}

#[test]
fn misconfigured_slo_files_are_reported_without_failing_artifact_generation() {
    let json = run_java_grading_script(
        "gradescope_slo_misconfigured.rn",
        "rune-hello",
        &[
            ("OPENAI_ENDPOINT", "http://127.0.0.1:1/v1"),
            ("OPENAI_API_KEY_SLO", "test-key"),
            ("OPENAI_MODEL", "test-model"),
        ],
        &[],
    );

    assert_eq!(json["tests"][0]["score"].as_f64(), Some(2.0));
    let codes = warning_codes(&json);
    assert!(codes.contains(&"SLO_NO_RELEVANT_FILES".to_string()));
    assert!(codes.contains(&"SLO_NO_RESPONSES".to_string()));
    assert!(
        json["output"]
            .as_str()
            .unwrap_or_default()
            .contains("Autograder completed with warnings")
    );
}
