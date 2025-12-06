#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Python-specific grading utilities.

/// LLM-based code review grader.
pub mod code_review;
/// Retrieval and source context helpers.
pub mod context;
/// Diagnostic helper data structures.
pub mod diagnostics;
/// Diff-based grading utilities.
pub mod diff;
/// Documentation grading helpers.
pub mod docs;
/// Tree-sitter query grading components.
pub mod query;
/// Shared grade result types.
pub mod results;
/// Test graders (pytest, unittest).
pub mod tests;

pub use code_review::CodeReviewGrader;
pub use diff::{DiffCase, DiffGrader};
pub use docs::DocsGrader;
pub use query::{Query, QueryConstraint, QueryGrader};
pub use tests::TestGrader;

pub use crate::{
    java::grade::{Grade, GradeResult},
    types::LineRef,
};
