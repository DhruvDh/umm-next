#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Retrieval and source context helpers for Python grading.

use std::path::Path;

use anyhow::Result;
use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
};

use crate::{
    python::{File, Project},
    types::LineRef,
};

/// Returns true when a `LineRef`'s filename (with or without a path prefix)
/// refers to the provided `File`.
fn line_ref_matches_file(line_ref: &LineRef, file: &File) -> bool {
    if line_ref.file_name == file.file_name()
        || line_ref.file_name == file.module_name()
        || line_ref
            .file_name
            .strip_suffix(".py")
            .unwrap_or(&line_ref.file_name)
            == file.name()
    {
        return true;
    }

    let lr_path = Path::new(line_ref.file_name.as_str());
    if lr_path == file.path() {
        return true;
    }

    if let Some(basename) = lr_path.file_name().and_then(|s| s.to_str()) {
        if basename == file.file_name() {
            return true;
        }

        if basename
            .strip_suffix(".py")
            .map(|b| b == file.name())
            .unwrap_or(false)
        {
            return true;
        }
    }

    false
}

/// Builds a context message from source snippets around the given line
/// references.
pub fn get_source_context(file: &File, line_refs: &[LineRef], window_size: usize) -> String {
    let mut context = String::new();
    let code_lines: Vec<&str> = file.code().lines().collect();
    let total_lines = code_lines.len();

    for line_ref in line_refs {
        if !line_ref_matches_file(line_ref, file) {
            continue;
        }

        let line_num = line_ref.line_number;
        let start = line_num.saturating_sub(window_size).max(1);
        let end = (line_num + window_size).min(total_lines);

        context.push_str(&format!("\n### {}:{}-{}\n```python\n", file.file_name(), start, end));

        for (idx, line) in code_lines.iter().enumerate() {
            let current_line = idx + 1;
            if current_line >= start && current_line <= end {
                let marker = if current_line == line_num {
                    ">>> "
                } else {
                    "    "
                };
                context.push_str(&format!("{}{:4}: {}\n", marker, current_line, line));
            }
        }

        context.push_str("```\n");
    }

    context
}

/// Builds a full codebase context message from all project files.
pub fn build_full_codebase_context(project: &Project) -> Result<ChatCompletionRequestMessage> {
    let mut content = String::new();
    content.push_str("## Full Codebase\n\n");

    for file in project.files() {
        content.push_str(&format!("### {}\n\n", file.file_name()));
        content.push_str(&format!("Type: {}\n", file.kind()));

        if !file.classes().is_empty() {
            content.push_str(&format!("Classes: {}\n", file.classes().join(", ")));
        }
        if !file.functions().is_empty() {
            content.push_str(&format!("Functions: {}\n", file.functions().join(", ")));
        }

        content.push_str("\n```python\n");
        content.push_str(file.code());
        content.push_str("\n```\n\n");
    }

    Ok(ChatCompletionRequestSystemMessageArgs::default()
        .content(content)
        .name("Codebase".to_string())
        .build()?
        .into())
}

/// Builds a heuristic context message from specific line references.
pub fn build_heuristic_context(
    project: &Project,
    line_refs: Vec<LineRef>,
    window_size: usize,
) -> Result<ChatCompletionRequestMessage> {
    let mut content = String::new();
    content.push_str("## Relevant Code Snippets\n\n");

    for line_ref in &line_refs {
        if let Ok(file) = project.identify(&line_ref.file_name) {
            content.push_str(&get_source_context(
                &file,
                std::slice::from_ref(line_ref),
                window_size,
            ));
        }
    }

    Ok(ChatCompletionRequestSystemMessageArgs::default()
        .content(content)
        .name("Context".to_string())
        .build()?
        .into())
}
