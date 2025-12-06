#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Shared grade result types for Python (re-exports from Java for consistency).

// Re-export from Java module to maintain API consistency
pub use crate::java::grade::results::{Grade, GradeResult};
