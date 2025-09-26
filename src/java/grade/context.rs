use std::{collections::HashSet, ops::RangeInclusive};

use anyhow::{Context, Result};
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionResponse,
};
use itertools::Itertools;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json;

use crate::{
    config,
    java::{File, FileType, Parser, Project, queries::METHOD_CALL_QUERY},
    types::LineRef,
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// Parameters describing a single retrieval function call emitted by the LLM.
pub(crate) struct RetrievalFunctionCallParams {
    /// Fully qualified class name provided by the function call.
    pub(crate) class_name:  String,
    /// Method identifier requested for context extraction.
    pub(crate) method_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
/// Wrapper used to deserialize multiple retrieval requests from the LLM
/// response.
pub(crate) struct RetrievalFunctionCallParamsArray {
    /// Collection of individual retrieval parameters.
    pub(crate) params: Vec<RetrievalFunctionCallParams>,
}

/// Builds the system and user messages supplied to the retrieval service.
fn compose_retrieval_messages(
    proj: &Project,
    grader_output: &str,
) -> Result<Vec<ChatCompletionRequestMessage>> {
    let prompts = config::prompts();
    let java_file_names = proj.files().iter().map(File::proper_name).join(", ");
    let synthesized_outline = proj.describe();
    let outro = prompts
        .retrieval_message_outro()
        .replace("{JAVA_FILE_NAMES}", &java_file_names)
        .replace("{SYNTHESIZED_OUTLINE}", &synthesized_outline);

    let intro = ChatCompletionRequestSystemMessageArgs::default()
        .content(prompts.retrieval_message_intro().to_string())
        .name("Instructor".to_string())
        .build()?;

    let grader_context = ChatCompletionRequestUserMessageArgs::default()
        .content(format!(
            "Here is the output (stdout and stderr) from running the auto-grader on my \
             submission:\n```\n{}\n```",
            grader_output
        ))
        .name("Student".to_string())
        .build()?;

    let outro = ChatCompletionRequestSystemMessageArgs::default()
        .content(outro)
        .name("Instructor".to_string())
        .build()?;

    Ok(vec![intro.into(), grader_context.into(), outro.into()])
}

/// Calls the external retrieval function using the shared runtime.
fn invoke_retrieval_service(
    messages: &[ChatCompletionRequestMessage],
) -> Result<CreateChatCompletionResponse> {
    let payload =
        serde_json::to_string(messages).context("Failed to serialize retrieval messages")?;
    let client = Client::new();
    let runtime = config::runtime();

    runtime.block_on(async {
        let response = client
            .post("https://umm-feedback-openai-func.deno.dev/")
            .body(payload)
            .send()
            .await
            .context("Failed to call retrieval service")?
            .error_for_status()
            .context("Retrieval service returned error status")?;

        response
            .json::<CreateChatCompletionResponse>()
            .await
            .context("Failed to deserialize retrieval response")
    })
}

/// Resolves `LineRef`s to concrete files and expands them into inclusive
/// ranges.
fn expand_line_refs<T: Into<LineRef>>(
    line_refs: Vec<T>,
    proj: &Project,
    start_offset: usize,
    num_lines: usize,
) -> Result<Vec<(File, LineRef, RangeInclusive<usize>)>> {
    let mut expanded: Vec<(File, LineRef, RangeInclusive<usize>)> = line_refs
        .into_iter()
        .map(|line_ref| {
            let line_ref = line_ref.into();
            let file = proj.identify(&line_ref.file_name)?;
            let start = match file.kind() {
                FileType::Test => line_ref.line_number.saturating_sub(num_lines),
                _ => line_ref.line_number.saturating_sub(start_offset),
            };
            let end = start + num_lines;
            Ok((file, line_ref, start..=end))
        })
        .collect::<Result<_, anyhow::Error>>()?;

    expanded.sort_by(|lhs, rhs| {
        rhs.1
            .file_name
            .cmp(&lhs.1.file_name)
            .then(lhs.1.line_number.cmp(&rhs.1.line_number))
    });
    expanded.dedup();

    Ok(expanded)
}

/// Coalesces overlapping ranges that belong to the same file.
fn merge_ranges(
    ranges: Vec<(File, LineRef, RangeInclusive<usize>)>,
    num_lines: usize,
) -> Vec<(File, LineRef, RangeInclusive<usize>)> {
    ranges
        .into_iter()
        .coalesce(|lhs, rhs| {
            if lhs.0 == rhs.0 {
                let lhs_start = *lhs.2.start();
                let lhs_end = *lhs.2.end();
                let rhs_start = *rhs.2.start();
                let rhs_end = *rhs.2.end();
                let expanded_range = rhs_start.saturating_sub(num_lines)..=(rhs_end + num_lines);

                if expanded_range.contains(&lhs_start) || expanded_range.contains(&lhs_end) {
                    Ok((lhs.0, lhs.1, lhs_start..=rhs_end))
                } else {
                    Err((lhs, rhs))
                }
            } else {
                Err((lhs, rhs))
            }
        })
        .collect()
}

/// Renders a source snippet and returns the method names invoked within it.
fn render_snippet_with_methods(
    file: &File,
    line_ref: &LineRef,
    range: &RangeInclusive<usize>,
) -> Result<(Vec<String>, HashSet<String>)> {
    let mut block = Vec::new();
    let total_lines = file.code().lines().count();

    if total_lines == 0 {
        block.push(format!(
            "- Lines {} to {} from {} -\n```",
            range.start(),
            range.end(),
            line_ref.file_name
        ));
        block.push("```".to_string());
        return Ok((block, HashSet::new()));
    }

    let mut start = *range.start();
    let mut end = *range.end();
    let snippet_len = if end >= start { end - start + 1 } else { 0 };
    if (snippet_len as f32) >= 0.6 * (total_lines as f32) {
        start = 0;
        end = total_lines.saturating_sub(1);
    }

    if start >= total_lines {
        start = total_lines.saturating_sub(1);
    }
    if end >= total_lines {
        end = total_lines.saturating_sub(1);
    }
    if end < start {
        end = start;
    }

    block.push(format!("- Lines {} to {} from {} -\n```", start, end, line_ref.file_name));

    let width = line_number_width(total_lines);
    let file_lines: Vec<&str> = file.code().lines().collect();
    let raw_lines: Vec<&str> = file_lines
        .iter()
        .skip(start)
        .take(end - start + 1)
        .copied()
        .collect();

    let sanitized_lines: Vec<String> = raw_lines
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            let sanitized = line.replace("\\\\", "\\").replace("\\\"", "\"");
            format!("{:width$}|{}", start + idx, sanitized)
        })
        .collect();

    block.extend(sanitized_lines.clone());
    block.push("```".to_string());

    let snippet_source = raw_lines.join("\n");
    let method_names = collect_method_names(&snippet_source)?;

    Ok((block, method_names))
}

/// Collects method invocation identifiers from a rendered snippet.
fn collect_method_names(source: &str) -> Result<HashSet<String>> {
    // TODO: replace snippet reparsing with node-based queries once Rune tests
    // exist.
    let parser =
        Parser::new(source.to_string()).context("Failed to parse snippet for method calls")?;
    let matches = parser
        .query(METHOD_CALL_QUERY)
        .context("Failed to execute method call query")?;

    let mut methods = HashSet::new();
    for capture in matches {
        if let Some(name) = capture.get("name") {
            methods.insert(name.to_string());
        }
    }
    Ok(methods)
}

/// Appends method bodies for the provided method names to the context buffer.
fn append_method_bodies(
    proj: &Project,
    method_names: &HashSet<String>,
    context: &mut Vec<String>,
) -> Result<()> {
    if method_names.is_empty() {
        return Ok(());
    }

    let mut seen = HashSet::new();
    for method_name in method_names {
        let query = format!(include_str!("../../queries/method_body_with_name.scm"), method_name);

        for file in proj.files() {
            if !matches!(file.kind(), FileType::Class | FileType::ClassWithMain) {
                continue;
            }

            let results = file.query(&query).with_context(|| {
                format!("Failed to query method body in {}", file.proper_name())
            })?;

            if results.is_empty() {
                continue;
            }

            let file_lines: Vec<&str> = file.code().lines().collect();
            let width = line_number_width(file_lines.len());

            for capture in results {
                let body = match capture.get("body") {
                    Some(body) => body,
                    None => continue,
                };

                if !seen.insert((file.proper_name(), method_name.clone())) {
                    continue;
                }

                let body_lines: Vec<&str> = body.lines().collect();
                if body_lines.is_empty() {
                    continue;
                }

                let formatted = format_numbered_block(&body_lines, &file_lines, width);
                context.push(format!(
                    "Method body from student's submission `{}#{}`:",
                    file.proper_name(),
                    method_name
                ));
                context.push(format!("\n```\n{}\n```\n", formatted));
            }
        }
    }

    Ok(())
}

/// Calculates the width required to display line numbers for a file.
fn line_number_width(total_lines: usize) -> usize {
    if total_lines == 0 {
        1
    } else {
        (total_lines as f32).log10().ceil() as usize
    }
}

/// Formats a block of source lines with prefixed line numbers.
fn format_numbered_block(block_lines: &[&str], file_lines: &[&str], width: usize) -> String {
    let start_line_number = block_lines
        .first()
        .and_then(|first_line| {
            let trimmed = first_line.trim();
            file_lines
                .iter()
                .position(|line| line.contains(trimmed))
                .map(|idx| idx + 1)
        })
        .unwrap_or(1);

    block_lines
        .iter()
        .enumerate()
        .map(|(idx, line)| format!("{:width$}|{}", start_line_number + idx, line))
        .collect::<Vec<_>>()
        .join("\n")
}
/// Builds an active-retrieval context using the grader output captured from
/// stdout/stderr.
pub fn build_active_retrieval_context(
    proj: &Project,
    grader_output: String,
) -> Result<ChatCompletionRequestMessage> {
    let messages = compose_retrieval_messages(proj, grader_output.as_str())?;
    let response = invoke_retrieval_service(&messages)?.choices[0]
        .message
        .clone();

    let tool_calls = response
        .tool_calls
        .as_ref()
        .context("No function call found in response.")?;
    let function_call = tool_calls.first().context("Function call list was empty")?;
    let args = &function_call.function.arguments;
    let function_call_args: RetrievalFunctionCallParamsArray =
        serde_json::from_str(args).context("Failed to parse retrieval function arguments")?;

    let mut context = Vec::new();
    for function_call_arg in function_call_args.params {
        let file = proj.identify(&function_call_arg.class_name)?;
        let query = format!(
            include_str!("../../queries/method_body_with_name.scm"),
            &function_call_arg.method_name
        );

        let results = file
            .query(&query)
            .with_context(|| format!("Failed to query method body in {}", file.proper_name()))?;

        for capture in results {
            let body = match capture.get("body") {
                Some(body) => body,
                None => continue,
            };
            context.push(format!(
                "Method body from student's submission for `{}#{}`:",
                file.proper_name(),
                function_call_arg.method_name
            ));
            context.push(format!("\n```\n{}\n```\n", body));
        }
    }

    Ok(ChatCompletionRequestSystemMessageArgs::default()
        .content(context.join("\n"))
        .name("Instructor".to_string())
        .build()?
        .into())
}

/// Builds a heuristic snippet-based context using the provided configuration.
pub fn build_heuristic_context(
    line_refs: Vec<LineRef>,
    proj: Project,
    cfg: crate::retrieval::HeuristicConfig,
) -> Result<ChatCompletionRequestMessage> {
    let expanded = expand_line_refs(line_refs, &proj, cfg.start_offset, cfg.num_lines)?;
    let merged = merge_ranges(expanded, cfg.num_lines);

    let mut context_lines = vec![
        "You cannot see all of the student's submission as you are an AI language model, with \
         limited context length. Here are some snippets of code the stacktrace indicates might be \
         relevant:\n"
            .to_string(),
    ];
    let mut methods = HashSet::new();

    for (file, line_ref, range) in merged.into_iter().take(cfg.max_line_refs) {
        let (mut snippet, snippet_methods) = render_snippet_with_methods(&file, &line_ref, &range)?;
        context_lines.append(&mut snippet);
        methods.extend(snippet_methods);
    }

    append_method_bodies(&proj, &methods, &mut context_lines)?;

    let mut context = context_lines.join("\n");
    if context.len() > config::PROMPT_TRUNCATE {
        context.truncate(config::PROMPT_TRUNCATE);
        context.push_str("...[TRUNCATED]");
    }

    Ok(ChatCompletionRequestSystemMessageArgs::default()
        .content(context)
        .name("Instructor".to_string())
        .build()?
        .into())
}

/// Backwards-compatible wrapper retaining the pre-refactor API.
pub fn get_source_context<T: Into<LineRef>>(
    line_refs: Vec<T>,
    proj: Project,
    start_offset: usize,
    num_lines: usize,
    max_line_refs: usize,
    try_use_active_retrieval: bool,
    active_retrieval_context: Option<String>,
) -> Result<ChatCompletionRequestMessage> {
    if try_use_active_retrieval
        && let Some(ctx) = active_retrieval_context.clone()
        && let Ok(message) = build_active_retrieval_context(&proj, ctx)
    {
        return Ok(message);
    }

    let line_refs: Vec<LineRef> = line_refs.into_iter().map(Into::into).collect();
    build_heuristic_context(
        line_refs,
        proj,
        crate::retrieval::HeuristicConfig {
            start_offset,
            num_lines,
            max_line_refs,
        },
    )
}
