#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::fs;

use anyhow::{Context, Result, anyhow};
use async_openai::types::chat::ChatCompletionRequestMessage;
use bon::{Builder, vec};
use serde::Serialize;
use serde_json;
use tokio::{runtime::Runtime, task::block_in_place};
use uuid::Uuid;

use super::results::GradeResult;
use crate::config;
/// Schema for `prompts` table
#[derive(Serialize, Debug, Builder)]
#[builder(on(String, into))]
pub struct PromptRow {
    /// UUID of data entry
    pub(crate) id:               String,
    /// ChatGPT message prompt
    pub(crate) messages:         Option<Vec<ChatCompletionRequestMessage>>,
    /// Name of the autograder requirement
    pub(crate) requirement_name: String,
    /// Reasons for penalty
    pub(crate) reason:           String,
    /// Grade/out_of as a string
    pub(crate) grade:            String,
    /// Status of prompt response generation - not_started, started, completed
    pub(crate) status:           String,
}

/// Generates feedback for a single `GradeResult` and posts it to the database.
pub(crate) fn generate_single_feedback(result: &GradeResult) -> Result<String> {
    if result.grade_value() < result.out_of_value() {
        let client = config::postgrest_client().ok_or_else(|| {
            anyhow!(
                "SUPABASE_URL and SUPABASE_ANON_KEY must be set to generate detailed penalty \
                 feedback."
            )
        })?;
        let id = Uuid::new_v4().to_string();
        let result = result.clone();
        let body = PromptRow::builder()
            .id(id.clone())
            .maybe_messages(result.prompt.clone())
            .requirement_name(result.requirement.clone())
            .reason(result.reason.clone())
            .grade(result.grade_struct().to_string())
            .status("not_started")
            .build();

        let messages = serde_json::to_string(&body)?;

        // Post to the database
        let submit = insert_prompt_row(client.clone(), messages.clone());
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => block_in_place(move || handle.block_on(submit))?,
            Err(_) => Runtime::new()
                .context("Failed to create Tokio runtime for Supabase call")?
                .block_on(insert_prompt_row(client, messages))?,
        }

        // Return feedback URL
        Ok(format!(
            "- For explanation and feedback on `{}` (refer rubric), please \
             see this link - https://feedback.dhruvdh.com/{}",
            result.requirement.clone(),
            id
        ))
    } else {
        Ok(String::from(
            "This type of feedback cannot be generated for submissions without penalty.",
        ))
    }
}

/// Generates a FEEDBACK file after prompting ChatGPT for feedback on a
/// collection of results.
pub fn generate_feedback<I>(results: I) -> Result<()>
where
    I: IntoIterator<Item = GradeResult>,
{
    let mut feedback = vec!["## Understanding Your Autograder Results\n"];

    for result in results.into_iter() {
        let fb = generate_single_feedback(&result)?;
        feedback.push(fb);
    }

    if !feedback.is_empty() {
        let feedback = feedback.join("\n");
        fs::write("FEEDBACK", &feedback).context("Something went wrong writing FEEDBACK file.")?;
        eprintln!("{}", &feedback);
    } else {
        fs::write(
            "FEEDBACK",
            "This type of feedback cannot be generated for submissions without penalty.",
        )
        .context("Something went wrong writing FEEDBACK file.")?;
    }

    Ok(())
}

/// Inserts the serialized prompt row into Supabase.
async fn insert_prompt_row(client: postgrest::Postgrest, messages: String) -> Result<()> {
    client
        .from("prompts")
        .insert(messages)
        .execute()
        .await
        .context("Failed to write prompt row to Supabase")?;
    Ok(())
}
