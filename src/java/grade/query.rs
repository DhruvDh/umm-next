use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context, Result};
use async_openai::types::ChatCompletionRequestSystemMessageArgs;
use itertools::Itertools;
use rhai::{AST, Array, Dynamic, FnPtr};
use snailquote::unescape;

use super::results::{Grade, GradeResult};
use crate::{
    config, create_engine,
    java::{Parser, Project},
};

/// Lazily initialized Rhai AST storage retained while scripting is disabled.
static SCRIPT_AST: OnceLock<Arc<Mutex<AST>>> = OnceLock::new();

/// Shared Rhai AST placeholder retained while the grading flow migrates off
/// Rhai.
fn script_ast() -> &'static Arc<Mutex<AST>> {
    SCRIPT_AST.get_or_init(|| Arc::new(Mutex::new(AST::empty())))
}
#[derive(Default, Debug, Clone)]
/// A struct to represent a treesitter query.
pub struct Query {
    /// The query to run.
    query:   String,
    /// The capture to extract from the query.
    capture: String,
    /// A function pointer to filter the matches using. Must return a boolean.
    filter:  Option<FnPtr>,
}

impl Query {
    /// Creates a new query with default values (empty strings).
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the query to run.
    pub fn query(&self) -> String {
        unescape(&format!("{:#?}", self.query)).unwrap()
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

    /// Gets the function to filter the results of the query.
    pub fn filter(&self) -> Option<FnPtr> {
        self.filter.clone()
    }

    /// Set the function to filter the results of the query.
    pub fn set_filter(mut self, filter: FnPtr) -> Self {
        self.filter = Some(filter);
        self
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

#[derive(Default, Clone)]
/// A struct to represent a query grader.
pub struct QueryGrader {
    /// The name of the requirement.
    req_name:   String,
    /// The grade for the requirement.
    out_of:     f64,
    /// The queries to run.
    queries:    Vec<Query>,
    /// The input to run the queries on.
    project:    Project,
    /// The file to run the query on.
    file:       String,
    /// The constraint of the query.
    constraint: QueryConstraint,
    /// The reason to share with the student.
    reason:     String,
}

impl QueryGrader {
    /// Creates a new query grader with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the name of the requirement.
    pub fn req_name(&self) -> &str {
        &self.req_name
    }

    /// Sets the name of the requirement.
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// Gets the "out of" grade for the requirement.
    pub fn out_of(&self) -> f64 {
        self.out_of
    }

    /// Sets the "out of" grade for the requirement.
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// Gets the file to run the query on.
    pub fn file(&self) -> &str {
        &self.file
    }

    /// Sets the file to run the query on.
    pub fn set_file(mut self, file: String) -> Self {
        self.file = file;
        self
    }

    /// Gets the project to run the query on.
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Sets the project to run the query on.
    pub fn set_project(mut self, project: Project) -> Self {
        self.project = project;
        self
    }

    /// Gets the queries to run.
    pub fn queries(&self) -> Vec<Query> {
        self.queries.clone()
    }

    /// Gets the constraint of the query.
    pub fn constraint(&self) -> QueryConstraint {
        self.constraint.clone()
    }

    /// Sets the constraint of the query to "must match at least once".
    pub fn must_match_at_least_once(mut self) -> Self {
        self.constraint = QueryConstraint::MustMatchAtLeastOnce;
        self
    }

    /// Sets the constraint of the query to "must match exactly n times".
    pub fn must_match_exactly_n_times(mut self, n: usize) -> Self {
        self.constraint = QueryConstraint::MustMatchExactlyNTimes(n);
        self
    }

    /// Sets the constraint of the query to "must not match".
    pub fn must_not_match(mut self) -> Self {
        self.constraint = QueryConstraint::MustNotMatch;
        self
    }

    /// Gets the reason to share with the student.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Sets the reason to share with the student.
    pub fn set_reason(mut self, reason: String) -> Self {
        self.reason = reason;
        self
    }

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

    /// Adds a capture to the last query.
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
    pub fn filter(#[allow(unused_mut)] mut self, f: FnPtr) -> Result<Self, QueryError> {
        if let Some(last) = self.queries.last_mut() {
            *last = last.clone().set_filter(f);
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

    /// Runs the queries, and returns the result.
    /// TODO: Make it so that it doesn't parse a new piece of code, just filters
    /// out the irrelevant line ranges. This performs better but more
    /// importantly is more accurate.
    pub fn run_query(&self) -> Result<Dynamic, QueryError> {
        let engine = create_engine();
        let ast = Arc::clone(script_ast());
        let ast = ast.lock().unwrap();

        let first = self
            .queries
            .first()
            .ok_or_else(|| QueryError::NoMatchesFound("No queries to run".to_string()))?;

        let file = self
            .project
            .identify(self.file())
            .map_err(|_| QueryError::FileNotFound(self.file().to_string()))?;

        let mut matches: Vec<String> = match file.query(&first.query()) {
            Ok(m) => {
                if first.capture().is_empty() {
                    return Err(QueryError::NoCaptureSelected(format!("{:#?}", first)));
                }
                let result = m
                    .iter()
                    .filter_map(|map| map.get(&first.capture()))
                    .cloned();

                let result: Vec<String> = if let Some(f) = first.filter() {
                    result
                        .filter(|x| f.call(&engine, &ast, (x.clone(),)).unwrap_or(false))
                        .collect()
                } else {
                    result.collect()
                };

                if m.is_empty() {
                    return Err(QueryError::NoMatchesFound(
                        unescape(&format!("{:#?}", first)).context("Unescape error")?,
                    ));
                }
                result
            }
            Err(e) => {
                return Err(QueryError::DuringQueryExecution {
                    q: first.query(),
                    e: format!("{:#?}", e),
                });
            }
        };

        if self.queries.len() == 1 {
            return Ok(matches.into());
        }

        for (prev_q, q) in self.queries().into_iter().tuple_windows() {
            if matches.is_empty() {
                return Err(QueryError::NoMatchesFound(
                    unescape(&format!("{:#?}", prev_q)).context("Unescape error")?,
                ));
            }

            if q.capture().is_empty() {
                return Err(QueryError::NoCaptureSelected(format!("{:#?}", q)));
            }

            let mut new_matches = vec![];

            for code in matches {
                let parser = Parser::new(code)
                    .context(format!("Failed to create parser for query: `{}`", q.query()))?;

                match parser.query(&q.query()) {
                    Ok(m) => {
                        let result = m.iter().filter_map(|map| map.get(&q.capture())).cloned();

                        let mut result: Vec<String> = if let Some(f) = q.filter() {
                            result
                                .filter(|x| f.call(&engine, &ast, (x.clone(),)).unwrap_or(false))
                                .collect()
                        } else {
                            result.collect()
                        };

                        new_matches.append(&mut result)
                    }
                    Err(e) => {
                        return Err(QueryError::DuringQueryExecution {
                            q: q.query(),
                            e: format!("{:#?}", e),
                        });
                    }
                };
            }

            matches = new_matches;
        }

        Ok(matches.into())
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
        let result: Vec<String> = match self.run_query() {
            Ok(r) => {
                let r: Array = r.cast();
                r.into_iter().map(|s| s.cast()).collect()
            }
            Err(e) => {
                return Ok(GradeResult {
                    requirement: self.req_name.clone(),
                    grade: Grade {
                        grade:  0.0,
                        out_of: self.out_of,
                    },
                    reason,
                    prompt: Some(vec![
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
                    ]),
                });
            }
        };

        match self.constraint {
            QueryConstraint::MustMatchAtLeastOnce => {
                if result.is_empty() {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  0.0,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: Some(vec![
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
                        ]),
                    })
                } else {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  self.out_of,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: None,
                    })
                }
            }
            QueryConstraint::MustMatchExactlyNTimes(n) => {
                if result.len() == n {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  self.out_of,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: None,
                    })
                } else {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  0.0,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: Some(vec![
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
                        ]),
                    })
                }
            }
            QueryConstraint::MustNotMatch => {
                if result.is_empty() {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  self.out_of,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: None,
                    })
                } else {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  0.0,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: Some(vec![
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
                        ]),
                    })
                }
            }
        }
    }
}
