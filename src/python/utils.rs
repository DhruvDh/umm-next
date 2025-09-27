use std::path::PathBuf;

use pyo3::{Bound, PyErr, PyResult, prelude::PyAnyMethods, types::PyAny};
use rhai::{Array, Dynamic};

use super::error::UmmError;

pub(crate) fn path_from_any(any: &Bound<'_, PyAny>) -> PyResult<PathBuf> {
    any.extract::<PathBuf>()
        .map_err(|_| PyErr::new::<UmmError, _>("Expected a path-like object (str or os.PathLike)"))
}

pub(crate) fn sequence_of_strings(any: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
    any.extract::<Vec<String>>()
        .map_err(|_| PyErr::new::<UmmError, _>("Expected an iterable of strings"))
}

pub(crate) fn strings_to_rhai_array(strings: Vec<String>) -> Array {
    strings.into_iter().map(Dynamic::from).collect()
}
