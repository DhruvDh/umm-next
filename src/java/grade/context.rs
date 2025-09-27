use std::{
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
};

use anyhow::{Context, Result};
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionResponse,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json;

use crate::{
    config,
    java::{File, FileType, Project},
    types::LineRef,
};

#[derive(Debug, Default)]
/// Numbered snippet lines paired with any method invocations discovered inside
/// them.
struct RenderedSnippet {
    /// Lines ready to be appended to the context buffer (header + code fence +
    /// numbered code).
    lines:   Vec<String>,
    /// Method identifiers captured within the snippet.
    methods: HashSet<String>,
}

type MethodsByFile = HashMap<String, HashSet<String>>;

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
    let client = config::http_client();
    let endpoint = config::retrieval_endpoint();
    let runtime = config::runtime();

    runtime.block_on(async move {
        let response = client
            .post(endpoint)
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

/// Renders a source snippet and returns the numbered lines alongside discovered
/// method calls.
fn render_snippet(
    file: &File,
    line_ref: &LineRef,
    range: &RangeInclusive<usize>,
    full_file_ratio: f32,
) -> Result<RenderedSnippet> {
    let total_lines = file.code().lines().count();

    if total_lines == 0 {
        let mut lines = Vec::with_capacity(2);
        lines.push(format!(
            "- Lines {} to {} from {} -",
            range.start(),
            range.end(),
            line_ref.file_name
        ));
        lines.push("```".to_string());
        lines.push("```".to_string());
        return Ok(RenderedSnippet {
            lines,
            methods: HashSet::new(),
        });
    }

    let (start, end) = compute_snippet_bounds(total_lines, range, full_file_ratio);
    let header = format!("- Lines {} to {} from {} -", start, end, line_ref.file_name);

    let file_lines: Vec<&str> = file.code().lines().collect();
    let snippet_lines = &file_lines[start..=end];
    let width = line_number_width(total_lines);
    let numbered_lines = format_numbered_snippet_lines(snippet_lines, start, width);

    let mut lines = Vec::with_capacity(numbered_lines.len() + 3);
    lines.push(header);
    lines.push("```".to_string());
    lines.extend(numbered_lines);
    lines.push("```".to_string());

    let methods = file
        .method_invocations()?
        .into_iter()
        .filter_map(|(name, line)| {
            let zero_based = line.saturating_sub(1);
            if zero_based >= start && zero_based <= end {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    Ok(RenderedSnippet { lines, methods })
}

/// Normalises an inclusive line range so it fits within the file and applies
/// the "full file" heuristic when the requested range covers most of the
/// source.
fn compute_snippet_bounds(
    total_lines: usize,
    range: &RangeInclusive<usize>,
    full_file_ratio: f32,
) -> (usize, usize) {
    if total_lines == 0 {
        return (0, 0);
    }

    let mut start = *range.start();
    let mut end = *range.end();

    if end < start {
        end = start;
    }

    let snippet_len = end.saturating_sub(start) + 1;
    if (snippet_len as f32) >= full_file_ratio * (total_lines as f32) {
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

    (start, end)
}

/// Renders the snippet lines with left-padded line numbers for display inside a
/// code fence.
fn format_numbered_snippet_lines(
    snippet_lines: &[&str],
    start_line: usize,
    width: usize,
) -> Vec<String> {
    snippet_lines
        .iter()
        .enumerate()
        .map(|(idx, line)| format!("{:width$}|{}", start_line + idx, sanitize_snippet_line(line)))
        .collect()
}

/// Reverts escaping introduced by javac diagnostics so the snippet reflects the
/// original source.
fn sanitize_snippet_line(line: &str) -> String {
    line.replace("\\\\", "\\").replace("\\\"", "\"")
}

/// Aggregates snippet lines and captured method names grouped by file.
fn build_snippet_sections(
    merged: Vec<(File, LineRef, RangeInclusive<usize>)>,
    cfg: crate::retrieval::HeuristicConfig,
) -> Result<(Vec<String>, MethodsByFile)> {
    let mut lines = Vec::new();
    let mut methods_by_file: MethodsByFile = HashMap::new();

    for (file, line_ref, range) in merged.into_iter().take(cfg.max_line_refs) {
        let rendered = render_snippet(&file, &line_ref, &range, cfg.full_file_ratio)?;
        lines.extend(rendered.lines);
        if !rendered.methods.is_empty() {
            methods_by_file
                .entry(file.proper_name())
                .or_default()
                .extend(rendered.methods);
        }
    }

    Ok((lines, methods_by_file))
}

/// Gathers method body sections for the provided method names, grouped by file.
fn collect_method_body_sections(
    proj: &Project,
    methods_by_file: &MethodsByFile,
) -> Result<Vec<String>> {
    if methods_by_file.is_empty() {
        return Ok(Vec::new());
    }

    let mut sections = Vec::new();
    let mut seen = HashSet::new();

    for (proper_name, methods) in methods_by_file {
        let file = proj
            .files()
            .iter()
            .find(|f| f.proper_name() == *proper_name)
            .with_context(|| {
                format!("File {proper_name} not found when gathering method bodies")
            })?;

        let width = line_number_width(file.code().lines().count());

        for method_name in methods {
            let bodies = file
                .method_bodies_named(method_name)
                .with_context(|| format!("Failed to query method body in {proper_name}"))?;

            for (body, start_line) in bodies {
                if !seen.insert((proper_name.clone(), method_name.clone())) {
                    continue;
                }

                let formatted = format_numbered_block(body.as_str(), start_line, width);
                sections.push(format!(
                    "Method body from student's submission `{proper_name}#{method_name}`:"
                ));
                sections.push(format!("\n```\n{}\n```\n", formatted));
            }
        }
    }

    Ok(sections)
}

/// Calculates the width required to display line numbers for a file.
fn line_number_width(total_lines: usize) -> usize {
    if total_lines == 0 {
        return 1;
    }

    let width = (total_lines as f32).log10().ceil() as usize;
    width.max(1)
}

/// Formats a block of source lines with prefixed line numbers.
fn format_numbered_block(body: &str, start_line: usize, width: usize) -> String {
    body.lines()
        .enumerate()
        .map(|(idx, line)| format!("{:width$}|{}", start_line + idx, line))
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
    let response = invoke_retrieval_service(&messages)?;
    let choice = response
        .choices
        .first()
        .context("Retrieval service returned no choices")?;
    let response_message = choice.message.clone();

    let tool_calls = response_message
        .tool_calls
        .as_ref()
        .context("No function call found in response message")?;
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

    let intro = "You cannot see all of the student's submission as you are an AI language model, \
                 with limited context length. Here are some snippets of code the stacktrace \
                 indicates might be relevant:\n"
        .to_string();

    let (snippet_lines, method_map) = build_snippet_sections(merged, cfg)?;
    let method_sections = collect_method_body_sections(&proj, &method_map)?;

    let mut context_lines = Vec::with_capacity(1 + snippet_lines.len() + method_sections.len());
    context_lines.push(intro);
    context_lines.extend(snippet_lines);
    context_lines.extend(method_sections);

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
            full_file_ratio: config::heuristic_full_file_ratio(),
        },
    )
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        path::PathBuf,
    };

    use async_openai::types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageContent,
        ChatCompletionRequestSystemMessageContentPart,
    };

    use super::*;
    use crate::{java::ProjectPaths, retrieval::HeuristicConfig};

    fn system_content_to_string(content: ChatCompletionRequestSystemMessageContent) -> String {
        match content {
            ChatCompletionRequestSystemMessageContent::Text(text) => text,
            ChatCompletionRequestSystemMessageContent::Array(parts) => parts
                .into_iter()
                .map(|part| match part {
                    ChatCompletionRequestSystemMessageContentPart::Text(text_part) => {
                        text_part.text
                    }
                })
                .collect::<Vec<_>>()
                .join(""),
        }
    }

    fn load_fixture_project(fixture: &str) -> Project {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/java")
            .join(fixture);
        let paths = ProjectPaths::new(root);
        Project::from_paths_for_tests(paths).expect("failed to load fixture project")
    }

    #[test]
    fn heuristic_context_emits_snippet_with_numbering() {
        let project = load_fixture_project("arraylist-example-solution");
        let line_ref = LineRef {
            file_name:   "DataStructures.ArrayList".to_string(),
            line_number: 107,
        };
        let config = HeuristicConfig {
            start_offset:    2,
            num_lines:       6,
            max_line_refs:   2,
            full_file_ratio: 0.6,
        };

        let message = build_heuristic_context(vec![line_ref], project, config)
            .expect("failed to build heuristic context");

        let ChatCompletionRequestMessage::System(system_message) = message else {
            panic!("expected system message");
        };

        let content = system_content_to_string(system_message.content);
        assert!(content.contains("You cannot see all of the student's submission"));
        assert!(content.contains("DataStructures.ArrayList"));
        assert!(content.contains("return remove(0);"));
        assert!(content.contains("throw new NoSuchElementException(\"List is empty.\");"));
        assert!(content.contains("101|  public T removeFirst()"));
    }

    #[test]
    fn snippet_bounds_clamped_when_range_exceeds_file() {
        let bounds = compute_snippet_bounds(10, &(15..=15), 0.6);
        assert_eq!(bounds, (9, 9));
    }

    #[test]
    fn snippet_bounds_expand_to_full_file_when_threshold_exceeded() {
        let bounds = compute_snippet_bounds(20, &(0..=19), 0.6);
        assert_eq!(bounds, (0, 19));
    }

    #[test]
    fn collect_method_sections_includes_each_method_once() {
        let project = load_fixture_project("arraylist-example-solution");
        let mut methods = HashSet::new();
        methods.insert("remove".to_string());
        let mut map = HashMap::new();
        map.insert("DataStructures.ArrayList".to_string(), methods);

        let sections = collect_method_body_sections(&project, &map).expect("method sections");
        let message_count = sections
            .iter()
            .filter(|line| line.contains("Method body from student's submission `"))
            .count();
        assert_eq!(message_count, 1, "expected exactly one method body header");
        assert!(
            sections
                .iter()
                .any(|line| line.contains("ArrayList#remove"))
        );
    }

    #[test]
    fn render_snippet_includes_invocations_on_range_edges() {
        let project = load_fixture_project("arraylist-example-solution");
        let file = project
            .identify("DataStructures.ArrayList")
            .expect("fixture file");
        let range = 102..=108;
        let rendered = render_snippet(
            &file,
            &LineRef {
                file_name:   file.proper_name(),
                line_number: 107,
            },
            &range,
            0.6,
        )
        .expect("render snippet");
        assert!(rendered.methods.contains("remove"));
        assert!(rendered.methods.contains("isEmpty"));
    }

    #[test]
    fn method_sections_preserve_capture_line_numbers() {
        let project = load_fixture_project("arraylist-example-solution");
        let mut methods = HashSet::new();
        methods.insert("removeFirst".to_string());
        let mut map = HashMap::new();
        map.insert("DataStructures.ArrayList".to_string(), methods);

        let sections = collect_method_body_sections(&project, &map).expect("method sections");
        let header_index = sections
            .iter()
            .position(|line| {
                line.contains(
                    "Method body from student's submission `DataStructures.ArrayList#removeFirst`",
                )
            })
            .expect("method header present");
        let body_block = &sections[header_index + 1];
        let expected_start = project
            .identify("DataStructures.ArrayList")
            .expect("fixture file")
            .method_bodies_named("removeFirst")
            .expect("method body lookup")
            .first()
            .map(|(_, line)| *line)
            .expect("method body line");
        assert!(
            body_block.starts_with(&format!("\n```\n{}|", expected_start)),
            "expected body to start with numbered line {} but saw {}",
            expected_start,
            body_block
        );
    }
}
