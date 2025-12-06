#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Documentation grading for Python (docstrings, PEP-257 compliance).

use anyhow::{Result, bail};
use async_openai::types::chat::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use bon::Builder;

use super::results::{Grade, GradeResult};
use crate::{config, python::Project};

/// A grader that checks Python docstrings and documentation quality.
#[derive(Clone, Default, Builder)]
#[builder(on(String, into))]
pub struct DocsGrader {
    /// The project being graded.
    #[builder(getter)]
    project:  Project,
    /// Files to check for documentation.
    #[builder(with = |iter: impl IntoIterator<Item = impl Into<String>>| {
        iter.into_iter().map(Into::into).collect::<Vec<String>>()
    })]
    #[builder(getter)]
    files:    Vec<String>,
    /// Total points available.
    #[builder(getter)]
    out_of:   f64,
    /// Requirement name.
    #[builder(getter)]
    req_name: String,
    /// Penalty per missing docstring.
    #[builder(default = 1.0)]
    #[builder(getter)]
    penalty:  f64,
}

impl DocsGrader {
    /// Builds and runs the grader.
    pub async fn run(self) -> Result<GradeResult> {
        if self.files.is_empty() {
            bail!("DocsGrader requires at least one file to grade");
        }
        self.grade_docs().await
    }

    /// Performs the documentation grading.
    async fn grade_docs(self) -> Result<GradeResult> {
        let prompts = config::python_prompts();
        let mut all_issues = Vec::new();
        let mut total_missing = 0usize;

        for file_name in &self.files {
            let file = self.project.identify(file_name)?;

            // Check for module docstring
            let has_module_docstring = self.check_module_docstring(&file)?;
            if !has_module_docstring {
                total_missing += 1;
                all_issues.push(format!("{}: Missing module docstring", file_name));
            }

            // Check function docstrings
            for func in file.functions() {
                if !self.check_function_docstring(&file, func)? {
                    total_missing += 1;
                    all_issues
                        .push(format!("{}: Missing docstring for function '{}'", file_name, func));
                }
            }

            // Check class docstrings
            for class in file.classes() {
                if !self.check_class_docstring(&file, class)? {
                    total_missing += 1;
                    all_issues
                        .push(format!("{}: Missing docstring for class '{}'", file_name, class));
                }
            }
        }

        let penalty = total_missing as f64 * self.penalty;
        let grade = (self.out_of - penalty).max(0.0);
        let reason = if all_issues.is_empty() {
            "All documentation present".to_string()
        } else {
            format!(
                "-{} due to {} missing docstrings:\n{}",
                penalty,
                total_missing,
                all_issues.join("\n")
            )
        };

        let prompt = if total_missing > 0 {
            Some(vec![
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(prompts.system_message().to_string())
                    .name("Instructor".to_string())
                    .build()?
                    .into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content(format!(
                        "The following documentation issues were found:\n\n{}",
                        all_issues.join("\n")
                    ))
                    .name("Student".to_string())
                    .build()?
                    .into(),
            ])
        } else {
            None
        };

        Ok(GradeResult::builder()
            .requirement(self.req_name.clone())
            .grade(Grade::new(grade, self.out_of))
            .reason(reason)
            .maybe_prompt(prompt)
            .build())
    }

    /// Checks if a file has a module-level docstring.
    fn check_module_docstring(&self, file: &crate::python::File) -> Result<bool> {
        let query = r#"
            (module
              (expression_statement
                (string) @docstring))
        "#;

        let results = file.query(query)?;

        // Check if there's a string as the first expression statement
        if let Some(first_result) = results.first()
            && first_result.get("docstring").is_some()
        {
            return Ok(true);
        }

        Ok(false)
    }

    /// Checks if a function has a docstring.
    fn check_function_docstring(
        &self,
        file: &crate::python::File,
        func_name: &str,
    ) -> Result<bool> {
        let query = format!(
            r#"
            (function_definition
              name: (identifier) @name
              body: (block
                (expression_statement
                  (string) @docstring))
              (#eq? @name "{}"))
            "#,
            func_name
        );

        let results = file.query(&query)?;
        Ok(!results.is_empty())
    }

    /// Checks if a class has a docstring.
    fn check_class_docstring(&self, file: &crate::python::File, class_name: &str) -> Result<bool> {
        let query = format!(
            r#"
            (class_definition
              name: (identifier) @name
              body: (block
                (expression_statement
                  (string) @docstring))
              (#eq? @name "{}"))
            "#,
            class_name
        );

        let results = file.query(&query)?;
        Ok(!results.is_empty())
    }
}
