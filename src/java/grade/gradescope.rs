use std::{collections::HashSet, fs, io::Write};

use anyhow::{Context, Result, ensure};
use async_openai::{
    Client as OpenAIClient,
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequest, CreateChatCompletionResponse,
    },
};
use serde::{Deserialize, Serialize};
use serde_json;
use tabled::{
    Table,
    settings::{Alignment, Modify, Panel, Style, Width, object::Rows},
};
use tokio::{runtime::Runtime, task::block_in_place};
use typed_builder::TypedBuilder;

use super::{feedback::generate_single_feedback, results::GradeResult};
use crate::{
    config::{self, OpenAiEnv},
    java::{File, Project},
};

/// Configuration options that control how Gradescope output is rendered.
#[derive(Debug, Clone)]
pub struct GradescopeConfig {
    /// Source files to include when generating SLO summaries.
    pub source_files:        Vec<String>,
    /// Test files to include when generating SLO summaries.
    pub test_files:          Vec<String>,
    /// Title displayed alongside SLO feedback.
    pub project_title:       String,
    /// Description displayed alongside SLO feedback.
    pub project_description: String,
    /// Threshold (0.0-1.0) for marking test cases as passing.
    pub pass_threshold:      f64,
    /// Whether to emit a textual overview table to stderr.
    pub show_table:          bool,
    /// Whether to emit the Gradescope JSON artifact.
    pub results_json:        bool,
    /// Whether to post per-test feedback via Supabase.
    pub feedback:            bool,
    /// Whether to write the Gradescope JSON to the local workspace for
    /// debugging.
    pub debug:               bool,
    /// Set of enabled SLO identifiers that should generate feedback.
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
#[derive(Serialize, Deserialize, Debug)]
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
#[derive(Serialize, Deserialize, Debug)]
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
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum GradescopeStatus {
    /// Indicates the test case passed successfully.
    Passed,
    /// Indicates the test case failed.
    Failed,
}

/// Represents the overall submission data.
#[derive(Serialize, Deserialize, Debug, TypedBuilder)]
#[builder(field_defaults(default, setter(into)))]
#[builder(doc)]
pub struct GradescopeSubmission {
    /// Optional overall score. Overrides total of test cases if specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,

    /// Optional execution time in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time: Option<u32>,

    /// Optional text relevant to the entire submission.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,

    /// Optional output format settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<GradescopeOutputFormat>,

    /// Optional default output format for test case outputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_output_format: Option<GradescopeOutputFormat>,

    /// Optional default output format for test case names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_name_format: Option<GradescopeOutputFormat>,

    /// Optional visibility setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<GradescopeVisibility>,

    /// Optional stdout visibility setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_visibility: Option<GradescopeVisibility>,

    /// Optional extra data to be stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_data: Option<serde_json::Value>,

    /// Optional test cases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests: Option<Vec<GradescopeTestCase>>,

    /// Optional leaderboard setup.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leaderboard: Option<Vec<GradescopeLeaderboardEntry>>,
}

/// Represents an individual test case.
#[derive(Serialize, Deserialize, Debug, TypedBuilder)]
#[builder(field_defaults(default, setter(into)))]
#[builder(doc)]
pub struct GradescopeTestCase {
    /// Optional score for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,

    /// Optional maximum score for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_score: Option<f64>,

    /// Optional status of the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<GradescopeStatus>,

    /// Optional name of the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional formatting for the test case name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_format: Option<GradescopeOutputFormat>,

    /// Optional number for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,

    /// Optional detailed output for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,

    /// Optional formatting for the test case output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<GradescopeOutputFormat>,

    /// Optional tags associated with the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    /// Optional visibility setting for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<GradescopeVisibility>,

    /// Optional extra data to be stored with the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_data: Option<serde_json::Value>,
}

/// Represents an entry in the leaderboard.
#[derive(Serialize, Deserialize, Debug, TypedBuilder)]
#[builder(field_defaults(default, setter(into)))]
#[builder(doc)]
pub struct GradescopeLeaderboardEntry {
    /// Name of the leaderboard metric.
    pub name: String,

    /// Value of the leaderboard metric.
    pub value: String,

    /// Optional ordering for the leaderboard metric.
    #[serde(skip_serializing_if = "Option::is_none")]
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

async fn generate_combined_slo_report(
    slo_responses: Vec<(&str, Result<CreateChatCompletionResponse, OpenAIError>)>,
    openai: &OpenAiEnv,
) -> Result<String> {
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
                // Log the error or handle it as appropriate for your use case
                eprintln!("Error processing SLO '{}': {:?}", name, e);
                individual_feedbacks
                    .push(format!("SLO: {}\n\nError: Unable to process this SLO.", name));
            }
        }
    }

    let combined_feedback = individual_feedbacks.join("\n\n---\n\n");

    let openai_client = OpenAIClient::with_config(
        OpenAIConfig::new()
            .with_api_base(openai.api_base().to_owned())
            .with_api_key(openai.api_key().to_owned()),
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
            model: openai.model().to_owned(),
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
) -> Result<Vec<(&'static str, Result<CreateChatCompletionResponse, OpenAIError>)>> {
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

        ensure!(
            !relevant_file_codes.is_empty(),
            "No relevant files ({:?}) with source code found for SLO {}",
            slo_file_type,
            slo_name
        );

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
                    .with_api_key(openai_config.api_key().to_owned()),
            );

            let response = openai_client
                .chat()
                .create(CreateChatCompletionRequest {
                    model: openai_config.model().to_owned(),
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
    Ok(slo_responses)
}

/// Print grade results to stderr and optionally emit a Gradescope JSON
/// artifact.
///
/// * `results`: collection of requirement-level grades to render.
/// * `config`: strongly typed configuration that replaces the legacy Rhai map.
pub fn show_result(results: Vec<GradeResult>, config: GradescopeConfig) -> Result<()> {
    let GradescopeConfig {
        source_files,
        test_files,
        project_title,
        project_description,
        pass_threshold,
        show_table,
        results_json: gradescope_json,
        feedback: gradescope_feedback,
        debug: gradescope_debug,
        enabled_slos,
    } = config;

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
        let project = Project::new()?;
        let mut test_cases = vec![];
        for result in &results {
            let feedback = if gradescope_feedback {
                generate_single_feedback(result)?
            } else {
                String::new()
            };

            let test_case = GradescopeTestCase::builder()
                .name(result.requirement())
                .name_format(GradescopeOutputFormat::Text)
                .max_score(result.out_of())
                .score(result.grade())
                .status(if result.grade() > pass_threshold * result.out_of() {
                    GradescopeStatus::Passed
                } else {
                    GradescopeStatus::Failed
                })
                .output(feedback)
                .output_format(GradescopeOutputFormat::Md)
                .build();

            test_cases.push(test_case);
        }

        if grade > pass_threshold * out_of && !enabled_slos.is_empty() {
            ensure!(
                !project_title.is_empty(),
                "Project title must be specified to generate SLO feedback"
            );
            ensure!(
                !project_description.is_empty(),
                "Project description must be specified to generate SLO feedback"
            );

            let openai_env = config::openai_config().ok_or_else(|| {
                anyhow::anyhow!(
                    "OPENAI_ENDPOINT, OPENAI_API_KEY_SLO, and OPENAI_MODEL must be set to \
                     generate SLO feedback"
                )
            })?;

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
                })?,
                Err(_) => Runtime::new()
                    .context("Failed to create Tokio runtime for SLO feedback generation")?
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
                    })?,
            };

            let combined_report = match (tokio::runtime::Handle::try_current(), slo_responses) {
                (Ok(handle), responses) => block_in_place(move || {
                    handle.block_on(async move {
                        generate_combined_slo_report(responses, env_ref).await
                    })
                })?,
                (Err(_), responses) => Runtime::new()
                    .context("Failed to create Tokio runtime for SLO report generation")?
                    .block_on(async { generate_combined_slo_report(responses, env_ref).await })?,
            };

            test_cases.push(
                GradescopeTestCase::builder()
                    .name("Student Learning Outcomes (SLOs) Feedback".to_string())
                    .name_format(GradescopeOutputFormat::Text)
                    .output(combined_report)
                    .output_format(GradescopeOutputFormat::Md)
                    .max_score(0f64)
                    .score(0f64)
                    .build(),
            );
        }
        let submission = GradescopeSubmission::builder()
            .tests(Some(test_cases))
            .test_output_format(GradescopeOutputFormat::Md)
            .test_name_format(GradescopeOutputFormat::Text)
            .stdout_visibility(GradescopeVisibility::Visible)
            .visibility(GradescopeVisibility::Visible)
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
