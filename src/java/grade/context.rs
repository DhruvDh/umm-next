use std::{collections::HashSet, ops::RangeInclusive};

use anyhow::{Result, ensure};
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionResponse,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json;

use super::diagnostics::LineRef;
use crate::{
    Dict, config,
    java::{File, FileType, Parser, Project, queries::METHOD_CALL_QUERY},
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

/// Retrieves the active context for a retrieval operation.
///
/// This function takes a reference to a `Project` and an optional `String` as
/// additional context. It ensures that the additional context is provided when
/// using active retrieval. It then prepares a series of
/// `ChatCompletionRequestMessage` and serializes them into a JSON string.
///
/// # Arguments
///
/// * `proj` - A reference to a `Project`.
/// * `additional_context` - An optional `String` that provides additional
///   context for the retrieval operation.
///
/// # Returns
///
/// * `Result<ChatCompletionRequestMessage>` - A `Result` that contains a
///   `ChatCompletionRequestMessage` if the operation was successful, or an
///   `Err` if it was not.
pub fn get_active_retrieval_context(
    proj: &Project,
    active_retrieval_context: Option<String>,
) -> Result<ChatCompletionRequestMessage> {
    ensure!(
        active_retrieval_context.is_some(),
        "Additional context must be provided when using active retrieval."
    );

    print!("Trying to decide what to share with AI for feedback...");

    let prompts = config::prompts();

    let java_file_names = proj.files().iter().map(File::proper_name).join(", ");
    let synthesized_outline = proj.describe();
    let outro_template = prompts.retrieval_message_outro();
    let outro = outro_template
        .replace("{JAVA_FILE_NAMES}", &java_file_names)
        .replace("{SYNTHESIZED_OUTLINE}", &synthesized_outline);

    let messages: Vec<ChatCompletionRequestMessage> = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(prompts.retrieval_message_intro().to_string())
            .name("Instructor".to_string())
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(format!(
                "Here is the output (stdout and stderr) from running the auto-grader on my \
                 submission:\n```\n{}\n```",
                active_retrieval_context.unwrap()
            ))
            .name("Student".to_string())
            .build()?
            .into(),
        ChatCompletionRequestSystemMessageArgs::default()
            .content(outro)
            .name("Instructor".to_string())
            .build()?
            .into(),
    ];

    let messages = serde_json::to_string(&messages).expect("Failed to serialize messages array");

    let client = reqwest::blocking::Client::new();
    let response: CreateChatCompletionResponse = client
        .post("https://umm-feedback-openai-func.deno.dev/")
        .body(messages)
        .send()?
        .json()?;
    let response = response.choices[0].message.clone();
    println!(" done!");
    ensure!(response.tool_calls.is_some(), "No function call found in response.");
    let function_call_args: RetrievalFunctionCallParamsArray = serde_json::from_str(
        response
            .tool_calls
            .unwrap()
            .first()
            .unwrap()
            .function
            .arguments
            .as_str(),
    )?;

    let mut context = Vec::new();
    for function_call_arg in function_call_args.params {
        let file = proj.identify(&function_call_arg.class_name)?;
        let query = format!(
            include_str!("../../queries/method_body_with_name.scm"),
            &function_call_arg.method_name
        );

        let res = file
            .query(&query)
            .or_else(|_| Ok::<Vec<Dict>, anyhow::Error>(vec![]))
            .unwrap();

        for r in res {
            let body = r.get("body").unwrap().to_string();
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

/// Returns a ChatCompletionRequestMessage with the given line references that
/// include contextual lines of code from the source
///
/// * `line_refs`: a vector of LineRef objects
/// * `proj`: a Project object
/// * `start_offset`: the number of lines of code to include before the line
/// * `num_lines`: the number of lines of code to include after the line
/// * `max_line_refs`: the maximum number of _processed_ line references to
///   include in the final message
/// * `try_use_active_retrieval`: whether to try to use active retrieval
/// * `additional_context`: additional context to use for
pub fn get_source_context<T: Into<LineRef>>(
    line_refs: Vec<T>,
    proj: Project,
    start_offset: usize,
    num_lines: usize,
    max_line_refs: usize,
    try_use_active_retrieval: bool,
    active_retrieval_context: Option<String>,
) -> Result<ChatCompletionRequestMessage> {
    if try_use_active_retrieval {
        match get_active_retrieval_context(&proj, active_retrieval_context) {
            Ok(message) => return Ok(message),
            Err(e) => {
                eprintln!("Failed to get active retrieval context: {e}");
            }
        }
    }

    let mut line_refs: Vec<(File, LineRef, RangeInclusive<usize>)> = line_refs
        .into_iter()
        .flat_map(|x| {
            let x = x.into();
            let file = proj.identify(&x.file_name)?;
            let start = match file.kind() {
                FileType::Test => x.line_number.saturating_sub(num_lines),
                _ => x.line_number.saturating_sub(start_offset),
            };
            let end = start + num_lines;
            Ok::<(File, LineRef, RangeInclusive<usize>), anyhow::Error>((file, x, start..=end))
        })
        .collect();

    line_refs.sort_by(|lhs, rhs| {
        rhs.1
            .file_name
            .cmp(&lhs.1.file_name)
            .then(lhs.1.line_number.cmp(&rhs.1.line_number))
    });
    line_refs.dedup();

    let mut context = Vec::new();
    context.push(
        "You cannot see all of the student's submission as you are an AI language model, with \
         limited context length. Here are some snippets of code the stacktrace indicates might be \
         relevant:
:\n"
        .to_string(),
    );
    let end_ticks = "\n```\n".to_string();
    let mut methods: HashSet<String> = HashSet::new();

    line_refs
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
        .take(max_line_refs)
        .for_each(|(file, f, r)| {
            let num_lines = r.size_hint().0;
            let count = file.code().lines().count();

            let (f, r) = if num_lines as f32 >= 0.6 * (count as f32) {
                (f, 0..=count)
            } else {
                (f, r)
            };

            context.push(format!(
                "- Lines {} to {} from {} -\n```",
                *r.start(),
                *r.end(),
                f.file_name
            ));

            let width = (count as f32).log10().ceil() as usize;

            let source_code_lines: Vec<String> = file.code().lines().map(String::from).collect();

            let relevant_source = source_code_lines
                .clone()
                .iter()
                .skip(*r.start())
                .take(num_lines)
                .enumerate()
                .map(|(line_n, x)| {
                    format!("{:width$}|{}", *r.start() + line_n, x)
                        .replace("\\\\", "\\")
                        .replace("\\\"", "\"")
                })
                .collect::<Vec<String>>();

            context.append(&mut (relevant_source.clone()));
            context.push(end_ticks.clone());

            match Parser::new(relevant_source.join("\n")) {
                Ok(parser) => {
                    let method_names: Vec<Dict> = parser
                        .query(METHOD_CALL_QUERY)
                        .or_else(|_| Ok::<Vec<Dict>, anyhow::Error>(vec![]))
                        .unwrap();

                    for method in method_names {
                        let method_name = method.get("name").unwrap().to_string();
                        methods.insert(method_name.clone());

                        let query = format!(
                            include_str!("../../queries/method_body_with_name.scm"),
                            &method_name
                        );

                        for f in proj.files() {
                            if *f.kind() == FileType::Class || *f.kind() == FileType::ClassWithMain
                            {
                                let res = f
                                    .query(&query)
                                    .or_else(|_| Ok::<Vec<Dict>, anyhow::Error>(vec![]))
                                    .unwrap();

                                for r in res {
                                    let body = r.get("body").unwrap().to_string();
                                    let body_lines =
                                        body.lines().map(String::from).collect::<Vec<_>>();
                                    if !body_lines.is_empty() {
                                        let start_line_number = source_code_lines
                                            .iter()
                                            .find_position(|x| {
                                                x.contains(body_lines.first().unwrap().trim())
                                            })
                                            .unwrap_or((0, &String::new()))
                                            .0;

                                        let body = body_lines
                                            .iter()
                                            .enumerate()
                                            .map(|(line_n, x)| {
                                                if start_line_number != 0 {
                                                    format!(
                                                        "{:width$}|{}",
                                                        start_line_number + line_n + 1,
                                                        x
                                                    )
                                                } else {
                                                    x.to_string()
                                                }
                                            })
                                            .collect::<Vec<String>>()
                                            .join("\n");

                                        context.push(format!(
                                            "Method body from student's submission `{}#{}`:",
                                            f.proper_name(),
                                            method_name
                                        ));
                                        context.push(format!("\n```\n{}\n```\n", body));
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error parsing partial source context: {e}");
                }
            };
        });

    let mut context = context.join("\n");
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
