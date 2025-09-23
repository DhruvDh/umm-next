use anyhow::{Result, anyhow};
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use itertools::Itertools;
use rhai::Array;
use tabled::{
    Table,
    settings::{Alignment, Modify, Panel, Style, Width, object::Rows},
};

use super::{
    context::get_source_context,
    results::{Grade, GradeResult},
};
use crate::{
    config,
    java::{JavaFileError, Project},
    parsers::parser,
};
#[derive(Clone)]
/// A struct representing arguments to grade_docs function
pub struct DocsGrader {
    /// * `project`: the project to grade
    pub project:  Project,
    /// * `files`: the files to grade
    pub files:    Array,
    /// * `out_of`: the total points for the requirement
    pub out_of:   f64,
    /// * `req_name`: the name of the requirement
    pub req_name: String,
    /// * `penalty`: the penalty to apply for each instance of a violation.
    ///   Optional, default is 3
    pub penalty:  f64,
}

impl Default for DocsGrader {
    fn default() -> Self {
        Self {
            project:  Project::default(),
            files:    Array::new(),
            out_of:   0.0,
            req_name: String::new(),
            penalty:  3.0,
        }
    }
}

impl DocsGrader {
    /// Getter for project
    pub fn project(&mut self) -> Project {
        self.project.clone()
    }

    /// Setter for project
    pub fn set_project(mut self, project: Project) -> Self {
        self.project = project;
        self
    }

    /// Getter for files
    pub fn files(&mut self) -> Array {
        self.files.clone()
    }

    /// Setter for files
    pub fn set_files(mut self, files: Array) -> Self {
        self.files = files;
        self
    }

    /// Getter for out_of
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// Setter for out_of
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// Getter for req_name
    pub fn req_name(&mut self) -> String {
        self.req_name.clone()
    }

    /// Setter for req_name
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// Getter for penalty
    pub fn penalty(&mut self) -> f64 {
        self.penalty
    }

    /// Setter for penalty
    pub fn set_penalty(mut self, penalty: f64) -> Self {
        self.penalty = penalty;
        self
    }

    /// Grades documentation by using the -Xdoclint javac flag.
    /// Scans javac output for generated warnings and grades accordingly.
    pub fn grade_docs(self) -> Result<GradeResult> {
        let mut diags = vec![];
        let mut all_diags = vec![];
        let prompts = config::prompts();
        let files: Vec<String> = self
            .files
            .iter()
            .map(|f| match f.clone().into_string() {
                Ok(n) => Ok(n),
                Err(e) => Err(anyhow!("files array has something that's not a string: {}", e)),
            })
            .try_collect()?;
        let out_of = self.out_of;
        let mut outputs = vec![];
        for name in &files {
            let file = self.project.identify(name)?;
            let output = match file.doc_check() {
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
                        get_source_context(diags, self.project, 1, 3, 6, false, None)?,
                    ];

                    return Ok(GradeResult {
                        requirement: self.req_name,
                        grade:       Grade::new(0.0, out_of),
                        reason:      String::from("See above."),
                        prompt:      Some(messages),
                    });
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

                    return Ok(GradeResult {
                        requirement: self.req_name,
                        grade:       Grade::new(0.0, out_of),
                        reason:      String::from("See above."),
                        prompt:      Some(messages),
                    });
                }
            };
            outputs.push(output.clone());
            for line in output.lines() {
                let result = parser::parse_diag(line);
                match result {
                    Ok(res) => {
                        if file.file_name() == res.file_name() {
                            diags.push(res.clone());
                        }
                        all_diags.push(res);
                    }
                    Err(_) => continue,
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
            let context = get_source_context(all_diags, self.project, 1, 3, 6, false, None)?;

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
                    .content(include_str!("../../prompts/javadoc.md").to_string())
                    .name("Instructor".to_string())
                    .build()?
                    .into(),
            ])
        } else {
            None
        };
        Ok(GradeResult {
            requirement: self.req_name,
            grade: Grade::new(grade, out_of),
            reason: String::from("See above."),
            prompt,
        })
    }
}
