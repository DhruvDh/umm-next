#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! LLM-based code review grader for Python.
//!
//! This module provides grading functionality that uses LLM to analyze code
//! quality and provide detailed feedback, similar to the original grader.py.

use anyhow::{Context, Result, anyhow, bail};
use async_openai::{
    Client as OpenAIClient,
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
};
use bon::Builder;
use serde::{Deserialize, Serialize};

use super::results::{Grade, GradeResult};
use crate::{config, python::Project};

/// Input information for a Python script (sample inputs for stdin).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputInfo {
    /// Sample inputs to provide to the script.
    pub sample_inputs:     Vec<String>,
    /// Description of input requirements.
    pub input_description: Option<String>,
}

/// Execution result from running a Python file.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Standard output.
    pub stdout:  String,
    /// Standard error.
    pub stderr:  String,
    /// Whether execution succeeded.
    pub success: bool,
    /// Input that was provided.
    pub input:   Option<String>,
}

/// A grader that uses LLM to provide code review and structured feedback.
#[derive(Clone, Default, Builder)]
#[builder(on(String, into))]
pub struct CodeReviewGrader {
    /// The project being graded.
    #[builder(getter)]
    project:             Project,
    /// Files to grade.
    #[builder(with = |iter: impl IntoIterator<Item = impl Into<String>>| {
        iter.into_iter().map(Into::into).collect::<Vec<String>>()
    })]
    #[builder(getter)]
    files:               Vec<String>,
    /// Path to assignment instructions (optional).
    #[builder(getter)]
    instructions_path:   Option<String>,
    /// Path to weekly course context (optional).
    #[builder(getter)]
    weekly_context_path: Option<String>,
    /// Requirement name.
    #[builder(getter)]
    req_name:            String,
    /// Total points available.
    #[builder(getter)]
    out_of:              f64,
    /// Whether to execute files and include output.
    #[builder(default = true)]
    #[builder(getter)]
    execute_files:       bool,
}

/// Structured code-review decision emitted by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodeReviewDecision {
    /// Whether the submission passes the code-review gate.
    pass:              bool,
    /// Markdown feedback shown to the student.
    feedback_markdown: String,
    /// Optional reasons attached to the decision.
    #[serde(default)]
    reasons:           Vec<String>,
}

/// Parses a JSON decision payload from model output.
fn parse_review_decision(content: &str) -> Result<CodeReviewDecision> {
    serde_json::from_str(content).or_else(|_| {
        let start = content
            .find('{')
            .context("Could not find JSON object start in model response")?;
        let end = content
            .rfind('}')
            .context("Could not find JSON object end in model response")?;
        if start > end {
            return Err(anyhow!(
                "Invalid JSON object bounds in model response: start index {} exceeds end index {}",
                start,
                end
            ));
        }
        let slice = content
            .get(start..=end)
            .context("Failed to extract JSON slice from model response")?;
        serde_json::from_str(slice).context("Failed to parse extracted JSON")
    })
}

/// Sends chat-completion request and returns assistant text.
async fn request_model_review(
    client: &OpenAIClient<OpenAIConfig>,
    model: &str,
    messages: Vec<ChatCompletionRequestMessage>,
) -> Result<String> {
    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages(messages)
        .temperature(0.2)
        .build()?;
    let response = client.chat().create(request).await?;
    Ok(response
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_else(|| "No review generated".to_string()))
}

impl CodeReviewGrader {
    /// Builds and runs the grader.
    pub async fn run(self) -> Result<GradeResult> {
        if self.files.is_empty() {
            bail!("CodeReviewGrader requires at least one file to grade");
        }
        self.grade_with_llm().await
    }

    /// Performs LLM-based grading.
    async fn grade_with_llm(self) -> Result<GradeResult> {
        let prompts = config::python_prompts();
        let openai =
            config::openai_env().ok_or_else(|| anyhow!("OpenAI environment not configured"))?;

        let mut prompt_content = String::new();
        let mut runtime_gate_passed = true;
        let mut runtime_failures = Vec::new();

        if let Some(ref path) = self.instructions_path
            && let Ok(content) = std::fs::read_to_string(path)
        {
            prompt_content.push_str("## Assignment Instructions\n\n");
            prompt_content.push_str("```markdown\n");
            prompt_content.push_str(&content);
            prompt_content.push_str("\n```\n\n");
        }

        prompt_content.push_str("## Python Files\n\n");

        for file_name in &self.files {
            let file = self.project.identify(file_name)?;

            prompt_content.push_str(&format!("### {}\n\n", file.file_name()));
            prompt_content.push_str(&format!("**Type:** {}\n", file.kind()));

            if !file.functions().is_empty() {
                prompt_content
                    .push_str(&format!("**Functions:** {}\n", file.functions().join(", ")));
            }
            if !file.classes().is_empty() {
                prompt_content.push_str(&format!("**Classes:** {}\n", file.classes().join(", ")));
            }

            prompt_content.push_str("\n#### Source Code\n\n```python\n");
            prompt_content.push_str(file.code());
            prompt_content.push_str("\n```\n\n");

            if self.execute_files && file.has_main() {
                prompt_content.push_str("#### Execution Output\n\n");
                match file.run(None).await {
                    Ok(output) => {
                        prompt_content.push_str("```\n");
                        prompt_content.push_str(&output);
                        prompt_content.push_str("\n```\n\n");
                    }
                    Err(e) => {
                        runtime_gate_passed = false;
                        runtime_failures.push(format!("{}: {}", file.file_name(), e));
                        prompt_content.push_str(&format!("**Error:** {}\n\n", e));
                    }
                }
            }
        }

        prompt_content.push_str("## Feedback Template\n\n");
        prompt_content.push_str(prompts.code_review_template());
        prompt_content.push_str(
            "\n\n## Required Output Format\nRespond with ONLY valid JSON (no markdown \
             fences):\n{\"pass\": <bool>, \"feedback_markdown\": <string>, \"reasons\": \
             [<string>, ...]}",
        );

        let mut messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(format!(
                    "{}\n\nYou must decide pass/fail and return JSON only in the exact schema \
                     requested by the user message.",
                    prompts.system_message()
                ))
                .build()?
                .into(),
        ];

        if let Some(ref path) = self.weekly_context_path
            && let Ok(content) = std::fs::read_to_string(path)
        {
            messages.push(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(format!("## Weekly Course Context\n\n{}", content))
                    .build()?
                    .into(),
            );
        }

        messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(prompt_content.clone())
                .build()?
                .into(),
        );

        let client = OpenAIClient::with_config(
            OpenAIConfig::new()
                .with_api_base(&openai.endpoint)
                .with_api_key(&openai.api_key),
        );
        let mut prompt_messages = messages.clone();

        let first_review =
            request_model_review(&client, &openai.model, prompt_messages.clone()).await?;
        let decision = match parse_review_decision(&first_review) {
            Ok(parsed) => parsed,
            Err(first_parse_err) => {
                let repair_message = ChatCompletionRequestUserMessageArgs::default()
                    .content(format!(
                        "Your previous response was not valid JSON for the required schema. \
                         Return ONLY valid JSON with keys `pass`, `feedback_markdown`, and \
                         `reasons` (array of strings).\n\nPrevious response:\n{}",
                        first_review
                    ))
                    .build()?
                    .into();
                prompt_messages.push(repair_message);
                let second_review =
                    request_model_review(&client, &openai.model, prompt_messages.clone()).await?;
                match parse_review_decision(&second_review) {
                    Ok(parsed) => parsed,
                    Err(second_parse_err) => {
                        let runtime_status = if self.execute_files {
                            if runtime_gate_passed {
                                "passed"
                            } else {
                                "failed"
                            }
                        } else {
                            "bypassed"
                        };
                        let reason = format!(
                            "Failed to parse structured code-review decision after one \
                             retry.\n\nFirst parse error: {first_parse_err:#}\nSecond parse \
                             error: {second_parse_err:#}\nRuntime gate: {runtime_status}\n\nFirst \
                             response:\n{first_review}\n\nSecond response:\n{second_review}"
                        );
                        return Ok(GradeResult::builder()
                            .requirement(self.req_name.clone())
                            .grade(Grade::new(0.0, self.out_of))
                            .reason(reason)
                            .prompt(prompt_messages)
                            .build());
                    }
                }
            }
        };

        let runtime_gate = !self.execute_files || runtime_gate_passed;
        let passed = runtime_gate && decision.pass;
        let grade = if passed { self.out_of } else { 0.0 };

        let mut reason_sections = vec![decision.feedback_markdown];
        if self.execute_files {
            if !runtime_gate_passed {
                reason_sections
                    .push(format!("Runtime gate failed: {}", runtime_failures.join(" | ")));
            }
        } else {
            reason_sections
                .push("Runtime gate bypassed because `execute_files=false`.".to_string());
        }
        if !decision.pass {
            if decision.reasons.is_empty() {
                reason_sections.push("LLM pass/fail gate failed.".to_string());
            } else {
                reason_sections
                    .push(format!("LLM pass/fail gate failed: {}", decision.reasons.join("; ")));
            }
        }

        Ok(GradeResult::builder()
            .requirement(self.req_name.clone())
            .grade(Grade::new(grade, self.out_of))
            .reason(reason_sections.join("\n\n"))
            .prompt(prompt_messages)
            .build())
    }
}
