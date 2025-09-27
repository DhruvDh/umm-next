#![allow(unsafe_op_in_unsafe_fn)]

use std::mem;

use pyo3::{
    Bound,
    prelude::{PyAnyMethods, *},
    types::PyAny,
};

use super::{
    error::{UmmError, anyhow_to_py, query_result_to_py},
    project::PyProject,
    results::PyGradeResult,
    utils::{sequence_of_strings, strings_to_rhai_array},
};
use crate::java::grade::{
    diff::DiffGrader, docs::DocsGrader, query::QueryGrader, tests::ByUnitTestGrader,
};

#[pyclass(module = "umm", name = "DocsGrader")]
pub struct PyDocsGrader {
    inner: DocsGrader,
}

impl Default for PyDocsGrader {
    fn default() -> Self {
        Self {
            inner: DocsGrader::default(),
        }
    }
}

#[pymethods]
impl PyDocsGrader {
    #[new]
    fn py_new() -> Self {
        Self::default()
    }

    fn project<'py>(
        mut slf: PyRefMut<'py, Self>,
        project: &PyProject,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_project(project.clone_inner());
        Ok(slf)
    }

    fn files<'py>(
        mut slf: PyRefMut<'py, Self>,
        files: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let entries = sequence_of_strings(files)?;
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_files(strings_to_rhai_array(entries));
        Ok(slf)
    }

    fn out_of<'py>(mut slf: PyRefMut<'py, Self>, points: f64) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_out_of(points);
        Ok(slf)
    }

    fn req_name<'py>(mut slf: PyRefMut<'py, Self>, name: String) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_req_name(name);
        Ok(slf)
    }

    fn penalty<'py>(mut slf: PyRefMut<'py, Self>, value: f64) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_penalty(value);
        Ok(slf)
    }

    fn run(&self, py: Python<'_>) -> PyResult<PyGradeResult> {
        let grader = self.inner.clone();
        let result = py.detach(move || grader.grade_docs());
        let grade_result = result.map_err(anyhow_to_py)?;
        Ok(PyGradeResult::from(grade_result))
    }
}

#[pyclass(module = "umm", name = "DiffGrader")]
pub struct PyDiffGrader {
    inner: DiffGrader,
}

impl Default for PyDiffGrader {
    fn default() -> Self {
        Self {
            inner: DiffGrader::default(),
        }
    }
}

#[pymethods]
impl PyDiffGrader {
    #[new]
    fn py_new() -> Self {
        Self::default()
    }

    fn project<'py>(
        mut slf: PyRefMut<'py, Self>,
        project: &PyProject,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_project(project.clone_inner());
        Ok(slf)
    }

    fn file<'py>(mut slf: PyRefMut<'py, Self>, file: String) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_file(file);
        Ok(slf)
    }

    fn req_name<'py>(mut slf: PyRefMut<'py, Self>, name: String) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_req_name(name);
        Ok(slf)
    }

    fn out_of<'py>(mut slf: PyRefMut<'py, Self>, points: f64) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_out_of(points);
        Ok(slf)
    }

    fn ignore_case<'py>(
        mut slf: PyRefMut<'py, Self>,
        enabled: bool,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_ignore_case(enabled);
        Ok(slf)
    }

    /// Accepts an iterable of `(input, expected)` string pairs.
    fn cases<'py>(
        mut slf: PyRefMut<'py, Self>,
        cases: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let parsed: Vec<(String, String)> = cases.extract().map_err(|_| {
            PyErr::new::<UmmError, _>("Expected an iterable of (input, expected) string pairs")
        })?;

        let mut inputs = Vec::with_capacity(parsed.len());
        let mut expected = Vec::with_capacity(parsed.len());

        for (input, expected_val) in parsed {
            inputs.push(input);
            expected.push(expected_val);
        }

        let mut inner = mem::take(&mut slf.inner);
        inner = inner.set_input(strings_to_rhai_array(inputs));
        inner = inner.set_expected(strings_to_rhai_array(expected));
        slf.inner = inner;
        Ok(slf)
    }

    fn run(&self, py: Python<'_>) -> PyResult<PyGradeResult> {
        let grader = self.inner.clone();
        let result = py.detach(move || grader.grade_by_diff());
        let grade_result = result.map_err(anyhow_to_py)?;
        Ok(PyGradeResult::from(grade_result))
    }
}

#[pyclass(module = "umm", name = "ByUnitTestGrader")]
pub struct PyByUnitTestGrader {
    inner: ByUnitTestGrader,
}

impl Default for PyByUnitTestGrader {
    fn default() -> Self {
        Self {
            inner: ByUnitTestGrader::default(),
        }
    }
}

#[pymethods]
impl PyByUnitTestGrader {
    #[new]
    fn py_new() -> Self {
        Self::default()
    }

    fn project<'py>(
        mut slf: PyRefMut<'py, Self>,
        project: &PyProject,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_project(project.clone_inner());
        Ok(slf)
    }

    fn test_files<'py>(
        mut slf: PyRefMut<'py, Self>,
        files: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let entries = sequence_of_strings(files)?;
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_test_files(strings_to_rhai_array(entries));
        Ok(slf)
    }

    fn expected_tests<'py>(
        mut slf: PyRefMut<'py, Self>,
        tests: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let entries = sequence_of_strings(tests)?;
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_expected_tests(strings_to_rhai_array(entries));
        Ok(slf)
    }

    fn out_of<'py>(mut slf: PyRefMut<'py, Self>, points: f64) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_out_of(points);
        Ok(slf)
    }

    fn req_name<'py>(mut slf: PyRefMut<'py, Self>, name: String) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_req_name(name);
        Ok(slf)
    }

    fn run(&self, py: Python<'_>) -> PyResult<PyGradeResult> {
        let grader = self.inner.clone();
        let result = py.detach(move || grader.grade_by_tests());
        let grade_result = result.map_err(anyhow_to_py)?;
        Ok(PyGradeResult::from(grade_result))
    }
}

#[pyclass(module = "umm", name = "QueryGrader")]
pub struct PyQueryGrader {
    inner: QueryGrader,
}

impl Default for PyQueryGrader {
    fn default() -> Self {
        Self {
            inner: QueryGrader::default(),
        }
    }
}

#[pymethods]
impl PyQueryGrader {
    #[new]
    fn py_new() -> Self {
        Self::default()
    }

    fn project<'py>(
        mut slf: PyRefMut<'py, Self>,
        project: &PyProject,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_project(project.clone_inner());
        Ok(slf)
    }

    fn file<'py>(mut slf: PyRefMut<'py, Self>, file: String) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_file(file);
        Ok(slf)
    }

    fn req_name<'py>(mut slf: PyRefMut<'py, Self>, name: String) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_req_name(name);
        Ok(slf)
    }

    fn out_of<'py>(mut slf: PyRefMut<'py, Self>, points: f64) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_out_of(points);
        Ok(slf)
    }

    fn reason<'py>(mut slf: PyRefMut<'py, Self>, reason: String) -> PyResult<PyRefMut<'py, Self>> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.set_reason(reason);
        Ok(slf)
    }

    fn must_match_at_least_once<'py>(mut slf: PyRefMut<'py, Self>) -> PyRefMut<'py, Self> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.must_match_at_least_once();
        slf
    }

    fn must_match_exactly<'py>(mut slf: PyRefMut<'py, Self>, count: usize) -> PyRefMut<'py, Self> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.must_match_exactly_n_times(count);
        slf
    }

    fn must_not_match<'py>(mut slf: PyRefMut<'py, Self>) -> PyRefMut<'py, Self> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.must_not_match();
        slf
    }

    /// Adds a raw query/capture pair to the grader.
    fn query<'py>(
        mut slf: PyRefMut<'py, Self>,
        query: String,
        capture: String,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let mut inner = mem::take(&mut slf.inner);
        inner = query_result_to_py(inner.query(query))?;
        inner = query_result_to_py(inner.capture(capture))?;
        slf.inner = inner;
        Ok(slf)
    }

    /// Convenience helper for common method-invocation queries.
    fn method_invocations_with_name<'py>(
        mut slf: PyRefMut<'py, Self>,
        method: String,
    ) -> PyRefMut<'py, Self> {
        let inner = mem::take(&mut slf.inner);
        slf.inner = inner.method_invocations_with_name(method);
        slf
    }

    fn run(&self, py: Python<'_>) -> PyResult<PyGradeResult> {
        let grader = self.inner.clone();
        let result = py.detach(move || grader.grade_by_query());
        let grade_result = result.map_err(anyhow_to_py)?;
        Ok(PyGradeResult::from(grade_result))
    }
}
