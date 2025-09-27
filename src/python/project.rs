#![allow(unsafe_op_in_unsafe_fn)]

use pyo3::{
    Bound, PyErr,
    prelude::*,
    types::{PyAny, PyType},
};
use serde_json;

use super::{
    error::{UmmError, result_to_py},
    utils::path_from_any,
};
use crate::java::Project;

#[pyclass(module = "umm", name = "Project")]
pub struct PyProject {
    pub(crate) inner: Project,
}

#[pymethods]
impl PyProject {
    #[new]
    fn py_new() -> PyResult<Self> {
        let inner = result_to_py(Project::new())?;
        Ok(Self { inner })
    }

    #[classmethod]
    fn from_path(_cls: &Bound<'_, PyType>, path: &Bound<'_, PyAny>) -> PyResult<Self> {
        let root = path_from_any(path)?;
        let inner = result_to_py(Project::from_root(root))?;
        Ok(Self { inner })
    }

    /// Returns a human-readable outline of the project files.
    fn describe(&self) -> PyResult<String> {
        Ok(self.inner.describe())
    }

    /// Returns the file names discovered in the project.
    fn file_names(&self) -> PyResult<Vec<String>> {
        Ok(self
            .inner
            .files()
            .iter()
            .map(|file| file.proper_name())
            .collect())
    }

    /// Returns the serialized JSON representation of the project metadata.
    fn to_json(&self) -> PyResult<String> {
        let json = serde_json::to_string(&self.inner)
            .map_err(|err| PyErr::new::<UmmError, _>(err.to_string()))?;
        Ok(json)
    }
}

impl PyProject {
    pub(crate) fn clone_inner(&self) -> Project {
        self.inner.clone()
    }
}
