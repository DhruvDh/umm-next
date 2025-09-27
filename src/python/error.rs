use anyhow::Error as AnyhowError;
use pyo3::{PyErr, create_exception, exceptions::PyException};

use crate::java::grade::query::QueryError;

create_exception!(umm, UmmError, PyException);
create_exception!(umm, UmmConfigError, UmmError);
create_exception!(umm, UmmRuntimeError, UmmError);
create_exception!(umm, UmmQueryError, UmmError);

pub(crate) fn anyhow_to_py(err: AnyhowError) -> PyErr {
    PyErr::new::<UmmError, _>(err.to_string())
}

pub(crate) fn result_to_py<T>(result: anyhow::Result<T>) -> Result<T, PyErr> {
    result.map_err(anyhow_to_py)
}

pub(crate) fn query_error_to_py(err: QueryError) -> PyErr {
    PyErr::new::<UmmQueryError, _>(err.to_string())
}

pub(crate) fn query_result_to_py<T>(result: Result<T, QueryError>) -> Result<T, PyErr> {
    result.map_err(query_error_to_py)
}
