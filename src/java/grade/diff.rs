#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use anyhow::{Context, Result, ensure};
use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
};
use bon::Builder;
use owo_colors::OwoColorize;
use similar::{Algorithm, ChangeTag, utils::diff_unicode_words};

use super::results::{Grade, GradeResult};
use crate::{
    config,
    java::{File, JavaFileError, Project, grade::LineRef},
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

/// Normalized view of a grader output used for display and comparison.
#[derive(Clone, Debug)]
struct NormalizedOutput {
    /// Content to show in human-readable messages.
    display: String,
    /// Content normalized for diff comparisons.
    compare: String,
}

impl NormalizedOutput {
    /// Returns the human-readable representation.
    fn display(&self) -> &str {
        &self.display
    }

    /// Returns the comparison-friendly representation.
    fn compare(&self) -> &str {
        &self.compare
    }
}

/// Captures the first diff failure encountered while grading.
struct DiffFailure {
    /// Console-friendly diff that preserves colour for stderr.
    console_output: String,
    /// Plain-text diff embedded in prompts.
    prompt_body:    String,
    /// Short description of the mismatch for the grade result.
    reason:         String,
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
#[derive(Clone, Default, Builder)]
#[builder(on(String, into))]
/// string. Any difference results in a `0` grade.
/// A grader that grades by diffing an `expected` string with an `actual`
pub struct DiffGrader {
    /// name of requirement
    #[builder(getter)]
    pub req_name:            String,
    /// points to give if all tests pass
    #[builder(getter)]
    pub out_of:              f64,
    /// the project to grade
    #[builder(getter)]
    pub project:             Project,
    /// Java file to run
    #[builder(getter)]
    pub file:                String,
    /// Diff cases pairing optional stdin with expected output.
    #[builder(
        default,
        with = |iter: impl IntoIterator<
            Item = (impl Into<String>, Option<impl Into<String>>)
        >| iter
            .into_iter()
            .map(|(expected, input)| DiffCase {
                expected: expected.into(),
                input:    input.map(Into::into),
            })
            .collect::<Vec<_>>()
    )]
    #[builder(getter)]
    pub cases:               Vec<DiffCase>,
    /// ignore case when comparing
    #[builder(default)]
    #[builder(getter)]
    pub ignore_case:         bool,
    /// preserve whitespace when comparing
    #[builder(default)]
    #[builder(getter)]
    pub preserve_whitespace: bool,
}

impl DiffGrader {
    /// Adds a single diff case after construction.
    pub fn case(mut self, expected: impl Into<String>, input: Option<impl Into<String>>) -> Self {
        self.cases.push(DiffCase {
            expected: expected.into(),
            input:    input.map(Into::into),
        });
        self
    }

    /// Builds and runs the configured diff grader.
    pub async fn run(self) -> Result<GradeResult> {
        self.grade_by_diff().await
    }

    /// Grades by diffing the `expected` and `actual` strings.
    pub async fn grade_by_diff(&self) -> Result<GradeResult> {
        let file = self.resolve_target()?;
        let prompts = config::java_prompts();

        for case in &self.cases {
            let expected = self.normalize_expected(case);
            let input = case.input.clone();

            let actual_raw = match file.run_with_input(input.clone()).await {
                Ok(out) => out,
                Err(JavaFileError::AtRuntime { output, diags }) => {
                    return self.execution_failure(
                        &prompts,
                        "Error running file for some cases.",
                        format!("Error while running -\n```\n{}\n```", output),
                        Some(filter_known_refs(&self.project, diags)),
                    );
                }
                Err(JavaFileError::DuringCompilation { stacktrace, diags }) => {
                    return self.execution_failure(
                        &prompts,
                        "Error compiling file for some cases.",
                        format!("Error while compiling -\n```\n{}\n```", stacktrace),
                        Some(filter_known_refs(&self.project, diags)),
                    );
                }
                Err(e) => {
                    return self.execution_failure(
                        &prompts,
                        "Unknown error while running file for some cases.",
                        format!("Unknown error -\n```\n{:?}\n```", e),
                        None,
                    );
                }
            };

            let actual = self.normalize_actual(actual_raw);
            if let Some(failure) = self.compare_outputs(&file, &expected, &actual, input.as_deref())
            {
                eprintln!("{}", failure.console_output);
                return self.build_prompt_payload(&file, &prompts, failure);
            }
        }

        Ok(GradeResult::builder()
            .requirement(self.req_name.clone())
            .grade(Grade::new(self.out_of, self.out_of))
            .reason("Got expected output")
            .maybe_prompt(None)
            .build())
    }

    /// Builds a failing `GradeResult` with the supplied reason and prompt
    /// messages.
    fn failure_result(
        &self,
        reason: impl Into<String>,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> GradeResult {
        GradeResult::builder()
            .requirement(self.req_name.clone())
            .grade(Grade::new(0.0, self.out_of))
            .reason(reason.into())
            .maybe_prompt(Some(messages))
            .build()
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

    /// Ensures a diff case exists and returns the file under test.
    fn resolve_target(&self) -> Result<File> {
        ensure!(
            !self.cases.is_empty(),
            "At least one diff case (input-expected pair) must be provided"
        );

        self.project.identify(&self.file)
    }

    /// Returns the normalized expectation for a diff case.
    fn normalize_expected(&self, case: &DiffCase) -> NormalizedOutput {
        self.normalize_text(case.expected.clone())
    }

    /// Normalizes student output captured from the subprocess run.
    fn normalize_actual(&self, raw: String) -> NormalizedOutput {
        self.normalize_text(raw)
    }

    /// Applies whitespace/case rules to produce a reusable normalized payload.
    fn normalize_text(&self, text: String) -> NormalizedOutput {
        let display = if self.preserve_whitespace {
            text
        } else {
            text.trim().to_string()
        };

        let compare = if self.ignore_case {
            display.to_lowercase()
        } else {
            display.clone()
        };

        NormalizedOutput { display, compare }
    }

    /// Builds an informative preview string matching the configured mode.
    fn preview(&self, text: &str) -> String {
        if self.preserve_whitespace {
            preview_preserving_whitespace(text)
        } else {
            preview_trimmed(text)
        }
    }

    /// Computes the diff between expected and actual output, returning the
    /// first failure.
    fn compare_outputs(
        &self,
        file: &File,
        expected: &NormalizedOutput,
        actual: &NormalizedOutput,
        input: Option<&str>,
    ) -> Option<DiffFailure> {
        let diff = diff_unicode_words(Algorithm::Patience, expected.compare(), actual.compare());

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

        if is_equal {
            return None;
        }

        let input_section = match input {
            Some(value) if !value.is_empty() => format!("\nInput:\n`{}`\n", value),
            _ => String::new(),
        };

        let console_output = format!(
            "Comparing expected and actual output for \
             {}:\n```{input_section}Expected:\n{}\nActual:\n{}\n```\n",
            file.file_name(),
            colored_expected,
            colored_actual,
            input_section = input_section,
        );

        let prompt_body = format!(
            "Comparing expected and actual output for \
             {}:\n```{input_section}Expected:\n{}\nActual:\n{}\n```\n",
            file.file_name(),
            plain_expected,
            plain_actual,
            input_section = input_section,
        );

        let reason = match input.filter(|value| !value.is_empty()) {
            Some(stdin) => format!(
                "First mismatch for {} (input: `{}`): expected \"{}\"; got \"{}\"",
                file.file_name(),
                self.preview(stdin),
                self.preview(expected.display()),
                self.preview(actual.display()),
            ),
            None => format!(
                "First mismatch for {}: expected \"{}\"; got \"{}\"",
                file.file_name(),
                self.preview(expected.display()),
                self.preview(actual.display()),
            ),
        };

        Some(DiffFailure {
            console_output,
            prompt_body,
            reason,
        })
    }

    /// Converts execution errors into a failing grade result with helpful
    /// context.
    fn execution_failure(
        &self,
        prompts: &crate::java::JavaPrompts,
        reason: &str,
        body: String,
        diags: Option<Vec<LineRef>>,
    ) -> Result<GradeResult> {
        let messages = self.build_error_messages(prompts, body, diags)?;
        Ok(self.failure_result(reason.to_string(), messages))
    }

    /// Assembles the prompt payload for a diff failure and returns the grade
    /// result.
    fn build_prompt_payload(
        &self,
        file: &File,
        prompts: &crate::java::JavaPrompts,
        failure: DiffFailure,
    ) -> Result<GradeResult> {
        let mut user_content = failure.prompt_body;
        let mut appended_note = false;

        if user_content.len() < config::PROMPT_TRUNCATE {
            let code_block = format!("\n\nSource code:\n```java\n{}\n```\n", file.code());
            if user_content.len() + code_block.len() <= config::PROMPT_TRUNCATE {
                user_content.push_str(&code_block);
                appended_note = true;
            } else {
                let note = format!("\n\nSource code omitted; refer to {}.", file.path().display());
                if user_content.len() + note.len() <= config::PROMPT_TRUNCATE {
                    user_content.push_str(&note);
                    appended_note = true;
                }
            }
        }

        if user_content.len() > config::PROMPT_TRUNCATE && !appended_note {
            user_content = truncate_with_notice(&user_content, config::PROMPT_TRUNCATE);
        }

        let retrieval_message = build_context_message(&self.project, None, Vec::<LineRef>::new())?;

        let system_message = ChatCompletionRequestSystemMessageArgs::default()
            .content(prompts.system_message().to_string())
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

        Ok(GradeResult::builder()
            .requirement(self.req_name.clone())
            .grade(Grade::new(0.0, self.out_of))
            .reason(failure.reason)
            .maybe_prompt(Some(vec![system_message, user_message, retrieval_message]))
            .build())
    }
}

impl<S> DiffGraderBuilder<S>
where
    S: diff_grader_builder::IsComplete,
{
    /// Build the grader and immediately execute it.
    pub async fn run(self) -> Result<GradeResult> {
        self.build().run().await
    }
}
