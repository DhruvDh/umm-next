#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Diff-based grading utilities for Python.

use anyhow::{Result, ensure};
use async_openai::types::chat::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use bon::Builder;
use similar::{ChangeTag, TextDiff};

use super::results::{Grade, GradeResult};
use crate::{config, python::Project};

/// Represents a single diff test case with optional stdin.
#[derive(Debug, Clone)]
pub struct DiffCase {
    /// Expected output.
    expected: String,
    /// Optional stdin input.
    input:    Option<String>,
}

impl DiffCase {
    /// Creates a new diff case with expected output.
    pub fn new(expected: impl Into<String>) -> Self {
        Self {
            expected: expected.into(),
            input:    None,
        }
    }

    /// Attaches stdin input to the diff case.
    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.input = Some(input.into());
        self
    }
}

/// A grader that compares expected output with actual output.
#[derive(Clone, Default, Builder)]
#[builder(on(String, into))]
pub struct DiffGrader {
    /// The project being graded.
    #[builder(getter)]
    project:             Project,
    /// Name of the file to run.
    #[builder(getter)]
    file:                String,
    /// Test cases to run.
    #[builder(default, with = |cases: impl IntoIterator<Item = DiffCase>| cases.into_iter().collect())]
    #[builder(getter)]
    cases:               Vec<DiffCase>,
    /// Whether to ignore case differences.
    #[builder(default = false)]
    #[builder(getter)]
    ignore_case:         bool,
    /// Whether to preserve whitespace in comparison.
    #[builder(default = false)]
    #[builder(getter)]
    preserve_whitespace: bool,
    /// Requirement name for reporting.
    #[builder(getter)]
    req_name:            String,
    /// Total points available.
    #[builder(getter)]
    out_of:              f64,
}

impl DiffGrader {
    /// Adds a test case.
    pub fn case(mut self, expected: impl Into<String>, input: Option<impl Into<String>>) -> Self {
        let mut case = DiffCase::new(expected);
        if let Some(inp) = input {
            case = case.with_input(inp);
        }
        self.cases.push(case);
        self
    }

    /// Builds and runs the grader.
    pub async fn run(self) -> Result<GradeResult> {
        ensure!(!self.cases.is_empty(), "DiffGrader requires at least one test case");
        self.grade_by_diff().await
    }

    /// Performs the diff grading.
    async fn grade_by_diff(&self) -> Result<GradeResult> {
        let file = self.project.identify(&self.file)?;
        let prompts = config::python_prompts();

        let mut all_passed = true;
        let mut reasons = Vec::new();
        let mut messages = Vec::new();

        for (idx, case) in self.cases.iter().enumerate() {
            let case_num = idx + 1;

            match file.run(case.input.clone()).await {
                Ok(actual) => {
                    let expected = self.normalize(&case.expected);
                    let actual_normalized = self.normalize(&actual);

                    if expected == actual_normalized {
                        reasons.push(format!("Case {}: PASSED", case_num));
                    } else {
                        all_passed = false;
                        let diff = self.format_diff(&case.expected, &actual);
                        reasons.push(format!("Case {}: FAILED\n{}", case_num, diff));

                        messages.push(
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(format!(
                                    "Test case {} \
                                     failed.\n\nExpected:\n```\n{}\n```\n\nActual:\n```\n{}\n```\\
                                     n\nDiff:\n```\n{}\n```",
                                    case_num, case.expected, actual, diff
                                ))
                                .name("Student".to_string())
                                .build()?
                                .into(),
                        );
                    }
                }
                Err(e) => {
                    all_passed = false;
                    let error_msg = format!("{}", e);
                    reasons.push(format!("Case {}: ERROR\n{}", case_num, error_msg));

                    messages.push(
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(format!(
                                "Test case {} resulted in an error:\n```\n{}\n```",
                                case_num, error_msg
                            ))
                            .name("Student".to_string())
                            .build()?
                            .into(),
                    );
                }
            }
        }

        let grade = if all_passed { self.out_of } else { 0.0 };
        let reason = reasons.join("\n\n");

        let prompt = if !messages.is_empty() {
            let mut full_messages = vec![
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(prompts.system_message().to_string())
                    .name("Instructor".to_string())
                    .build()?
                    .into(),
            ];
            full_messages.extend(messages);
            Some(full_messages)
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

    /// Normalizes a string for comparison.
    fn normalize(&self, s: &str) -> String {
        let mut result = s.to_string();

        if !self.preserve_whitespace {
            // Normalize line endings
            result = result.replace("\r\n", "\n");
            // Trim trailing whitespace from each line
            result = result
                .lines()
                .map(|l| l.trim_end())
                .collect::<Vec<_>>()
                .join("\n");
            // Trim overall
            result = result.trim().to_string();
        }

        if self.ignore_case {
            result = result.to_lowercase();
        }

        result
    }

    /// Formats a diff between expected and actual output.
    fn format_diff(&self, expected: &str, actual: &str) -> String {
        let diff = TextDiff::from_lines(expected, actual);
        let mut output = String::new();

        for change in diff.iter_all_changes() {
            let prefix = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            output.push_str(&format!("{} {}", prefix, change));
        }

        output
    }
}
