use anyhow::{Context, Result, ensure};
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use owo_colors::OwoColorize;
use similar::{Algorithm, ChangeTag, utils::diff_unicode_words};

use super::results::{Grade, GradeResult};
use crate::{
    config,
    java::{JavaFileError, Project, grade::LineRef},
    retrieval::build_context_message,
};

/// Filters line references down to files that exist in the discovered project.
fn filter_known_refs<T>(project: &Project, refs: Vec<T>) -> Vec<LineRef>
where
    T: Into<LineRef>,
{
    refs.into_iter()
        .map(Into::into)
        .filter(|line_ref| project.contains(line_ref.file_name()))
        .collect()
}

#[derive(Debug, Clone)]
/// Represents a single diff test case pairing optional stdin with an expected
/// output.
pub struct DiffCase {
    /// Optional stdin provided to the student's program.
    pub input:    Option<String>,
    /// Expected stdout/stderr from the program execution.
    pub expected: String,
}

impl DiffCase {
    /// Creates a diff case with only expected output.
    pub fn new(expected: impl Into<String>) -> Self {
        Self {
            input:    None,
            expected: expected.into(),
        }
    }

    /// Attaches stdin to the diff case.
    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.input = Some(input.into());
        self
    }
}
#[derive(Clone, Default)]
/// string. Any difference results in a `0` grade.
/// A grader that grades by diffing an `expected` string with an `actual`
pub struct DiffGrader {
    /// name of requirement
    pub req_name:    String,
    /// points to give if all tests pass
    pub out_of:      f64,
    /// the project to grade
    pub project:     Project,
    /// Java file to run
    pub file:        String,
    /// Diff cases pairing optional stdin with expected output.
    pub cases:       Vec<DiffCase>,
    /// ignore case when comparing
    pub ignore_case: bool,
}

impl DiffGrader {
    /// creates a new DiffGrader
    pub fn new() -> Self {
        Self::default()
    }

    /// gets the `req_name` field
    pub fn req_name(&self) -> String {
        self.req_name.clone()
    }

    /// sets the `req_name` field
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// gets the `out_of` field
    pub fn out_of(&self) -> f64 {
        self.out_of
    }

    /// sets the `out_of` field
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// Returns the configured diff cases.
    pub fn cases(&self) -> &[DiffCase] {
        self.cases.as_ref()
    }

    /// Replaces diff cases with the provided collection.
    pub fn set_cases<I>(mut self, cases: I) -> Self
    where
        I: Into<Vec<DiffCase>>,
    {
        self.cases = cases.into();
        self
    }

    /// Appends a diff case to the grader configuration.
    pub fn add_case(mut self, case: DiffCase) -> Self {
        self.cases.push(case);
        self
    }

    /// gets the `project` field
    pub fn project(&self) -> Project {
        self.project.clone()
    }

    /// sets the `project` field
    pub fn set_project(mut self, project: Project) -> Self {
        self.project = project;
        self
    }

    /// gets the `file` field
    pub fn file(&self) -> String {
        self.file.clone()
    }

    /// sets the `file` field
    pub fn set_file(mut self, file: String) -> Self {
        self.file = file;
        self
    }

    /// gets the `ignore_case` field
    pub fn ignore_case(&self) -> bool {
        self.ignore_case
    }

    /// sets the `ignore_case` field
    pub fn set_ignore_case(mut self, ignore_case: bool) -> Self {
        self.ignore_case = ignore_case;
        self
    }

    /// Grades by diffing the `expected` and `actual` strings.
    pub fn grade_by_diff(&self) -> Result<GradeResult> {
        ensure!(
            !self.cases.is_empty(),
            "At least one diff case (input-expected pair) must be provided"
        );

        let file = self.project.identify(&self.file)?;
        let prompt_set = config::prompts();
        let mut prompt_sections = Vec::new();
        let mut first_failure_reason = None;

        let preview = |text: &str| -> String {
            let snippet = text.trim();
            let first_line = snippet.lines().next().unwrap_or("");
            let mut head = first_line.chars().take(80).collect::<String>();
            if first_line.chars().count() > head.chars().count() {
                head.push('…');
            }
            if head.is_empty() {
                "[empty]".to_string()
            } else {
                head
            }
        };

        for case in &self.cases {
            let expected = if self.ignore_case {
                case.expected.to_lowercase().trim().to_string()
            } else {
                case.expected.trim().to_string()
            };
            let input = case.input.clone();

            let actual_out = {
                let out = match file.run_with_input(input.clone()) {
                    Ok(out) => out,
                    Err(JavaFileError::AtRuntime { output, diags }) => {
                        let resolved_diags = filter_known_refs(&self.project, diags);
                        let messages = vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(prompt_set.system_message().to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(format!("Error while running -\n```\n{}\n```", output))
                                .name("Student".to_string())
                                .build()
                                .context("Failed to build user message")?
                                .into(),
                            build_context_message(&self.project, None, resolved_diags)?,
                        ];
                        return Ok(GradeResult {
                            requirement: self.req_name.clone(),
                            grade:       Grade::new(0.0, self.out_of),
                            reason:      "Error running file for some cases.".to_string(),
                            prompt:      Some(messages),
                        });
                    }
                    Err(JavaFileError::DuringCompilation { stacktrace, diags }) => {
                        let resolved_diags = filter_known_refs(&self.project, diags);
                        let messages = vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(prompt_set.system_message().to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(format!(
                                    "Error while compiling -\n```\n{}\n```",
                                    stacktrace
                                ))
                                .name("Student".to_string())
                                .build()
                                .context("Failed to build user message")?
                                .into(),
                            build_context_message(&self.project, None, resolved_diags)?,
                        ];
                        return Ok(GradeResult {
                            requirement: self.req_name.clone(),
                            grade:       Grade::new(0.0, self.out_of),
                            reason:      "Error compiling file for some cases.".to_string(),
                            prompt:      Some(messages),
                        });
                    }
                    Err(e) => {
                        let messages = vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(prompt_set.system_message().to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(format!("Unknown error -\n```\n{:?}\n```", e))
                                .name("Student".to_string())
                                .build()
                                .context("Failed to build user message")?
                                .into(),
                        ];
                        return Ok(GradeResult {
                            requirement: self.req_name.clone(),
                            grade:       Grade::new(0.0, self.out_of),
                            reason:      "Unknown error while running file for some cases."
                                .to_string(),
                            prompt:      Some(messages),
                        });
                    }
                };

                if self.ignore_case {
                    out.to_lowercase().trim().to_string()
                } else {
                    out.trim().to_string()
                }
            };

            let diff = diff_unicode_words(Algorithm::Patience, &expected, &actual_out);

            let mut is_equal = true;
            let mut colored_expected = String::new();
            let mut colored_actual = String::new();
            let mut plain_expected = String::new();
            let mut plain_actual = String::new();

            for (change, value) in diff {
                match change {
                    ChangeTag::Equal => {
                        colored_expected.push_str(value);
                        colored_actual.push_str(value);
                        plain_expected.push_str(value);
                        plain_actual.push_str(value);
                    }
                    ChangeTag::Insert => {
                        colored_actual.push_str(&format!("{}", value.green()));
                        plain_actual.push_str(value);
                        if !value.trim().is_empty() {
                            is_equal = false;
                        }
                    }
                    ChangeTag::Delete => {
                        colored_expected.push_str(&format!("{}", value.red()));
                        plain_expected.push_str(value);
                        if !value.trim().is_empty() {
                            is_equal = false;
                        }
                    }
                }
            }

            if !is_equal {
                let input_section = match input.as_deref() {
                    Some(value) if !value.is_empty() => format!("\nInput:\n`{}`\n", value),
                    _ => String::new(),
                };

                let console_diff = format!(
                    "Comparing expected and actual output for \
                     {}:\n```{input_section}Expected:\n{}\nActual:\n{}\n```\n",
                    file.file_name(),
                    colored_expected,
                    colored_actual,
                    input_section = input_section,
                );

                let prompt_diff = format!(
                    "Comparing expected and actual output for \
                     {}:\n```{input_section}Expected:\n{}\nActual:\n{}\n```\n",
                    file.file_name(),
                    plain_expected,
                    plain_actual,
                    input_section = input_section,
                );

                eprintln!("{console_diff}");
                prompt_sections.push(prompt_diff);

                if first_failure_reason.is_none() {
                    let expected_preview = preview(&plain_expected);
                    let actual_preview = preview(&plain_actual);
                    let reason = match input.as_deref().filter(|value| !value.is_empty()) {
                        Some(stdin) => format!(
                            "First mismatch for {} (input: `{}`): expected \"{}\"; got \"{}\"",
                            file.file_name(),
                            preview(stdin),
                            expected_preview,
                            actual_preview,
                        ),
                        None => format!(
                            "First mismatch for {}: expected \"{}\"; got \"{}\"",
                            file.file_name(),
                            expected_preview,
                            actual_preview,
                        ),
                    };
                    first_failure_reason = Some(reason);
                }
            }
        }

        if prompt_sections.is_empty() {
            Ok(GradeResult {
                requirement: self.req_name.clone(),
                grade:       Grade {
                    grade:  self.out_of,
                    out_of: self.out_of,
                },
                reason:      "Got expected output".to_string(),
                prompt:      None,
            })
        } else {
            let mut user_content = prompt_sections.join("\n\n");
            user_content.push_str("\n\nSource code:\n```java\n");
            user_content.push_str(file.code());
            user_content.push_str("\n```\n");

            if user_content.len() > config::PROMPT_TRUNCATE {
                user_content.truncate(config::PROMPT_TRUNCATE);
                user_content.push_str("...[TRUNCATED]");
            }

            let retrieval_message =
                build_context_message(&self.project, None, Vec::<LineRef>::new())?;

            let system_message = ChatCompletionRequestSystemMessageArgs::default()
                .content(prompt_set.system_message().to_string())
                .name("Instructor".to_string())
                .build()
                .context("Failed to build system message")?
                .into();

            let user_message = ChatCompletionRequestUserMessageArgs::default()
                .content(user_content)
                .name("Student".to_string())
                .build()
                .context("Failed to build user message")?
                .into();

            Ok(GradeResult {
                requirement: self.req_name.clone(),
                grade:       Grade {
                    grade:  0.0,
                    out_of: self.out_of,
                },
                reason:      first_failure_reason.unwrap_or_else(|| "See above.".to_string()),
                prompt:      Some(vec![system_message, user_message, retrieval_message]),
            })
        }
    }
}
