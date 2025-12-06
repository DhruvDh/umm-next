#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Test graders for Python (pytest, unittest).

use std::time::Duration;

use anyhow::{Result, bail};
use async_openai::types::chat::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use bon::Builder;

use super::results::{Grade, GradeResult};
use crate::{
    config,
    process::{self, StdinSource},
    python::{Project, util::python_module_with_deps_command},
    types::LineRef,
};

/// A grader that runs Python tests using pytest.
#[derive(Clone, Default, Builder)]
#[builder(on(String, into))]
pub struct TestGrader {
    /// The project being graded.
    #[builder(getter)]
    project:    Project,
    /// Test files to run.
    #[builder(with = |iter: impl IntoIterator<Item = impl Into<String>>| {
        iter.into_iter().map(Into::into).collect::<Vec<String>>()
    })]
    #[builder(getter)]
    test_files: Vec<String>,
    /// Requirement name.
    #[builder(getter)]
    req_name:   String,
    /// Total points available.
    #[builder(getter)]
    out_of:     f64,
}

impl TestGrader {
    /// Builds and runs the grader.
    pub async fn run(self) -> Result<GradeResult> {
        if self.test_files.is_empty() {
            bail!("TestGrader requires at least one test file");
        }
        self.grade_by_tests().await
    }

    /// Performs the test grading.
    async fn grade_by_tests(self) -> Result<GradeResult> {
        let prompts = config::python_prompts();

        let mut total_tests = 0usize;
        let mut passed_tests = 0usize;
        let mut all_outputs = Vec::new();
        let mut all_diags = Vec::new();

        for test_file_name in &self.test_files {
            let file = self.project.identify(test_file_name)?;

            // Build pytest command using uv --with pytest
            let path_str = file.path().to_string_lossy();
            let (cmd, args) = python_module_with_deps_command(
                "pytest",
                &["pytest"],
                &["-v", "--tb=short", &path_str],
            )?;

            let collected = process::run_collect(
                &cmd,
                &args,
                StdinSource::Null,
                None,
                &[],
                Some(Duration::from_secs(120)),
            )
            .await?;

            let stdout = String::from_utf8_lossy(&collected.stdout).to_string();
            let stderr = String::from_utf8_lossy(&collected.stderr).to_string();
            let output = format!("{}\n{}", stdout, stderr);

            all_outputs.push(format!("### {}\n```\n{}\n```", test_file_name, output));

            // Parse test results
            let (file_total, file_passed) = self.parse_pytest_output(&output);
            total_tests += file_total;
            passed_tests += file_passed;

            // Extract diagnostics from failures
            if !collected.status.success() {
                let diags = self.extract_failure_locations(&output);
                all_diags.extend(diags);
            }
        }

        let grade = if total_tests > 0 {
            (passed_tests as f64 / total_tests as f64) * self.out_of
        } else {
            0.0
        };

        let reason = format!(
            "{}/{} tests passed ({:.1}%)",
            passed_tests,
            total_tests,
            if total_tests > 0 {
                (passed_tests as f64 / total_tests as f64) * 100.0
            } else {
                0.0
            }
        );

        let prompt = if passed_tests < total_tests {
            Some(vec![
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(prompts.system_message().to_string())
                    .name("Instructor".to_string())
                    .build()?
                    .into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content(format!("Test results:\n\n{}", all_outputs.join("\n\n")))
                    .name("Student".to_string())
                    .build()?
                    .into(),
            ])
        } else {
            None
        };

        Ok(GradeResult::builder()
            .requirement(self.req_name.clone())
            .grade(Grade::new(grade, self.out_of))
            .reason(reason)
            .maybe_prompt(prompt)
            .build())
    }

    /// Parses pytest output to extract test counts.
    fn parse_pytest_output(&self, output: &str) -> (usize, usize) {
        let mut total = 0usize;
        let mut passed = 0usize;

        // Look for summary line like "5 passed, 2 failed"
        for line in output.lines() {
            let line = line.trim();

            // Match patterns like "5 passed" or "5 passed, 2 failed"
            if line.contains("passed") || line.contains("failed") || line.contains("error") {
                // Parse "X passed"
                if let Some(idx) = line.find(" passed") {
                    if let Some(start) = line[..idx]
                        .rfind(|c: char| !c.is_ascii_digit())
                        .map(|i| i + 1)
                    {
                        if let Ok(n) = line[start..idx].trim().parse::<usize>() {
                            passed += n;
                            total += n;
                        }
                    } else if let Ok(n) = line[..idx].trim().parse::<usize>() {
                        passed += n;
                        total += n;
                    }
                }

                // Parse "X failed"
                if let Some(idx) = line.find(" failed") {
                    if let Some(start) = line[..idx]
                        .rfind(|c: char| !c.is_ascii_digit())
                        .map(|i| i + 1)
                    {
                        if let Ok(n) = line[start..idx].trim().parse::<usize>() {
                            total += n;
                        }
                    } else if let Ok(n) = line[..idx].trim().parse::<usize>() {
                        total += n;
                    }
                }

                // Parse "X error"
                if let Some(idx) = line.find(" error")
                    && let Some(start) = line[..idx]
                        .rfind(|c: char| !c.is_ascii_digit())
                        .map(|i| i + 1)
                    && let Ok(n) = line[start..idx].trim().parse::<usize>()
                {
                    total += n;
                }
            }
        }

        // If we couldn't parse, try counting individual test results
        if total == 0 {
            for line in output.lines() {
                if line.contains("PASSED") {
                    passed += 1;
                    total += 1;
                } else if line.contains("FAILED") || line.contains("ERROR") {
                    total += 1;
                }
            }
        }

        (total, passed)
    }

    /// Extracts failure locations from pytest output.
    fn extract_failure_locations(&self, output: &str) -> Vec<LineRef> {
        let mut refs = Vec::new();

        for line in output.lines() {
            // Match patterns like "test_file.py:42: AssertionError"
            if let Some(colon_idx) = line.find(':')
                && let Some(second_colon) = line[colon_idx + 1..].find(':')
                && line[..colon_idx].ends_with(".py")
                && let Ok(line_num) =
                    line[colon_idx + 1..colon_idx + 1 + second_colon].parse::<usize>()
            {
                refs.push(LineRef {
                    file_name:   line[..colon_idx].to_string(),
                    line_number: line_num,
                });
            }
        }

        refs
    }
}
