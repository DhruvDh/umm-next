#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use anyhow::{Result, bail};
use async_openai::types::chat::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use bon::Builder;
use tabled::{
    Table,
    settings::{Alignment, Modify, Panel, Style, Width, object::Rows},
};

use super::results::{Grade, GradeResult};
use crate::{
    config,
    java::{JavaFileError, Project, parsers::parser},
    retrieval::build_context_message,
};
#[derive(Clone, Builder)]
#[builder(on(String, into))]
/// A struct representing arguments to grade_docs function
pub struct DocsGrader {
    /// * `project`: the project to grade
    #[builder(getter)]
    pub project:  Project,
    /// * `files`: the files to grade
    #[builder(with = |iter: impl IntoIterator<Item = impl Into<String>>| {
        iter.into_iter().map(Into::into).collect::<Vec<String>>()
    })]
    #[builder(getter)]
    pub files:    Vec<String>,
    /// * `out_of`: the total points for the requirement
    #[builder(getter)]
    pub out_of:   f64,
    /// * `req_name`: the name of the requirement
    #[builder(getter)]
    pub req_name: String,
    /// * `penalty`: the penalty to apply for each instance of a violation.
    ///   Optional, default is 3
    #[builder(default = 3.0)]
    #[builder(getter)]
    pub penalty:  f64,
}

impl Default for DocsGrader {
    fn default() -> Self {
        Self {
            project:  Project::default(),
            files:    Vec::new(),
            out_of:   0.0,
            req_name: String::new(),
            penalty:  3.0,
        }
    }
}

impl DocsGrader {
    /// Grades documentation by using the -Xdoclint javac flag.
    /// Scans javac output for generated warnings and grades accordingly.
    pub async fn grade_docs(self) -> Result<GradeResult> {
        let mut diags = vec![];
        let mut all_diags = vec![];
        let prompts = config::java_prompts();
        let files = self.files.clone();
        let out_of = self.out_of;
        let mut outputs = vec![];
        for name in &files {
            let file = self.project.identify(name)?;
            let output = match file.doc_check().await {
                Ok(o) => o,
                Err(JavaFileError::DuringCompilation { stacktrace, diags }) => {
                    let messages = vec![
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content(prompts.system_message().to_string())
                            .name("Instructor".to_string())
                            .build()?
                            .into(),
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(format!("Compiler error -\n```\n{}\n```", stacktrace))
                            .name("Student".to_string())
                            .build()?
                            .into(),
                        build_context_message(&self.project, None, diags)?,
                    ];

                    return Ok(GradeResult::builder()
                        .requirement(self.req_name.clone())
                        .grade(Grade::new(0.0, out_of))
                        .reason("See above.")
                        .maybe_prompt(Some(messages))
                        .build());
                }
                Err(e) => {
                    let messages = vec![
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content(prompts.system_message().to_string())
                            .name("Instructor".to_string())
                            .build()?
                            .into(),
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(format!("Unknown error -\n```\n{:?}\n```", e))
                            .name("Student".to_string())
                            .build()?
                            .into(),
                    ];

                    return Ok(GradeResult::builder()
                        .requirement(self.req_name.clone())
                        .grade(Grade::new(0.0, out_of))
                        .reason("See above.")
                        .maybe_prompt(Some(messages))
                        .build());
                }
            };
            outputs.push(output.clone());
            for line in output.lines() {
                if let Ok(res) = parser::parse_diag(line) {
                    if file.file_name() == res.file_name() {
                        diags.push(res.clone());
                    }
                    all_diags.push(res);
                }
            }
        }

        let penalty = diags.len() as f64 * self.penalty;
        let grade = if out_of - penalty > 0.0 {
            out_of - penalty
        } else {
            0.0
        };

        let num_diags = diags.len();
        eprintln!(
            "{}",
            Table::new(&diags)
                .with(Panel::header(format!("Check javadoc for {}", files.join(", "))))
                .with(Panel::footer(format!("-{penalty} due to {num_diags} nits")))
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

        let prompt = if num_diags > 0 {
            let context = build_context_message(&self.project, None, all_diags)?;

            let mut outputs = outputs
                .iter()
                .map(|output| format!("```\n{output}\n```"))
                .collect::<Vec<String>>()
                .join("\n\n---\n\n");

            if outputs.len() > config::PROMPT_TRUNCATE {
                outputs.truncate(config::PROMPT_TRUNCATE);
                outputs.push_str("...[TRUNCATED]");
            }

            Some(vec![
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(prompts.system_message().to_string())
                    .name("Instructor".to_string())
                    .build()?
                    .into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content(outputs)
                    .name("Student".to_string())
                    .build()?
                    .into(),
                context,
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(include_str!("../prompts/javadoc.md").to_string())
                    .name("Instructor".to_string())
                    .build()?
                    .into(),
            ])
        } else {
            None
        };
        Ok(GradeResult::builder()
            .requirement(self.req_name.clone())
            .grade(Grade::new(grade, out_of))
            .reason("See above.")
            .maybe_prompt(prompt)
            .build())
    }
}

impl DocsGrader {
    /// Builds and runs the documentation grader.
    pub async fn run(self) -> Result<GradeResult> {
        if self.files.is_empty() {
            bail!("DocsGrader requires at least one file to grade");
        }
        self.grade_docs().await
    }
}

impl<S> DocsGraderBuilder<S>
where
    S: docs_grader_builder::IsComplete,
{
    /// Build the grader and immediately execute it.
    pub async fn run(self) -> Result<GradeResult> {
        self.build().run().await
    }
}
