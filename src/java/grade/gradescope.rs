#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{collections::HashSet, fs, io::Write};

use anyhow::{Context, Result};
use async_openai::{
    Client as OpenAIClient,
    config::OpenAIConfig,
    error::OpenAIError,
    types::chat::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequest, CreateChatCompletionResponse,
    },
};
use bon::Builder;
use serde::{Deserialize, Serialize};
use serde_json;
use tabled::{
    Table,
    settings::{Alignment, Modify, Panel, Style, Width, object::Rows},
};
use tokio::{runtime::Runtime, task::block_in_place};

use super::{feedback::generate_single_feedback, results::GradeResult};
use crate::{
    config::{self, OpenAiEnv},
    java::{File, Project},
};

/// Configuration options that control how Gradescope output is rendered.
#[derive(Debug, Clone, Builder)]
#[builder(on(String, into))]
pub struct GradescopeConfig {
    /// Source files to include when generating SLO summaries.
    #[builder(default, with = |iter: impl IntoIterator<Item = impl Into<String>>| {
        iter.into_iter().map(Into::into).collect::<Vec<String>>()
    })]
    pub source_files:        Vec<String>,
    /// Test files to include when generating SLO summaries.
    #[builder(default, with = |iter: impl IntoIterator<Item = impl Into<String>>| {
        iter.into_iter().map(Into::into).collect::<Vec<String>>()
    })]
    pub test_files:          Vec<String>,
    /// Title displayed alongside SLO feedback.
    #[builder(default)]
    pub project_title:       String,
    /// Description displayed alongside SLO feedback.
    #[builder(default)]
    pub project_description: String,
    /// Threshold (0.0-1.0) for marking test cases as passing.
    #[builder(default = 0.7)]
    pub pass_threshold:      f64,
    /// Whether to emit a textual overview table to stderr.
    #[builder(default = true)]
    pub show_table:          bool,
    /// Whether to emit the Gradescope JSON artifact.
    #[builder(default)]
    pub results_json:        bool,
    /// Whether to post per-test feedback via Supabase.
    #[builder(default)]
    pub feedback:            bool,
    /// Whether to write the Gradescope JSON to the local workspace for
    /// debugging.
    #[builder(default)]
    pub debug:               bool,
    /// Set of enabled SLO identifiers that should generate feedback.
    #[builder(default, with = |iter: impl IntoIterator<Item = impl Into<String>>| {
        iter.into_iter().map(Into::into).collect::<HashSet<String>>()
    })]
    pub enabled_slos:        HashSet<String>,
}

impl Default for GradescopeConfig {
    fn default() -> Self {
        Self {
            source_files:        Vec::new(),
            test_files:          Vec::new(),
            project_title:       String::new(),
            project_description: String::new(),
            pass_threshold:      0.7,
            show_table:          true,
            results_json:        false,
            feedback:            false,
            debug:               false,
            enabled_slos:        HashSet::new(),
        }
    }
}
/// Represents output format settings for Gradescope submissions.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GradescopeOutputFormat {
    /// Plain text format.
    Text,
    /// HTML format.
    Html,
    /// This is very similar to the "html" format option but will also convert
    /// \n into <br /> and \n\n+ into a page break.
    SimpleFormat,
    /// Markdown format.
    Md,
    /// ANSI format for including ANSI escape codes (often used in terminal
    /// outputs).
    Ansi,
}

/// Represents visibility settings for Gradescope submissions and test cases.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GradescopeVisibility {
    /// Hidden from students.
    Hidden,
    /// Visible after the due date of the assignment.
    AfterDueDate,
    /// Visible after the grades are published.
    AfterPublished,
    /// Always visible to students.
    Visible,
}

/// Represents the status of a test case in Gradescope submissions.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GradescopeStatus {
    /// Indicates the test case passed successfully.
    Passed,
    /// Indicates the test case failed.
    Failed,
}

/// Represents the overall submission data.
#[derive(Serialize, Deserialize, Debug, Builder)]
#[builder(on(String, into))]
pub struct GradescopeSubmission {
    /// Optional overall score. Overrides total of test cases if specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub score: Option<f64>,

    /// Optional execution time in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub execution_time: Option<u32>,

    /// Optional text relevant to the entire submission.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub output: Option<String>,

    /// Optional output format settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub output_format: Option<GradescopeOutputFormat>,

    /// Optional default output format for test case outputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub test_output_format: Option<GradescopeOutputFormat>,

    /// Optional default output format for test case names.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub test_name_format: Option<GradescopeOutputFormat>,

    /// Optional visibility setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub visibility: Option<GradescopeVisibility>,

    /// Optional stdout visibility setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub stdout_visibility: Option<GradescopeVisibility>,

    /// Optional extra data to be stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub extra_data: Option<serde_json::Value>,

    /// Optional test cases.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    #[builder(with = FromIterator::from_iter)]
    pub tests: Option<Vec<GradescopeTestCase>>,

    /// Optional leaderboard setup.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    #[builder(with = FromIterator::from_iter)]
    pub leaderboard: Option<Vec<GradescopeLeaderboardEntry>>,
}

/// Represents an individual test case.
#[derive(Serialize, Deserialize, Debug, Builder)]
#[builder(on(String, into))]
pub struct GradescopeTestCase {
    /// Optional score for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub score: Option<f64>,

    /// Optional maximum score for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub max_score: Option<f64>,

    /// Optional status of the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub status: Option<GradescopeStatus>,

    /// Optional name of the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub name: Option<String>,

    /// Optional formatting for the test case name.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub name_format: Option<GradescopeOutputFormat>,

    /// Optional number for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub number: Option<String>,

    /// Optional detailed output for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub output: Option<String>,

    /// Optional formatting for the test case output.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub output_format: Option<GradescopeOutputFormat>,

    /// Optional tags associated with the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    #[builder(with = FromIterator::from_iter)]
    pub tags: Option<Vec<String>>,

    /// Optional visibility setting for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub visibility: Option<GradescopeVisibility>,

    /// Optional extra data to be stored with the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub extra_data: Option<serde_json::Value>,
}

/// Represents an entry in the leaderboard.
#[derive(Serialize, Deserialize, Debug, Builder)]
#[builder(on(String, into))]
pub struct GradescopeLeaderboardEntry {
    /// Name of the leaderboard metric.
    #[builder(getter)]
    pub name: String,

    /// Value of the leaderboard metric.
    #[builder(getter)]
    pub value: String,

    /// Optional ordering for the leaderboard metric.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(getter)]
    pub order: Option<String>,
}

/// What kind of file the SLO is for.
#[derive(Debug)]
enum SLOFileType {
    /// Only source files.
    Source,
    /// Only test files.
    Test,
    /// Both source and test files.
    SourceAndTest,
}

/// Non-fatal warning captured while assembling a Gradescope artifact.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct GradescopeWarning {
    /// Stable warning code that callers can parse.
    code:    String,
    /// Scope for the warning (requirement name, SLO key, or submission-level).
    scope:   String,
    /// Human-readable warning details.
    message: String,
}

/// Builds a warning payload and appends it to the warning list.
fn record_warning(
    warnings: &mut Vec<GradescopeWarning>,
    code: &str,
    scope: impl Into<String>,
    message: impl Into<String>,
) -> GradescopeWarning {
    let warning = GradescopeWarning {
        code:    code.to_string(),
        scope:   scope.into(),
        message: message.into(),
    };
    eprintln!("Gradescope warning [{}:{}] {}", warning.code, warning.scope, warning.message);
    warnings.push(warning.clone());
    warning
}

/// Renders a single warning line suitable for test-case markdown output.
fn warning_line(warning: &GradescopeWarning) -> String {
    format!("[warning:{}:{}] {}", warning.code, warning.scope, warning.message)
}

/// Renders a submission-level warning summary in markdown.
fn warning_summary(warnings: &[GradescopeWarning]) -> String {
    let mut lines = vec!["Autograder completed with warnings:".to_string()];
    for warning in warnings {
        lines.push(format!("- {} (`{}::{}`)", warning.message, warning.code, warning.scope));
    }
    lines.join("\n")
}

/// Renders per-SLO feedback that can be used directly or as fallback when
/// combined report synthesis fails.
fn render_individual_slo_feedback(
    slo_responses: &[(&str, Result<CreateChatCompletionResponse, OpenAIError>)],
) -> String {
    let mut individual_feedbacks = Vec::new();

    for (name, resp) in slo_responses {
        match resp {
            Ok(response) => {
                let content = response
                    .choices
                    .first()
                    .and_then(|choice| choice.message.content.clone())
                    .unwrap_or_default();
                individual_feedbacks.push(format!("SLO: {}\n\n{}", name, content));
            }
            Err(e) => {
                eprintln!("Error processing SLO '{}': {:?}", name, e);
                individual_feedbacks
                    .push(format!("SLO: {}\n\nError: Unable to process this SLO.", name));
            }
        }
    }

    individual_feedbacks.join("\n\n---\n\n")
}

/// Renders the combined SLO report used in Gradescope artifacts.
async fn generate_combined_slo_report(
    slo_responses: &[(&str, Result<CreateChatCompletionResponse, OpenAIError>)],
    openai: &OpenAiEnv,
) -> Result<String> {
    let combined_feedback = render_individual_slo_feedback(slo_responses);

    let openai_client = OpenAIClient::with_config(
        OpenAIConfig::new()
            .with_api_base(openai.api_base().to_owned())
            .with_api_key(openai.get_api_key().to_owned()),
    );

    let messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(
                "You are an AI assistant tasked with creating a concise, well-structured report \
                 that combines feedback from multiple Student Learning Outcomes (SLOs). Your goal \
                 is to provide a comprehensive overview of the student's performance across all \
                 SLOs, highlighting strengths, areas for improvement, and specific \
                 recommendations.",
            )
            .name("Instructor")
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(format!(
                "Please create a combined report based on the following individual SLO \
                 feedbacks:\n\n{}",
                combined_feedback
            ))
            .name("Student")
            .build()?
            .into(),
    ];

    let response = openai_client
        .chat()
        .create(CreateChatCompletionRequest {
            model: openai.get_model().to_owned(),
            messages,
            temperature: openai.temperature(),
            top_p: openai.top_p(),
            n: Some(1),
            stream: Some(false),
            reasoning_effort: Some(openai.reasoning_effort()),
            ..Default::default()
        })
        .await?;

    response
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone())
        .ok_or_else(|| anyhow::anyhow!("No content in OpenAI response"))
}

/// Generates SLO responses for a given project.
///
/// # Arguments
///
/// * `project` - The project for which to generate SLO responses.
/// * `source_files` - A list of source files in the project.
/// * `test_files` - A list of test files in the project.
/// * `project_title` - The title of the project.
/// * `project_description` - A description of the project.
///
/// # Returns
///
/// A vector of tuples containing the SLO name and the result of the SLO
/// response.
async fn generate_slo_responses(
    project: &Project,
    source_files: &[String],
    test_files: &[String],
    project_title: &str,
    project_description: &str,
    enabled_slos: &HashSet<String>,
    openai: &OpenAiEnv,
) -> Result<(
    Vec<(&'static str, Result<CreateChatCompletionResponse, OpenAIError>)>,
    Vec<GradescopeWarning>,
)> {
    let prompts = config::java_prompts();
    let slos = vec![
        (
            "slo_algorithmic_solutions",
            "Algorithmic Solutions",
            prompts.algorithmic_solutions_slo(),
            SLOFileType::Source,
        ),
        (
            "slo_code_readability",
            "Code Readability and Formatting",
            prompts.code_readability_slo(),
            SLOFileType::SourceAndTest,
        ),
        (
            "slo_comments",
            "Comments",
            prompts.comments_written_slo(),
            SLOFileType::SourceAndTest,
        ),
        (
            "slo_error_handling",
            "Error Handling",
            prompts.error_handling_slo(),
            SLOFileType::SourceAndTest,
        ),
        ("slo_logic", "Logic", prompts.logic_slo(), SLOFileType::SourceAndTest),
        (
            "slo_naming_conventions",
            "Naming Conventions",
            prompts.naming_conventions_slo(),
            SLOFileType::SourceAndTest,
        ),
        (
            "slo_oop_programming",
            "Object Oriented Programming",
            prompts.object_oriented_programming_slo(),
            SLOFileType::SourceAndTest,
        ),
        ("slo_syntax", "Syntax", prompts.syntax_slo(), SLOFileType::SourceAndTest),
        ("slo_testing", "Testing", prompts.testing_slo(), SLOFileType::Test),
    ];

    let mut slo_requests = Vec::new();
    let mut warnings = Vec::new();

    for (slo_key, slo_name, slo_system_message, slo_file_type) in slos {
        if !enabled_slos.contains(slo_key) {
            continue;
        }

        let relevant_files: Vec<File> = match slo_file_type {
            SLOFileType::Source => source_files
                .iter()
                .filter_map(|x| project.identify(x).ok())
                .collect(),
            SLOFileType::Test => test_files
                .iter()
                .filter_map(|x| project.identify(x).ok())
                .collect(),
            SLOFileType::SourceAndTest => source_files
                .iter()
                .chain(test_files.iter())
                .filter_map(|x| project.identify(x).ok())
                .collect(),
        };

        let relevant_file_codes: Vec<String> = relevant_files
            .iter()
            .map(|x| x.code().to_string())
            .collect();

        if relevant_file_codes.is_empty() {
            record_warning(
                &mut warnings,
                "SLO_NO_RELEVANT_FILES",
                slo_key,
                format!(
                    "Skipping SLO '{}' because no relevant source code was found for {:?}",
                    slo_name, slo_file_type
                ),
            );
            continue;
        }

        let mut student_message = vec![format!(
            "# Submission for {project_title}\n\nDescription: {project_description}"
        )];

        for (file, code) in relevant_files.iter().zip(relevant_file_codes.iter()) {
            student_message.push(format!(
                "\n\n## Contents of {file_name}\n\n```java\n{code}\n```",
                file_name = file.proper_name(),
                code = code
            ));
        }

        let student_message = student_message.join("\n\n");
        let messages = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(slo_system_message.to_string())
                .name("Instructor".to_string())
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(student_message)
                .name("Student".to_string())
                .build()?
                .into(),
        ];

        let openai_config = openai.clone();
        slo_requests.push(async move {
            let openai_client = OpenAIClient::with_config(
                OpenAIConfig::new()
                    .with_api_base(openai_config.api_base().to_owned())
                    .with_api_key(openai_config.get_api_key().to_owned()),
            );

            let response = openai_client
                .chat()
                .create(CreateChatCompletionRequest {
                    model: openai_config.get_model().to_owned(),
                    messages: messages.clone(),
                    temperature: openai_config.temperature(),
                    top_p: openai_config.top_p(),
                    n: Some(1),
                    stream: Some(false),
                    reasoning_effort: Some(openai_config.reasoning_effort()),
                    ..Default::default()
                })
                .await;

            (slo_name, response)
        });
    }

    let slo_responses = futures::future::join_all(slo_requests).await;
    Ok((slo_responses, warnings))
}

/// Print grade results to stderr and optionally emit a Gradescope JSON
/// artifact.
///
/// * `results`: collection of requirement-level grades to render.
/// * `config`: strongly typed configuration that replaces the legacy Rhai map.
pub fn show_result(results: Vec<GradeResult>, config: GradescopeConfig) -> Result<()> {
    let show_table = config.show_table;
    let gradescope_json = config.results_json;
    let gradescope_feedback = config.feedback;
    let gradescope_debug = config.debug;
    let pass_threshold = config.pass_threshold;
    let source_files = config.source_files.clone();
    let test_files = config.test_files.clone();
    let project_title = config.project_title.clone();
    let project_description = config.project_description.clone();
    let enabled_slos = config.enabled_slos.clone();

    let (grade, out_of) = results
        .iter()
        .fold((0f64, 0f64), |acc, r| (acc.0 + r.grade_value(), acc.1 + r.out_of_value()));

    if show_table {
        eprintln!(
            "{}",
            Table::new(&results)
                .with(Panel::header("Grading Overview"))
                .with(Panel::footer(format!("Total: {grade:.2}/{out_of:.2}")))
                .with(Modify::new(Rows::new(1..)).with(Width::wrap(24).keep_words(true)))
                .with(
                    Modify::new(Rows::first())
                        .with(Alignment::center())
                        .with(Alignment::center_vertical()),
                )
                .with(
                    Modify::new(Rows::last())
                        .with(Alignment::center())
                        .with(Alignment::center_vertical()),
                )
                .with(Style::modern())
        );
    }

    if gradescope_json {
        let mut warnings = Vec::new();
        let mut test_cases = vec![];

        for result in &results {
            let mut feedback_lines = Vec::new();
            if gradescope_feedback {
                match generate_single_feedback(result) {
                    Ok(feedback) => feedback_lines.push(feedback),
                    Err(err) => {
                        let warning = record_warning(
                            &mut warnings,
                            "SUPABASE_FEEDBACK_FAILED",
                            result.requirement.clone(),
                            format!("Could not generate detailed feedback: {err:#}"),
                        );
                        feedback_lines.push(warning_line(&warning));
                    }
                }
            }

            let test_case = GradescopeTestCase::builder()
                .name(result.requirement.clone())
                .name_format(GradescopeOutputFormat::Text)
                .max_score(result.out_of_value())
                .score(result.grade_value())
                .status(if result.grade_value() > pass_threshold * result.out_of_value() {
                    GradescopeStatus::Passed
                } else {
                    GradescopeStatus::Failed
                })
                .output(feedback_lines.join("\n"))
                .output_format(GradescopeOutputFormat::Md)
                .build();

            test_cases.push(test_case);
        }

        if grade > pass_threshold * out_of && !enabled_slos.is_empty() {
            if project_title.trim().is_empty() {
                record_warning(
                    &mut warnings,
                    "SLO_PROJECT_TITLE_MISSING",
                    "submission",
                    "Skipping SLO feedback because project title is empty",
                );
            } else if project_description.trim().is_empty() {
                record_warning(
                    &mut warnings,
                    "SLO_PROJECT_DESCRIPTION_MISSING",
                    "submission",
                    "Skipping SLO feedback because project description is empty",
                );
            } else if let Some(openai_env) = config::openai_config() {
                match Project::new() {
                    Ok(project) => {
                        let env_ref = &openai_env;
                        let slo_responses = match tokio::runtime::Handle::try_current() {
                            Ok(handle) => block_in_place(|| {
                                handle.block_on(async {
                                    generate_slo_responses(
                                        &project,
                                        &source_files,
                                        &test_files,
                                        &project_title,
                                        &project_description,
                                        &enabled_slos,
                                        env_ref,
                                    )
                                    .await
                                })
                            }),
                            Err(_) => Runtime::new()
                                .context(
                                    "Failed to create Tokio runtime for SLO feedback generation",
                                )?
                                .block_on(async {
                                    generate_slo_responses(
                                        &project,
                                        &source_files,
                                        &test_files,
                                        &project_title,
                                        &project_description,
                                        &enabled_slos,
                                        env_ref,
                                    )
                                    .await
                                }),
                        };

                        match slo_responses {
                            Ok((responses, mut slo_warnings)) => {
                                warnings.append(&mut slo_warnings);

                                if responses.is_empty() {
                                    record_warning(
                                        &mut warnings,
                                        "SLO_NO_RESPONSES",
                                        "submission",
                                        "No SLO prompts were executed; skipping SLO report",
                                    );
                                } else {
                                    let combined_report =
                                        match tokio::runtime::Handle::try_current() {
                                            Ok(handle) => block_in_place(|| {
                                                handle.block_on(async {
                                                    generate_combined_slo_report(
                                                        &responses, env_ref,
                                                    )
                                                    .await
                                                })
                                            }),
                                            Err(_) => Runtime::new()
                                                .context(
                                                    "Failed to create Tokio runtime for SLO \
                                                     report generation",
                                                )?
                                                .block_on(async {
                                                    generate_combined_slo_report(
                                                        &responses, env_ref,
                                                    )
                                                    .await
                                                }),
                                        };

                                    match combined_report {
                                        Ok(report) => test_cases.push(
                                            GradescopeTestCase::builder()
                                                .name(
                                                    "Student Learning Outcomes (SLOs) Feedback"
                                                        .to_string(),
                                                )
                                                .name_format(GradescopeOutputFormat::Text)
                                                .output(report)
                                                .output_format(GradescopeOutputFormat::Md)
                                                .max_score(0f64)
                                                .score(0f64)
                                                .build(),
                                        ),
                                        Err(err) => {
                                            let warning = record_warning(
                                                &mut warnings,
                                                "SLO_COMBINED_REPORT_FAILED",
                                                "submission",
                                                format!(
                                                    "Failed to generate combined SLO report: \
                                                     {err:#}"
                                                ),
                                            );
                                            let fallback =
                                                render_individual_slo_feedback(&responses);
                                            test_cases.push(
                                                GradescopeTestCase::builder()
                                                    .name(
                                                        "Student Learning Outcomes (SLOs) Feedback"
                                                            .to_string(),
                                                    )
                                                    .name_format(GradescopeOutputFormat::Text)
                                                    .output(format!(
                                                        "{}\n\n{}\n\n{}",
                                                        warning_line(&warning),
                                                        "Falling back to individual SLO feedback.",
                                                        fallback
                                                    ))
                                                    .output_format(GradescopeOutputFormat::Md)
                                                    .max_score(0f64)
                                                    .score(0f64)
                                                    .build(),
                                            );
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                record_warning(
                                    &mut warnings,
                                    "SLO_REQUESTS_FAILED",
                                    "submission",
                                    format!("Failed to generate SLO requests: {err:#}"),
                                );
                            }
                        }
                    }
                    Err(err) => {
                        record_warning(
                            &mut warnings,
                            "SLO_PROJECT_DISCOVERY_FAILED",
                            "submission",
                            format!(
                                "Skipping SLO feedback because project discovery failed: {err:#}"
                            ),
                        );
                    }
                }
            } else {
                record_warning(
                    &mut warnings,
                    "SLO_OPENAI_CONFIG_MISSING",
                    "submission",
                    "Skipping SLO feedback because OpenAI configuration is missing",
                );
            }
        }

        let submission_output = if warnings.is_empty() {
            None
        } else {
            Some(warning_summary(&warnings))
        };
        let submission_extra_data = if warnings.is_empty() {
            None
        } else {
            Some(serde_json::json!({ "warnings": warnings }))
        };

        let submission_output_format = submission_output
            .as_ref()
            .map(|_| GradescopeOutputFormat::Md);
        let submission = GradescopeSubmission::builder()
            .tests(test_cases)
            .test_output_format(GradescopeOutputFormat::Md)
            .test_name_format(GradescopeOutputFormat::Text)
            .stdout_visibility(GradescopeVisibility::Visible)
            .visibility(GradescopeVisibility::Visible)
            .maybe_output(submission_output)
            .maybe_output_format(submission_output_format)
            .maybe_extra_data(submission_extra_data)
            .build();

        let mut file = fs::File::create(if gradescope_debug {
            "./results.json"
        } else {
            "/autograder/results/results.json"
        })?;
        file.write_all(serde_json::to_string_pretty(&submission)?.as_bytes())?;
    }

    Ok(())
}
