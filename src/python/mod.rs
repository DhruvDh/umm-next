#![allow(unsafe_op_in_unsafe_fn)]

use pyo3::{Bound, prelude::*, types::PyModule};

mod error;
mod graders;
mod project;
mod results;
mod utils;

use error::{UmmConfigError, UmmError, UmmQueryError, UmmRuntimeError};
use graders::{PyByUnitTestGrader, PyDiffGrader, PyDocsGrader, PyQueryGrader};
use project::PyProject;
use results::PyGradeResult;

/// PyO3 entry module. This will be populated with concrete bindings as
/// we expose the Rust API surface.
#[pymodule]
pub fn umm(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("UmmError", py.get_type::<UmmError>())?;
    m.add("UmmConfigError", py.get_type::<UmmConfigError>())?;
    m.add("UmmRuntimeError", py.get_type::<UmmRuntimeError>())?;
    m.add("UmmQueryError", py.get_type::<UmmQueryError>())?;

    m.add_class::<PyProject>()?;
    m.add_class::<PyGradeResult>()?;
    m.add_class::<PyDocsGrader>()?;
    m.add_class::<PyDiffGrader>()?;
    m.add_class::<PyByUnitTestGrader>()?;
    m.add_class::<PyQueryGrader>()?;

    Ok(())
}
