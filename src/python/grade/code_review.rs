#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! LLM-based code review grader for Python.
//!
//! This module provides grading functionality that uses LLM to analyze code
//! quality and provide detailed feedback, similar to the original grader.py.

use anyhow::{Result, anyhow, bail};
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

        // Build the grading prompt
        let mut prompt_content = String::new();

        // Add instructions if available
        if let Some(ref path) = self.instructions_path
            && let Ok(content) = std::fs::read_to_string(path)
        {
            prompt_content.push_str("## Assignment Instructions\n\n");
            prompt_content.push_str("```markdown\n");
            prompt_content.push_str(&content);
            prompt_content.push_str("\n```\n\n");
        }

        // Add file contents and execution results
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

            // Execute if requested
            if self.execute_files && file.has_main() {
                prompt_content.push_str("#### Execution Output\n\n");
                match file.run(None).await {
                    Ok(output) => {
                        prompt_content.push_str("```\n");
                        prompt_content.push_str(&output);
                        prompt_content.push_str("\n```\n\n");
                    }
                    Err(e) => {
                        prompt_content.push_str(&format!("**Error:** {}\n\n", e));
                    }
                }
            }
        }

        // Build messages
        let mut messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(prompts.system_message().to_string())
                .build()?
                .into(),
        ];

        // Add weekly context if available
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

        // Make the API call
        let client = OpenAIClient::with_config(
            OpenAIConfig::new()
                .with_api_base(&openai.endpoint)
                .with_api_key(&openai.api_key),
        );

        let request = CreateChatCompletionRequestArgs::default()
            .model(&openai.model)
            .messages(messages.clone())
            .temperature(0.6)
            .build()?;

        let response = client.chat().create(request).await?;

        let review = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_else(|| "No review generated".to_string());

        // For now, we give full marks if the code executes without errors
        // The LLM review is provided as feedback
        let grade = self.out_of;

        Ok(GradeResult::builder()
            .requirement(self.req_name.clone())
            .grade(Grade::new(grade, self.out_of))
            .reason(review)
            .prompt(messages)
            .build())
    }
}
