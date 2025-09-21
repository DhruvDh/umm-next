use anyhow::{Context, Result, ensure};
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use colored::Colorize;
use rhai::Array;
use similar::{Algorithm, ChangeTag, utils::diff_unicode_words};

use super::{
    context::get_source_context,
    results::{Grade, GradeResult},
};
use crate::{
    config,
    java::{JavaFileError, Project},
};
#[derive(Clone, Default)]
/// string. Any difference results in a `0` grade.
/// A grader that grades by diffing an `expected` string with an `actual`
pub struct DiffGrader {
    /// name of requirement
    pub req_name:    String,
    /// points to give if all tests pass
    pub out_of:      f64,
    /// the project to grade
    pub project:     Project,
    /// Java file to run
    pub file:        String,
    /// the expected output
    pub expected:    Array,
    /// the actual output
    pub input:       Array,
    /// ignore case when comparing
    pub ignore_case: bool,
}

impl DiffGrader {
    /// creates a new DiffGrader
    pub fn new() -> Self {
        Self::default()
    }

    /// gets the `req_name` field
    pub fn req_name(&mut self) -> String {
        self.req_name.clone()
    }

    /// sets the `req_name` field
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// gets the `out_of` field
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// sets the `out_of` field
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// gets the `expected` field
    pub fn expected(&mut self) -> Array {
        self.expected.clone()
    }

    /// sets the `expected` field
    pub fn set_expected(mut self, expected: Array) -> Self {
        self.expected = expected;
        self
    }

    /// gets the `actual` field
    pub fn input(&mut self) -> Array {
        self.input.clone()
    }

    /// sets the `actual` field
    pub fn set_input(mut self, input: Array) -> Self {
        self.input = input;
        self
    }

    /// gets the `project` field
    pub fn project(&mut self) -> Project {
        self.project.clone()
    }

    /// sets the `project` field
    pub fn set_project(mut self, project: Project) -> Self {
        self.project = project;
        self
    }

    /// gets the `file` field
    pub fn file(&mut self) -> String {
        self.file.clone()
    }

    /// sets the `file` field
    pub fn set_file(mut self, file: String) -> Self {
        self.file = file;
        self
    }

    /// gets the `ignore_case` field
    pub fn ignore_case(&mut self) -> bool {
        self.ignore_case
    }

    /// sets the `ignore_case` field
    pub fn set_ignore_case(mut self, ignore_case: bool) -> Self {
        self.ignore_case = ignore_case;
        self
    }

    /// Grades by diffing the `expected` and `actual` strings.
    pub fn grade_by_diff(&mut self) -> Result<GradeResult> {
        ensure!(
            !self.expected.is_empty() & !self.input.is_empty(),
            "At least one test case (input-expected pair) must be provided"
        );
        ensure!(
            self.expected.len() == self.input.len(),
            "expected and input case arrays must be of the same length"
        );

        let file = self.project.identify(&self.file)?;
        let prompt_set = config::prompts();
        let mut prompts = vec![];

        for (expected, input) in self.expected.iter().zip(self.input.iter()) {
            let expected = {
                let expected = expected.clone().cast::<String>();
                if self.ignore_case {
                    expected.to_lowercase().trim().to_string()
                } else {
                    expected.trim().to_string()
                }
            };
            let input = input.clone().cast::<String>();

            let actual_out = {
                let out = match file.run(Some(input.clone())) {
                    Ok(out) => out,
                    Err(JavaFileError::AtRuntime { output, diags }) => {
                        let messages = vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(prompt_set.system_message().to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(format!("Error while running -\n```\n{}\n```", output))
                                .name("Student".to_string())
                                .build()
                                .context("Failed to build user message")?
                                .into(),
                            get_source_context(diags, self.project.clone(), 3, 6, 6, false, None)?,
                        ];
                        return Ok(GradeResult {
                            requirement: self.req_name.clone(),
                            grade:       Grade::new(0.0, self.out_of),
                            reason:      "Error running file for some cases.".to_string(),
                            prompt:      Some(messages),
                        });
                    }
                    Err(JavaFileError::DuringCompilation { stacktrace, diags }) => {
                        let messages = vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(prompt_set.system_message().to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(format!(
                                    "Error while compiling -\n```\n{}\n```",
                                    stacktrace
                                ))
                                .name("Student".to_string())
                                .build()
                                .context("Failed to build user message")?
                                .into(),
                            get_source_context(diags, self.project.clone(), 3, 6, 6, false, None)?,
                        ];
                        return Ok(GradeResult {
                            requirement: self.req_name.clone(),
                            grade:       Grade::new(0.0, self.out_of),
                            reason:      "Error compiling file for some cases.".to_string(),
                            prompt:      Some(messages),
                        });
                    }
                    Err(e) => {
                        let messages = vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(prompt_set.system_message().to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(format!("Unknown error -\n```\n{:?}\n```", e))
                                .name("Student".to_string())
                                .build()
                                .context("Failed to build user message")?
                                .into(),
                        ];
                        return Ok(GradeResult {
                            requirement: self.req_name.clone(),
                            grade:       Grade::new(0.0, self.out_of),
                            reason:      "Unknown error while running file for some cases."
                                .to_string(),
                            prompt:      Some(messages),
                        });
                    }
                };

                if self.ignore_case {
                    out.to_lowercase().trim().to_string()
                } else {
                    out.trim().to_string()
                }
            };

            let diff = diff_unicode_words(Algorithm::Patience, &expected, &actual_out);

            let mut is_equal = true;
            let mut expected = String::new();
            let mut actual = String::new();

            for (change, value) in diff {
                match change {
                    ChangeTag::Equal => {
                        expected.push_str(value);
                        actual.push_str(value);
                    }
                    ChangeTag::Insert => {
                        actual.push_str(format!("{}", value.green()).as_str());
                        if !value.trim().is_empty() {
                            is_equal = false;
                        }
                    }
                    ChangeTag::Delete => {
                        expected.push_str(format!("{}", value.red()).as_str());
                        if !value.trim().is_empty() {
                            is_equal = false;
                        }
                    }
                }
            }

            if !is_equal {
                let prompt = format!(
                    "Comparing expected and actual output for \
                     {}:\n```{inp}Expected:\n{}\nActual:\n{}\n```\n",
                    file.file_name(),
                    expected,
                    actual,
                    inp = if self.input.is_empty() {
                        String::new()
                    } else {
                        format!("\nInput:\n`{}`\n", input)
                    },
                );

                eprintln!("{prompt}");
                prompts.push(prompt);
            }
        }

        if prompts.is_empty() {
            Ok(GradeResult {
                requirement: self.req_name.clone(),
                grade:       Grade {
                    grade:  self.out_of,
                    out_of: self.out_of,
                },
                reason:      "Got expected output".to_string(),
                prompt:      None,
            })
        } else {
            let context = format!(
                "{prompt}\n\nSource code:\n```java\n{code}\n```\nMy tests are failing due to the \
                 above.",
                prompt = prompts.join("\n\n"),
                code = file.parser().code()
            );

            Ok(GradeResult {
                requirement: self.req_name.clone(),
                grade:       Grade {
                    grade:  0.0,
                    out_of: self.out_of,
                },
                reason:      "See above.".to_string(),
                prompt:      Some(vec![
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(config::prompts().system_message().to_string())
                        .name("Instructor".to_string())
                        .build()
                        .context("Failed to build system message")?
                        .into(),
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(context)
                        .name("Student".to_string())
                        .build()
                        .context("Failed to build system message")?
                        .into(),
                ]),
            })
        }
    }
}
