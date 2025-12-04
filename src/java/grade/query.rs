#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{fmt, sync::Arc};

use anyhow::{Context, Result};
use async_openai::types::chat::ChatCompletionRequestSystemMessageArgs;
use bon::Builder;
use snailquote::unescape;

use super::results::{Grade, GradeResult};
use crate::{
    config,
    java::{Parser, Project},
};

/// Predicate invoked to keep query results that satisfy additional constraints.
type QueryPredicate = Arc<dyn Fn(&str) -> bool + Send + Sync + 'static>;
#[derive(Default, Clone)]
/// A struct to represent a treesitter query.
pub struct Query {
    /// The query to run.
    query:   String,
    /// The capture to extract from the query.
    capture: String,
    /// Optional predicate applied to captured matches to refine the results.
    filter:  Option<QueryPredicate>,
}

impl Query {
    /// Creates a new query with default values (empty strings).
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the query to run.
    pub fn query(&self) -> Result<String, QueryError> {
        unescape(&format!("{:#?}", self.query)).map_err(|e| QueryError::DuringQueryExecution {
            q: self.query.clone(),
            e: e.to_string(),
        })
    }

    /// Sets the query to run.
    pub fn set_query(mut self, query: String) -> Self {
        self.query = query;
        self
    }

    /// Gets the captures to extract from the query.
    pub fn capture(&self) -> String {
        self.capture.clone()
    }

    /// Sets the captures to extract from the query.
    pub fn set_capture(mut self, capture: String) -> Self {
        self.capture = capture;
        self
    }

    /// Returns the optional predicate used to filter captured results.
    pub fn filter(&self) -> Option<QueryPredicate> {
        self.filter.clone()
    }

    /// Sets the predicate used to filter captured results.
    pub fn set_filter_fn<F>(mut self, filter: F) -> Self
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.filter = Some(Arc::new(filter));
        self
    }
}

impl fmt::Debug for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Query")
            .field("query", &self.query)
            .field("capture", &self.capture)
            .finish()
    }
}

/// An enum to represent possible errors when running a query.
#[derive(thiserror::Error, Debug)]
pub enum QueryError {
    /// No file was selected to run the query on.
    #[error("No file was selected to run the query on.")]
    NoFileSelected,
    /// No capture was selected to extract from the query.
    #[error("No capture was selected to extract from the query: {0}")]
    NoCaptureSelected(String),
    /// No previous query to add capture or filter to.
    #[error("No previous query to add capture or filter to.")]
    NoPreviousQuery,
    /// The file selected to run the query on does not exist.
    #[error("The file selected (`{0}`) to run the query on could not be found.")]
    FileNotFound(String),
    /// The query could not be run.
    #[error(
        "This query could not be run, likely due to a syntax \
         error.\nQuery:\n```\n{q}\n```\nError:\n```\n{e}\n```"
    )]
    DuringQueryExecution {
        /// The query that could not be run.
        q: String,
        /// The error that occurred.
        e: String,
    },
    /// No matches found for a previously selected capture, all subsequent
    /// queries will return nothing.
    #[error(
        "No matches found for a previously selected capture: `{0}`, all subsequent queries will \
         return nothing."
    )]
    NoMatchesFound(String),
    /// Unknown error.
    #[error("Unknown error: {0}")]
    Unknown(#[from] anyhow::Error),
}

#[derive(Default, Clone)]
/// An enum to represent the constraint of a query.
pub enum QueryConstraint {
    #[default]
    /// The query must match at least once.
    MustMatchAtLeastOnce,
    /// The query must match exactly once.
    MustMatchExactlyNTimes(usize),
    /// Must not match.
    MustNotMatch,
}

#[derive(Default, Clone, Builder)]
#[builder(on(String, into))]
/// A struct to represent a query grader.
pub struct QueryGrader {
    /// The name of the requirement.
    #[builder(getter)]
    req_name:   String,
    /// The grade for the requirement.
    #[builder(getter)]
    out_of:     f64,
    /// The queries to run.
    #[builder(default)]
    #[builder(with = FromIterator::from_iter)]
    #[builder(getter)]
    queries:    Vec<Query>,
    /// The input to run the queries on.
    #[builder(getter)]
    project:    Project,
    /// The file to run the query on.
    #[builder(getter)]
    file:       String,
    /// The constraint of the query.
    #[builder(default)]
    #[builder(getter)]
    constraint: QueryConstraint,
    /// The reason to share with the student.
    #[builder(default)]
    #[builder(getter)]
    reason:     String,
}

impl QueryGrader {
    /// Adds a query to run.
    /// If no file has been selected, this will throw an error.
    pub fn query(#[allow(unused_mut)] mut self, q: String) -> Result<Self, QueryError> {
        if self.file.is_empty() {
            return Err(QueryError::NoFileSelected);
        }

        self.queries.push(Query {
            query:   q,
            capture: String::new(),
            filter:  None,
        });

        Ok(self)
    }

    /// Adds a predicate that filters results from the most recent query.
    /// If no queries have been added, this will throw an error.
    pub fn capture(#[allow(unused_mut)] mut self, c: String) -> Result<Self, QueryError> {
        if let Some(last) = self.queries.last_mut() {
            *last = last.clone().set_capture(c);
            Ok(self)
        } else {
            Err(QueryError::NoPreviousQuery)
        }
    }

    /// Adds a capture to the last query.
    /// If no queries have been added, this will throw an error.
    pub fn filter<F>(#[allow(unused_mut)] mut self, f: F) -> Result<Self, QueryError>
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        if let Some(last) = self.queries.last_mut() {
            *last = last.clone().set_filter_fn(f);
            Ok(self)
        } else {
            Err(QueryError::NoPreviousQuery)
        }
    }

    /// Selects entire method body and returns
    pub fn method_body_with_name(mut self, method_name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("../queries/method_body_with_name.scm"), method_name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects entire method body and returns
    pub fn method_body_with_return_type(mut self, return_type: String) -> Self {
        self.queries.push(Query {
            query:   format!(
                include_str!("../queries/method_body_with_return_type.scm"),
                return_type
            ),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects and returns the entire main method
    pub fn main_method(mut self) -> Self {
        self.queries.push(Query {
            query:   include_str!("../queries/main_method.scm").to_string(),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects entire class body with name
    pub fn class_body_with_name(mut self, class_name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("../queries/class_with_name.scm"), class_name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects local variable declaration statements
    pub fn local_variables(mut self) -> Self {
        self.queries.push(Query {
            query:   String::from("((local_variable_declaration) @var)"),
            capture: "var".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects local variable declaration statements with supplied name
    pub fn local_variables_with_name(mut self, name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("../queries/local_variable_with_name.scm"), name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects local variable declaration statements with supplied type
    pub fn local_variables_with_type(mut self, type_name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("../queries/local_variable_with_type.scm"), type_name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects if statements (entire, including else if and else)
    pub fn if_statements(mut self) -> Self {
        self.queries.push(Query {
            query:   String::from("((if_statement) @if)"),
            capture: "if".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects for loops
    pub fn for_loops(mut self) -> Self {
        self.queries.push(Query {
            query:   String::from("((for_statement) @for)"),
            capture: "for".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects while loops
    pub fn while_loops(mut self) -> Self {
        self.queries.push(Query {
            query:   String::from("((while_statement) @while)"),
            capture: "while".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects method invocations
    pub fn method_invocations(mut self) -> Self {
        self.queries.push(Query {
            query:   include_str!("../queries/method_invocation.scm").to_string(),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects method invocations with supplied name
    pub fn method_invocations_with_name(mut self, name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("../queries/method_invocations_with_name.scm"), name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects method invocations with supplied arguments
    pub fn method_invocations_with_arguments(mut self, name: String) -> Self {
        self.queries.push(Query {
            query:   format!(
                include_str!("../queries/method_invocations_with_arguments.scm"),
                name
            ),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects method invocations with supplied object
    pub fn method_invocations_with_object(mut self, name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("../queries/method_invocations_with_object.scm"), name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Runs the configured queries and returns the captured results.
    /// TODO: Make it so that it doesn't parse a new piece of code, just filters
    /// out the irrelevant line ranges. This performs better but more
    /// importantly is more accurate.
    pub fn run_query(&self) -> Result<Vec<String>, QueryError> {
        let first = self
            .queries
            .first()
            .ok_or_else(|| QueryError::NoMatchesFound("No queries to run".to_string()))?;

        let file = self
            .project
            .identify(&self.file)
            .map_err(|_| QueryError::FileNotFound(self.file.to_string()))?;

        let first_query = first.query()?;

        let mut matches = match file.query(&first_query) {
            Ok(m) => {
                if first.capture().is_empty() {
                    return Err(QueryError::NoCaptureSelected(format!("{:#?}", first)));
                }
                if m.is_empty() {
                    return Err(QueryError::NoMatchesFound(first_query.clone()));
                }

                let mut captured: Vec<String> = m
                    .iter()
                    .filter_map(|map| map.get(&first.capture()))
                    .cloned()
                    .collect();

                if let Some(predicate) = first.filter() {
                    captured.retain(|value| predicate(value));
                }

                captured
            }
            Err(e) => {
                return Err(QueryError::DuringQueryExecution {
                    q: first_query.clone(),
                    e: format!("{:#?}", e),
                });
            }
        };

        for (index, query) in self.queries.iter().enumerate().skip(1) {
            if matches.is_empty() {
                let previous = &self.queries[index - 1];
                return Err(QueryError::NoMatchesFound(previous.query()?));
            }

            if query.capture().is_empty() {
                return Err(QueryError::NoCaptureSelected(format!("{:#?}", query)));
            }

            let mut new_matches = Vec::new();
            let current_matches = std::mem::take(&mut matches);

            let query_src = query.query()?;

            for snippet in current_matches {
                let parser = Parser::new(snippet.clone())
                    .context(format!("Failed to create parser for query: `{}`", query_src))
                    .map_err(QueryError::Unknown)?;

                let raw =
                    parser
                        .query(&query_src)
                        .map_err(|e| QueryError::DuringQueryExecution {
                            q: query_src.clone(),
                            e: format!("{:#?}", e),
                        })?;

                let mut captured: Vec<String> = raw
                    .iter()
                    .filter_map(|map| map.get(&query.capture()))
                    .cloned()
                    .collect();

                if let Some(predicate) = query.filter() {
                    captured.retain(|value| predicate(value));
                }

                new_matches.extend(captured);
            }

            matches = new_matches;
        }
        Ok(matches)
    }

    /// Grades the file according to the supplied queries, captures, and
    /// constraints.
    pub fn grade_by_query(self) -> Result<GradeResult> {
        let reason = if self.reason.trim().is_empty() {
            eprintln!(
                "Warning: No reason provided for query grading. Feedback to student will not be \
                 very helpful."
            );
            match self.constraint {
                QueryConstraint::MustMatchAtLeastOnce => {
                    "Query Constraint: Must match at least once.".to_string()
                }
                QueryConstraint::MustMatchExactlyNTimes(n) => {
                    format!("Query Constraint: Must match exactly {n} times.")
                }
                QueryConstraint::MustNotMatch => "Query Constraint: Must not match.".to_string(),
            }
        } else {
            self.reason.to_string()
        };

        let prompt_set = config::java_prompts();
        let result = match self.run_query() {
            Ok(matches) => matches,
            Err(e) => {
                return Ok(GradeResult::builder()
                    .requirement(self.req_name.clone())
                    .grade(Grade::new(0.0, self.out_of))
                    .reason(reason.clone())
                    .maybe_prompt(Some(vec![
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content(prompt_set.system_message().to_string())
                            .name("Instructor".to_string())
                            .build()
                            .context("Failed to build system message")?
                            .into(),
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content(format!(
                                "Something went wrong when using treesitter queries to grade \
                                 `{}`. Error message:\n\n```\n{}\n```\n",
                                self.file, e
                            ))
                            .name("Instructor".to_string())
                            .build()
                            .context("Failed to build system message")?
                            .into(),
                    ]))
                    .build());
            }
        };

        match self.constraint {
            QueryConstraint::MustMatchAtLeastOnce => {
                if result.is_empty() {
                    Ok(GradeResult::builder()
                        .requirement(self.req_name.clone())
                        .grade(Grade::new(0.0, self.out_of))
                        .reason(reason.clone())
                        .maybe_prompt(Some(vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(prompt_set.system_message().to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(format!("For file `{}`: {}.", self.file, self.reason))
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                        ]))
                        .build())
                } else {
                    Ok(GradeResult::builder()
                        .requirement(self.req_name.clone())
                        .grade(Grade::new(self.out_of, self.out_of))
                        .reason(reason.clone())
                        .maybe_prompt(None)
                        .build())
                }
            }
            QueryConstraint::MustMatchExactlyNTimes(n) => {
                if result.len() == n {
                    Ok(GradeResult::builder()
                        .requirement(self.req_name.clone())
                        .grade(Grade::new(self.out_of, self.out_of))
                        .reason(reason.clone())
                        .maybe_prompt(None)
                        .build())
                } else {
                    Ok(GradeResult::builder()
                        .requirement(self.req_name.clone())
                        .grade(Grade::new(0.0, self.out_of))
                        .reason(reason.clone())
                        .maybe_prompt(Some(vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(prompt_set.system_message().to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(format!("For file `{}`: {}", self.file, self.reason))
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                        ]))
                        .build())
                }
            }
            QueryConstraint::MustNotMatch => {
                if result.is_empty() {
                    Ok(GradeResult::builder()
                        .requirement(self.req_name.clone())
                        .grade(Grade::new(self.out_of, self.out_of))
                        .reason(reason)
                        .maybe_prompt(None)
                        .build())
                } else {
                    Ok(GradeResult::builder()
                        .requirement(self.req_name.clone())
                        .grade(Grade::new(0.0, self.out_of))
                        .reason(reason)
                        .maybe_prompt(Some(vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(prompt_set.system_message().to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(format!("For file `{}`: {}", self.file, self.reason))
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                        ]))
                        .build())
                }
            }
        }
    }
}

impl QueryGrader {
    /// Builds and runs the query grader.
    pub fn run(self) -> Result<GradeResult> {
        self.grade_by_query()
    }
}

impl<S> QueryGraderBuilder<S>
where
    S: query_grader_builder::IsComplete,
{
    /// Build the grader and immediately execute it.
    pub fn run(self) -> Result<GradeResult> {
        self.build().run()
    }
}
