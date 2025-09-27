#![allow(unsafe_op_in_unsafe_fn)]

use pyo3::{Py, PyAny, prelude::*, types::PyDict};

use super::error::UmmError;
use crate::java::grade::results::GradeResult;

#[pyclass(module = "umm", name = "GradeResult")]
pub struct PyGradeResult {
    pub(crate) inner: GradeResult,
}

impl From<GradeResult> for PyGradeResult {
    fn from(inner: GradeResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyGradeResult {
    /// Requirement identifier.
    #[getter]
    pub fn requirement(&self) -> String {
        self.inner.requirement()
    }

    /// Numeric grade awarded for this requirement.
    #[getter]
    pub fn grade(&self) -> f64 {
        self.inner.grade()
    }

    /// Maximum achievable grade for this requirement.
    #[getter]
    pub fn out_of(&self) -> f64 {
        self.inner.out_of()
    }

    /// Explanation associated with the grade (may be empty).
    #[getter]
    pub fn reason(&self) -> String {
        self.inner.reason()
    }

    /// Prompt payload used when generating AI feedback, serialized to JSON if
    /// present.
    #[getter]
    pub fn prompt_json(&self) -> PyResult<Option<String>> {
        let prompt = self
            .inner
            .prompt()
            .map(|messages| serde_json::to_string(&messages))
            .transpose()
            .map_err(|err| PyErr::new::<UmmError, _>(err.to_string()))?;
        Ok(prompt)
    }

    pub fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        dict.set_item("requirement", self.requirement())?;
        dict.set_item("grade", self.grade())?;
        dict.set_item("out_of", self.out_of())?;
        dict.set_item("reason", self.reason())?;
        dict.set_item("prompt_json", self.prompt_json()?)?;
        Ok(dict.into())
    }
}
