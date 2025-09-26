#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

/// Retrieval and source context helpers.
pub mod context;
/// Diagnostic helper data structures.
pub mod diagnostics;
/// Diff-based grading utilities.
pub mod diff;
/// Documentation grading helpers.
pub mod docs;
/// Feedback generation helpers.
pub mod feedback;
/// Gradescope integration utilities.
pub mod gradescope;
/// Tree-sitter query grading components.
pub mod query;
/// Shared grade result types.
pub mod results;
/// Unit, mutation, and hidden test graders.
pub mod tests;

pub use context::{build_active_retrieval_context, build_heuristic_context, get_source_context};
pub use diagnostics::{JavacDiagnostic, MutationDiagnostic};
pub use diff::DiffGrader;
pub use docs::DocsGrader;
pub use feedback::{PromptRow, generate_feedback};
pub use gradescope::{
    GradescopeLeaderboardEntry, GradescopeOutputFormat, GradescopeStatus, GradescopeSubmission,
    GradescopeTestCase, GradescopeVisibility, show_result,
};
pub use query::{Query, QueryConstraint, QueryError, QueryGrader};
pub use results::{Grade, GradeResult};
pub use tests::{ByHiddenTestGrader, ByUnitTestGrader, UnitTestGrader};

pub use crate::types::LineRef;
