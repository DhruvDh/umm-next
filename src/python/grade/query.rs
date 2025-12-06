#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Tree-sitter query grading for Python.

use std::{fmt, sync::Arc};

use anyhow::{Result, bail};
use bon::Builder;

use super::results::{Grade, GradeResult};
use crate::python::Project;

/// Predicate invoked to filter query results.
type QueryPredicate = Arc<dyn Fn(&str) -> bool + Send + Sync + 'static>;

/// Represents a tree-sitter query with optional capture and filter.
#[derive(Default, Clone)]
pub struct Query {
    /// The tree-sitter query string.
    query:   Option<String>,
    /// The capture name to extract.
    capture: String,
    /// Optional filter predicate.
    filter:  Option<QueryPredicate>,
}

impl Query {
    /// Creates a new empty query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the query string.
    pub fn query(&self) -> Result<String, QueryError> {
        self.query.clone().ok_or(QueryError::NoQueryProvided)
    }

    /// Sets the query string.
    pub fn set_query(mut self, query: String) -> Self {
        self.query = Some(query);
        self
    }

    /// Gets the capture name.
    pub fn capture(&self) -> String {
        self.capture.clone()
    }

    /// Sets the capture name.
    pub fn set_capture(mut self, capture: String) -> Self {
        self.capture = capture;
        self
    }

    /// Gets the filter predicate.
    pub fn filter(&self) -> Option<QueryPredicate> {
        self.filter.clone()
    }

    /// Sets the filter predicate.
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

/// Errors that can occur during query grading.
#[derive(thiserror::Error, Debug)]
pub enum QueryError {
    /// No file was selected.
    #[error("No file was selected to run the query on.")]
    NoFileSelected,
    /// No query was provided.
    #[error("No query was provided.")]
    NoQueryProvided,
    /// No queries were added.
    #[error("No queries were added to the grader.")]
    NoQueriesAdded,
    /// Query returned no matches.
    #[error("Query '{0}' returned no matches.")]
    NoMatchesFound(String),
    /// Unknown error.
    #[error("Unknown error: {0}")]
    Unknown(#[from] anyhow::Error),
}

/// Constraint on query matching.
#[derive(Default, Clone, Debug)]
pub enum QueryConstraint {
    #[default]
    /// Must match at least once.
    MustMatchAtLeastOnce,
    /// Must match exactly N times.
    MustMatchExactlyNTimes(usize),
    /// Must not match.
    MustNotMatch,
}

/// A grader that uses tree-sitter queries to validate code structure.
#[derive(Default, Clone, Builder)]
#[builder(on(String, into))]
pub struct QueryGrader {
    /// The project being graded.
    #[builder(getter)]
    project:    Project,
    /// Name of the file to query.
    #[builder(getter)]
    file:       String,
    /// Queries to run.
    #[builder(default)]
    #[builder(getter)]
    queries:    Vec<Query>,
    /// Constraint to apply.
    #[builder(default)]
    #[builder(getter)]
    constraint: QueryConstraint,
    /// Reason to report on failure.
    #[builder(default)]
    #[builder(getter)]
    reason:     String,
    /// Requirement name.
    #[builder(getter)]
    req_name:   String,
    /// Total points available.
    #[builder(getter)]
    out_of:     f64,
}

impl QueryGrader {
    /// Adds a query to run.
    pub fn query(mut self, q: String) -> Result<Self, QueryError> {
        if self.file.is_empty() {
            return Err(QueryError::NoFileSelected);
        }
        self.queries.push(Query::new().set_query(q));
        Ok(self)
    }

    /// Sets the capture for the last query.
    pub fn capture(mut self, c: String) -> Result<Self, QueryError> {
        if self.queries.is_empty() {
            return Err(QueryError::NoQueriesAdded);
        }
        if let Some(last) = self.queries.last_mut() {
            *last = std::mem::take(last).set_capture(c);
        }
        Ok(self)
    }

    /// Adds a filter to the last query.
    pub fn filter<F>(mut self, f: F) -> Result<Self, QueryError>
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        if self.queries.is_empty() {
            return Err(QueryError::NoQueriesAdded);
        }
        if let Some(last) = self.queries.last_mut() {
            *last = std::mem::take(last).set_filter_fn(f);
        }
        Ok(self)
    }

    /// Convenience: query for a function with a specific name.
    pub fn function_with_name(mut self, name: String) -> Self {
        let query =
            format!(r#"(function_definition name: (identifier) @name (#eq? @name "{}"))"#, name);
        self.queries.push(
            Query::new()
                .set_query(query)
                .set_capture("name".to_string()),
        );
        self
    }

    /// Convenience: query for a class with a specific name.
    pub fn class_with_name(mut self, name: String) -> Self {
        let query =
            format!(r#"(class_definition name: (identifier) @name (#eq? @name "{}"))"#, name);
        self.queries.push(
            Query::new()
                .set_query(query)
                .set_capture("name".to_string()),
        );
        self
    }

    /// Convenience: query for any function definition.
    pub fn has_function(mut self) -> Self {
        let query = "(function_definition name: (identifier) @name)".to_string();
        self.queries.push(
            Query::new()
                .set_query(query)
                .set_capture("name".to_string()),
        );
        self
    }

    /// Convenience: query for any class definition.
    pub fn has_class(mut self) -> Self {
        let query = "(class_definition name: (identifier) @name)".to_string();
        self.queries.push(
            Query::new()
                .set_query(query)
                .set_capture("name".to_string()),
        );
        self
    }

    /// Convenience: check for list comprehension usage.
    pub fn uses_list_comprehension(mut self) -> Self {
        let query = "(list_comprehension) @comp".to_string();
        self.queries.push(
            Query::new()
                .set_query(query)
                .set_capture("comp".to_string()),
        );
        self
    }

    /// Convenience: check for for loop usage.
    pub fn uses_for_loop(mut self) -> Self {
        let query = "(for_statement) @loop".to_string();
        self.queries.push(
            Query::new()
                .set_query(query)
                .set_capture("loop".to_string()),
        );
        self
    }

    /// Convenience: check for while loop usage.
    pub fn uses_while_loop(mut self) -> Self {
        let query = "(while_statement) @loop".to_string();
        self.queries.push(
            Query::new()
                .set_query(query)
                .set_capture("loop".to_string()),
        );
        self
    }

    /// Convenience: check for if statement usage.
    pub fn uses_if_statement(mut self) -> Self {
        let query = "(if_statement) @stmt".to_string();
        self.queries.push(
            Query::new()
                .set_query(query)
                .set_capture("stmt".to_string()),
        );
        self
    }

    /// Builds and runs the grader.
    pub async fn run(self) -> Result<GradeResult> {
        self.grade_by_query().await
    }

    /// Performs the query grading.
    async fn grade_by_query(self) -> Result<GradeResult> {
        if self.queries.is_empty() {
            bail!("QueryGrader requires at least one query");
        }

        let file = self.project.identify(&self.file)?;
        let mut all_passed = true;
        let mut reasons = Vec::new();

        for (idx, query) in self.queries.iter().enumerate() {
            let query_str = query.query()?;
            let results = file.query(&query_str)?;

            // Apply filter if present
            let filtered_results: Vec<_> = if let Some(filter) = query.filter() {
                let capture_name = query.capture();
                results
                    .into_iter()
                    .filter(|r| {
                        if let Some(val) = r.get(&capture_name) {
                            filter(val)
                        } else {
                            true
                        }
                    })
                    .collect()
            } else {
                results
            };

            let match_count = filtered_results.len();
            let passed = match &self.constraint {
                QueryConstraint::MustMatchAtLeastOnce => match_count >= 1,
                QueryConstraint::MustMatchExactlyNTimes(n) => match_count == *n,
                QueryConstraint::MustNotMatch => match_count == 0,
            };

            if !passed {
                all_passed = false;
                let constraint_desc = match &self.constraint {
                    QueryConstraint::MustMatchAtLeastOnce => "at least 1 match".to_string(),
                    QueryConstraint::MustMatchExactlyNTimes(n) => format!("exactly {} matches", n),
                    QueryConstraint::MustNotMatch => "no matches".to_string(),
                };
                reasons.push(format!(
                    "Query {}: Expected {}, found {} matches",
                    idx + 1,
                    constraint_desc,
                    match_count
                ));
            }
        }

        let grade = if all_passed { self.out_of } else { 0.0 };
        let reason = if all_passed {
            "All queries passed".to_string()
        } else if !self.reason.is_empty() {
            self.reason.clone()
        } else {
            reasons.join("\n")
        };

        Ok(GradeResult::builder()
            .requirement(self.req_name)
            .grade(Grade::new(grade, self.out_of))
            .reason(reason)
            .build())
    }
}
