use anyhow::{Context, Result, ensure};
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
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

/// Truncates `content` to the provided `limit`, appending a notice to indicate
/// omitted output.
fn truncate_with_notice(content: &str, limit: usize) -> String {
    if content.len() <= limit {
        return content.to_string();
    }

    let mut end = limit;
    while end > 0 && !content.is_char_boundary(end) {
        end -= 1;
    }

    let mut truncated = content[..end].to_string();
    if let Some(index) = truncated.rfind('\n') {
        truncated.truncate(index);
    }

    truncated.push_str("\n...[TRUNCATED]");
    truncated
}

/// Produces a short preview by trimming surrounding whitespace and capturing
/// the first non-empty line.
fn preview_trimmed(text: &str) -> String {
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
}

/// Produces a preview while visualizing whitespace so mismatches stand out in
/// grader explanations.
fn preview_preserving_whitespace(text: &str) -> String {
    let normalized = text
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace(' ', "␠");

    let first_line = normalized.lines().next().unwrap_or("");
    let mut head = first_line.chars().take(80).collect::<String>();
    if first_line.chars().count() > head.chars().count() {
        head.push('…');
    }
    if head.is_empty() {
        "[empty]".to_string()
    } else {
        head
    }
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
    pub req_name:            String,
    /// points to give if all tests pass
    pub out_of:              f64,
    /// the project to grade
    pub project:             Project,
    /// Java file to run
    pub file:                String,
    /// Diff cases pairing optional stdin with expected output.
    pub cases:               Vec<DiffCase>,
    /// ignore case when comparing
    pub ignore_case:         bool,
    /// preserve whitespace when comparing
    pub preserve_whitespace: bool,
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

    /// sets the `preserve_whitespace` field
    pub fn set_preserve_whitespace(mut self, preserve_whitespace: bool) -> Self {
        self.preserve_whitespace = preserve_whitespace;
        self
    }

    /// Grades by diffing the `expected` and `actual` strings.
    pub async fn grade_by_diff(&self) -> Result<GradeResult> {
        ensure!(
            !self.cases.is_empty(),
            "At least one diff case (input-expected pair) must be provided"
        );

        let file = self.project.identify(&self.file)?;
        let prompt_set = config::java_prompts();
        let mut prompt_section = None;
        let mut first_failure_reason = None;

        let preview = |text: &str| -> String {
            if self.preserve_whitespace {
                preview_preserving_whitespace(text)
            } else {
                preview_trimmed(text)
            }
        };

        for case in &self.cases {
            let expected_display = if self.preserve_whitespace {
                case.expected.clone()
            } else {
                case.expected.trim().to_string()
            };
            let expected_compare = if self.ignore_case {
                expected_display.to_lowercase()
            } else {
                expected_display.clone()
            };
            let input = case.input.clone();

            let actual_raw = match file.run_with_input(input.clone()).await {
                Ok(out) => out,
                Err(JavaFileError::AtRuntime { output, diags }) => {
                    let resolved_diags = filter_known_refs(&self.project, diags);
                    let messages = self.build_error_messages(
                        &prompt_set,
                        format!("Error while running -\n```\n{}\n```", output),
                        Some(resolved_diags),
                    )?;
                    return Ok(self.failure_result("Error running file for some cases.", messages));
                }
                Err(JavaFileError::DuringCompilation { stacktrace, diags }) => {
                    let resolved_diags = filter_known_refs(&self.project, diags);
                    let messages = self.build_error_messages(
                        &prompt_set,
                        format!("Error while compiling -\n```\n{}\n```", stacktrace),
                        Some(resolved_diags),
                    )?;
                    return Ok(
                        self.failure_result("Error compiling file for some cases.", messages)
                    );
                }
                Err(e) => {
                    let messages = self.build_error_messages(
                        &prompt_set,
                        format!("Unknown error -\n```\n{:?}\n```", e),
                        None,
                    )?;
                    return Ok(self.failure_result(
                        "Unknown error while running file for some cases.",
                        messages,
                    ));
                }
            };

            let actual_display = if self.preserve_whitespace {
                actual_raw.clone()
            } else {
                actual_raw.trim().to_string()
            };
            let actual_compare = if self.ignore_case {
                actual_display.to_lowercase()
            } else {
                actual_display.clone()
            };

            let diff = diff_unicode_words(Algorithm::Patience, &expected_compare, &actual_compare);

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
                        if self.preserve_whitespace || !value.trim().is_empty() {
                            is_equal = false;
                        }
                    }
                    ChangeTag::Delete => {
                        colored_expected.push_str(&format!("{}", value.red()));
                        plain_expected.push_str(value);
                        if self.preserve_whitespace || !value.trim().is_empty() {
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
                prompt_section.get_or_insert(prompt_diff);

                if first_failure_reason.is_none() {
                    let expected_preview = preview(&expected_display);
                    let actual_preview = preview(&actual_display);
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

                break;
            }
        }

        if let Some(mut user_content) = prompt_section {
            let mut appended_note = false;

            if user_content.len() < config::PROMPT_TRUNCATE {
                let code_block = format!("\n\nSource code:\n```java\n{}\n```\n", file.code());
                if user_content.len() + code_block.len() <= config::PROMPT_TRUNCATE {
                    user_content.push_str(&code_block);
                    appended_note = true;
                } else {
                    let note =
                        format!("\n\nSource code omitted; refer to {}.", file.path().display());
                    if user_content.len() + note.len() <= config::PROMPT_TRUNCATE {
                        user_content.push_str(&note);
                        appended_note = true;
                    }
                }
            }

            if user_content.len() > config::PROMPT_TRUNCATE && !appended_note {
                user_content = truncate_with_notice(&user_content, config::PROMPT_TRUNCATE);
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
        } else {
            Ok(GradeResult {
                requirement: self.req_name.clone(),
                grade:       Grade {
                    grade:  self.out_of,
                    out_of: self.out_of,
                },
                reason:      "Got expected output".to_string(),
                prompt:      None,
            })
        }
    }

    /// Builds a failing `GradeResult` with the supplied reason and prompt
    /// messages.
    fn failure_result(
        &self,
        reason: impl Into<String>,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> GradeResult {
        GradeResult {
            requirement: self.req_name.clone(),
            grade:       Grade::new(0.0, self.out_of),
            reason:      reason.into(),
            prompt:      Some(messages),
        }
    }

    /// Renders the instructor/student messages used when diff grading fails.
    fn build_error_messages(
        &self,
        prompts: &crate::java::JavaPrompts,
        body: String,
        diags: Option<Vec<LineRef>>,
    ) -> Result<Vec<ChatCompletionRequestMessage>> {
        let mut messages = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(prompts.system_message().to_string())
                .name("Instructor".to_string())
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(body)
                .name("Student".to_string())
                .build()?
                .into(),
        ];

        if let Some(diags) = diags {
            messages.push(build_context_message(&self.project, None, diags)?);
        }

        Ok(messages)
    }
}
