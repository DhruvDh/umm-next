Below is a **hands‑on, end‑to‑end guide** to (1) *discover* what needs to be exposed from your Rust codebase, (2) *design and wrap* it cleanly for Python using the builder pattern you prefer, and (3) *validate* that the Python surface exactly mirrors the Rust capabilities—covering common, tricky, and edge‑case method shapes.

> The examples below use PyO3 and assume grader‑like types (e.g., `Project`, `DocsGrader`, `ByUnitTestGrader`, `HiddenTestGrader`, `DiffGrader`, `QueryGrader`, `GradeResult`, etc.). If your exact names differ, treat them as placeholders.

---

## 0) High‑level goals

* **Expose** your Rust *domain objects* and *builder methods* to Python with intuitive, chainable APIs.
* **Keep business logic in Rust**, using Python as a thin orchestration layer.
* **Guarantee parity**: every public Rust capability that matters in practice is accessible and behaves the same in Python.
* **Make it pleasant**: clean signatures, robust type conversions, good error messages, no surprises.

---

## 1) Systematic discovery: what to expose

Run these repo scans to enumerate the candidate API surface. You’re looking for:
*public structs*, *public builders*, *constructors*, *chainable setters*, *“runner” methods* (e.g., `run`, `grade`, `execute`), and any **callbacks** or **filters**.

> **Tip:** Replace `src` if your code is split into crates.

### 1.1 Find graders and core domain structs

```bash
rg -n --glob '!target' 'pub struct .*Grader'
rg -n --glob '!target' 'pub struct (Project|.*Config|.*Result|.*Query|.*Constraint)'
rg -n --glob '!target' 'enum .* (Visibility|Level|Constraint|Mode|Kind|Status)'
```

### 1.2 Find builder-y methods and constructors

```bash
# Associated constructors
rg -n --glob '!target' 'impl .* \{[\s\S]*?pub fn (new|from|default)\('

# Chainable setter patterns (adapt if you use different verbs)
rg -n --glob '!target' 'pub fn (with_|set_|add_|files?|inputs?|expected|project|req|name|out_of|points|penalty|case_insensitive|url|class|package|constraint|query)\('
```

### 1.3 Find “runner”/terminal methods

```bash
rg -n --glob '!target' 'pub fn (run|grade|execute|apply|finalize)\('
```

### 1.4 Find callback/closure-accepting APIs (trickiest to expose)

```bash
rg -n --glob '!target' 'Fn\(|FnMut\(|FnOnce\('
rg -n --glob '!target' 'impl.*<.*F:.*Fn'   # generics with function bounds
```

### 1.5 Find public types used in the APIs (for conversion planning)

```bash
rg -n --glob '!target' 'pub type '
rg -n --glob '!target' '-> (Result|Option|Vec|HashMap|HashSet|Path|PathBuf|Cow|Duration|NonZero|u128|i128)'
```

> **Output:** a working inventory (CSV or Markdown) that lists: **Type**, **Method**, **Receiver kind** (by‑value `self`, `&mut self`, `&self`, associated), **Args**, **Return type**, **Notes** (generic? closure? lifetime?).

---

## 2) Method‑shape taxonomy & how to expose each

Builder APIs come in many shapes. Below are the usual suspects and how to wrap them for Python.

> **Core pattern for chainability:** In PyO3, make builder methods accept a `PyRefMut<Self>` (instead of `&mut self`) and return the same `PyRefMut<Self>`. This lets you **chain calls naturally in Python**.

### 2.1 Constructors (associated functions)

* **Rust:** `impl Foo { pub fn new(...) -> Self { ... } }`
* **Python exposure:** `#[pymethods] #[new] fn py_new(...) -> Self { ... }`
* **Tip:** Keep “smart constructors” as `#[classmethod]` if you want named constructors, e.g., `Foo.from_path(path)`.

### 2.2 Chainable setters (mutating builder)

* **Rust:** `pub fn files(&mut self, files: Vec<String>) -> &mut Self`
* **Python:**

  ```rust
  #[pymethods]
  impl PyDocsGrader {
      fn files<'py>(mut slf: PyRefMut<'py, Self>, files: &PyAny) -> PyResult<PyRefMut<'py, Self>> {
          slf.inner.files(vec_of_str(files)?);
          Ok(slf)
      }
  }
  ```

  *`vec_of_str` is a helper you’ll write; see §4.1.*

### 2.3 Consuming builders

* **Rust:** `pub fn files(self, files: Vec<String>) -> Self`
* **Python wrapping pattern:** take and return `PyRefMut<Self>` and **replace** the inner object:

  ```rust
  fn files<'py>(mut slf: PyRefMut<'py, Self>, files: &PyAny) -> PyResult<PyRefMut<'py, Self>> {
      let inner = std::mem::take(&mut slf.inner);
      slf.inner = inner.files(vec_of_str(files)?);
      Ok(slf)
  }
  ```

### 2.4 Getters / inspectors

* **Rust:** `pub fn out_of(&self) -> f64`
* **Python:** return a simple Python type (`f64` → `float`).

  ```rust
  fn out_of(&self) -> f64 { self.inner.out_of() }
  ```

### 2.5 Terminal “runner”

* **Rust:** `pub fn run(&mut self) -> Result<GradeResult, Error>`
* **Python:** release the GIL during heavy work and convert `Result` to exceptions/values.

  ```rust
  fn run(&mut self, py: Python<'_>) -> PyResult<PyGradeResult> {
      let res = py.allow_threads(|| self.inner.run()).map_err(to_pyerr)?;
      Ok(PyGradeResult::from(res))
  }
  ```

### 2.6 Enums (constraints, visibility, modes)

* Expose as Python **enum.Enum** or accept **string literals** and map to Rust enums.
* Prefer enums for IDE help & static checking; accept strings as a convenience with validation.

### 2.7 Callbacks / filters (advanced)

* **Rust:** `fn filter<F>(&mut self, f: F) -> &mut Self where F: Fn(&AstNode) -> bool + Send + 'static`
* **Python:** accept any Python callable, store as `Py<PyAny>`, and call it when needed:

  ```rust
  #[pyclass]
  pub struct PyQuery {
      inner: Query,
      filter: Option<Py<PyAny>>,
  }

  #[pymethods]
  impl PyQuery {
      fn filter<'py>(mut slf: PyRefMut<'py, Self>, f: PyObject) -> PyResult<PyRefMut<'py, Self>> {
          slf.filter = Some(f.into());
          Ok(slf)
      }
  }

  // Later, when applying:
  if let Some(py_cb) = &self.filter {
      let ok = Python::with_gil(|py| -> PyResult<bool> {
          let ret = py_cb.call1(py, (node_to_py(py, node)?,))?;
          ret.extract(py)
      })?;
      if !ok { /* ... */ }
  }
  ```

  * **Be careful** with performance: batch calls, minimize GIL churn, and keep callback optional.

### 2.8 Generics / trait bounds

* Specialize to concrete types at the Python boundary. If a method is generic over `AsRef<Path>`, expose a Python signature that accepts `str | os.PathLike` and convert to `PathBuf` internally (see §4.2).

### 2.9 Lifetimes & borrowed data

* **Rule:** Do **not** expose borrowed references (`&str`, `&[T]`) directly to Python. Convert to **owned** (`String`, `Vec<T>`) before storing or returning. Keep Python‑visible types owning their data.

---

## 3) Designing the Python‑facing API (what Python users should see)

Aim for a small, coherent set:

* **Project/Workspace**
* **Graders**: `DocsGrader`, `ByUnitTestGrader`, `UnitTestGrader`, `HiddenTestGrader`, `DiffGrader`, `QueryGrader`
* **Supporting types**: `Query`, `Constraint`, `GradeResult`, `Visibility` (and any relevant config objects)
* **Utilities**: `show_results(results)`, `generate_feedback(results, ...)` (if part of your product)

**Example (Python):**

```python
from umm import Project, DocsGrader, ByUnitTestGrader, Query, Constraint, show_results

proj = Project.from_path(".")
docs = (
    DocsGrader()
    .project(proj)
    .files(["MyClass"])
    .out_of(10.0)
    .req_name("Docs")
)
tests = (
    ByUnitTestGrader()
    .project(proj)
    .test_files(["MyClassTest"])
    .expected_tests(["test_foo", "test_bar"])
    .out_of(20.0)
    .req_name("Student Tests")
)

r1 = docs.run()
r2 = tests.run()
show_results([r1, r2])
```

---

## 4) Implementation patterns with PyO3

### 4.1 Conversion helpers (build once, reuse everywhere)

```rust
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PySequence};
use std::path::PathBuf;

fn vec_of_str(obj: &PyAny) -> PyResult<Vec<String>> {
    if let Ok(s) = obj.extract::<String>() {
        return Ok(vec![s]);
    }
    if let Ok(seq) = obj.downcast::<PySequence>() {
        let mut out = Vec::with_capacity(seq.len()? as usize);
        for item in seq.iter()? {
            out.push(item?.extract::<String>()?);
        }
        return Ok(out);
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "Expected str or sequence of str",
    ))
}

fn pathbuf_from_any(obj: &PyAny) -> PyResult<PathBuf> {
    // Accept str or os.PathLike
    if let Ok(s) = obj.extract::<String>() {
        return Ok(PathBuf::from(s));
    }
    // __fspath__ protocol
    if let Ok(pathlike) = obj.getattr("__fspath__") {
        let s = pathlike.call0()?.extract::<String>()?;
        return Ok(PathBuf::from(s));
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "Expected str or os.PathLike",
    ))
}
```

### 4.2 Error mapping

Create a top‑level exception and `From` conversions:

```rust
use pyo3::exceptions::{PyException, PyRuntimeError, PyValueError};
use pyo3::create_exception;

create_exception!(umm, UmmError, PyException);

fn to_pyerr<E: std::error::Error + Send + Sync + 'static>(e: E) -> PyErr {
    // Map by type or message; add variants if you have distinct error kinds.
    PyRuntimeError::new_err(format!("Rust error: {}", e))
}
```

### 4.3 Wrapping a canonical grader with chainable builders

```rust
use pyo3::prelude::*;

#[pyclass(module = "umm")]
pub struct PyDocsGrader {
    inner: DocsGrader, // your real Rust type
}

#[pymethods]
impl PyDocsGrader {
    #[new]
    fn py_new() -> Self {
        Self { inner: DocsGrader::default() }
    }

    fn project<'py>(mut slf: PyRefMut<'py, Self>, project: PyRef<'py, PyProject>)
        -> PyResult<PyRefMut<'py, Self>>
    {
        slf.inner.project(project.inner.clone());
        Ok(slf)
    }

    fn files<'py>(mut slf: PyRefMut<'py, Self>, files: &PyAny) -> PyResult<PyRefMut<'py, Self>> {
        slf.inner.files(vec_of_str(files)?);
        Ok(slf)
    }

    #[pyo3(name = "out_of")]
    fn out_of_points<'py>(mut slf: PyRefMut<'py, Self>, points: f64)
        -> PyResult<PyRefMut<'py, Self>>
    {
        slf.inner.out_of(points);
        Ok(slf)
    }

    fn req_name<'py>(mut slf: PyRefMut<'py, Self>, name: String) -> PyResult<PyRefMut<'py, Self>> {
        slf.inner.req_name(name);
        Ok(slf)
    }

    fn run(&mut self, py: Python<'_>) -> PyResult<PyGradeResult> {
        let res = py.allow_threads(|| self.inner.run()).map_err(to_pyerr)?;
        Ok(PyGradeResult::from(res))
    }
}
```

> **Why `PyRefMut`?** It allows `return self` for chaining without moving/duplicating the Python object.

### 4.4 Wrapping a support type (e.g., `Project`)

```rust
#[pyclass(module = "umm")]
pub struct PyProject { pub(crate) inner: Project }

#[pymethods]
impl PyProject {
    #[new]
    fn py_new() -> Self { Self { inner: Project::default() } }

    #[classmethod]
    fn from_path(_cls: &PyType, path: &PyAny) -> PyResult<Self> {
        let p = pathbuf_from_any(path)?;
        Ok(Self { inner: Project::from_path(p) })
    }
}
```

### 4.5 Returning rich results (e.g., `GradeResult`)

```rust
#[pyclass(module = "umm")]
pub struct PyGradeResult {
    requirement: String,
    grade: f64,
    out_of: f64,
    reason: Option<String>,
    // add fields you want visible in Python
}

impl From<GradeResult> for PyGradeResult {
    fn from(r: GradeResult) -> Self {
        Self {
            requirement: r.requirement,
            grade: r.grade,
            out_of: r.out_of,
            reason: r.reason,
        }
    }
}

#[pymethods]
impl PyGradeResult {
    #[getter] fn requirement(&self) -> &str { &self.requirement }
    #[getter] fn grade(&self) -> f64 { self.grade }
    #[getter] fn out_of(&self) -> f64 { self.out_of }
    #[getter] fn reason(&self) -> Option<&str> { self.reason.as_deref() }
}
```

### 4.6 Enums (Pythonic and ergonomic)

Option A (stringly‑typed, with validation):

```rust
#[pyfunction]
fn set_visibility(grader: &mut PyDocsGrader, vis: &str) -> PyResult<()> {
    let v = match vis.to_ascii_lowercase().as_str() {
        "hidden" => Visibility::Hidden,
        "visible" => Visibility::Visible,
        _ => return Err(PyValueError::new_err("Expected 'hidden' or 'visible'"))
    };
    grader.inner.visibility(v);
    Ok(())
}
```

Option B (export real Python enums; recommended if the set is small and stable):

```rust
#[pyclass(module = "umm")]
#[derive(Clone, Copy)]
pub struct PyVisibility(Visibility);

#[pymethods]
impl PyVisibility {
    #[classattr] pub const Hidden: Self = Self(Visibility::Hidden);
    #[classattr] pub const Visible: Self = Self(Visibility::Visible);
}
```

### 4.7 Exposing callbacks safely

* Store `Py<PyAny>` in the builder.
* Call inside `Python::with_gil` and catch exceptions, mapping to your error.
* For performance, **avoid calling per element** in a hot loop; pre‑filter if possible or allow **vectorized** predicates (call once with a list).

---

## 5) Module layout & packaging (with maturin)

**`Cargo.toml`**

```toml
[package]
name = "umm"
edition = "2021"

[lib]
name = "umm"
crate-type = ["cdylib"]   # required for Python extension

[dependencies]
pyo3 = { version = "0.22", features = ["extension-module"] }
# your other deps...

[features]
default = ["python-api"]
python-api = []
```

**`pyproject.toml`**

```toml
[build-system]
requires = ["maturin>=1.5"]
build-backend = "maturin"

[project]
name = "umm"
version = "0.1.0"
requires-python = ">=3.9"
description = "Umm grading API (Python bindings to Rust)"
```

Build locally:

```bash
maturin develop  # dev install into current venv
# or
maturin build --release  # create wheels
```

> Generate `.pyi` type stubs: use `pyo3-stubgen` or `maturin develop --extras` and integrate `stubgen` in CI (see §8).

---

## 6) Validation plan (exhaustive & automated)

### 6.1 API parity checklist

Create a machine‑readable spec from your inventory step:

* **For each Rust type** you plan to expose:

  * [ ] Constructor exposed
  * [ ] Every chainable setter exposed
  * [ ] Terminal method(s) exposed
  * [ ] Getters exposed (if useful)
  * [ ] Enums available and documented
  * [ ] Errors mapped

Keep this spec in `api_parity.yaml` and **assert it in tests** (see next).

### 6.2 Python‑side tests (pytest)

* **Smoke chaining works**

  ```python
  def test_docs_chaining(project):
      from umm import DocsGrader
      r = (
        DocsGrader()
        .project(project)
        .files(["A"])
        .out_of(10.0)
        .req_name("Docs")
        .run()
      )
      assert r.out_of == 10.0
  ```

* **Parity tests vs. Rust direct calls**
  Compile a tiny Rust “reference” program (or use `#[cfg(test)]`) that runs the same grader with same inputs. Compare JSON‑serialized `GradeResult` (Rust) vs. Python’s `PyGradeResult` to ensure **identical** fields.

* **Callback tests** (if any)
  Verify Python callable receives the right shape and filters as expected; test exceptions propagate with helpful messages.

* **Type conversion tests**
  – `str`/`PathLike` to `PathBuf`
  – `list`/`tuple`/`iterable` to `Vec<String>`
  – Options map to `None`/values

* **GIL & threads**
  Confirm long operations release GIL (`allow_threads`) by running a concurrent Python thread during `.run()` and asserting it makes progress.

### 6.3 Rust‑side tests (unit & integration)

* **Wrapper compile tests**: use the builder wrappers in Rust tests via `Python::with_gil` to ensure the PyO3 signatures remain valid after refactors.
* **`#[deny(missing_docs)]` + rustdoc** for the Python‑visible types to keep docs fresh.

### 6.4 Contract tests: method presence

At import time, assert the Python module really exposes what you expect:

```python
def test_api_surface():
    import umm, inspect
    present = set(dir(umm))
    required = {"Project", "DocsGrader", "ByUnitTestGrader", "GradeResult"}  # etc
    missing = required - present
    assert not missing, f"Missing API: {missing}"
```

---

## 7) Edge cases & patterns to standardize

1. **Multiple “input forms”** (e.g., `files`: can be a single string or a list): support both.
2. **Numeric types**: prefer `f64`/`i64` at the boundary; document units.
3. **Enums**: accept either enum objects or strings (validate strictly).
4. **Ownership**: never store Python references to data that Rust may drop; convert to owned Rust types.
5. **Diagnostics**: add `.debug()` or `.explain()` on results if you want richer feedback, and expose them to Python.
6. **Logging**: provide an optional `log_to(file_or_stream)` for graders; in Python, allow passing a path or `sys.stdout`/`sys.stderr` (map streams carefully).
7. **Time limits**: if `.run()` could hang, offer `.timeout(seconds)` as a builder step; enforce in Rust, report a specific Python exception on timeout.
8. **Determinism**: make it easy to seed any randomness (`.seed(n)`).

---

## 8) Developer ergonomics for Python users

* **Docstrings**: Use `#[pyo3(text_signature = "(..., ...)")]` and doc comments on `#[pymethods]` so `help(umm.DocsGrader)` is informative.
* **Type hints**: ship `.pyi` stubs. Add a small CI job:

  ```bash
  maturin develop
  python -m pyo3_stubgen umm -o stubs
  # verify stubs exist & include in sdist/wheel
  ```
* **Meaningful exceptions**: map your domain errors to `UmmError`, `UmmConfigError`, `UmmRuntimeError`, etc., not just `RuntimeError`.

---

## 9) Migration helpers (from Rhai/Rune to Python)

* Keep Python method names **close** to the old script names (`out_of`, `req_name`, `test_files`, `expected_tests`, etc.).
* Provide a **migration cheatsheet** mapping old script syntax to Python calls.
* If you had “global” functions before (e.g., `show_results(results)`), expose them as `#[pyfunction]` at module level for familiarity.

---

## 10) Worked “mini‑module” example

```rust
use pyo3::prelude::*;
mod utils; // vec_of_str, pathbuf_from_any, to_pyerr, etc.

#[pyclass(module = "umm")]
pub struct PyProject { pub(crate) inner: Project }

#[pymethods]
impl PyProject {
    #[classmethod]
    fn from_path(_cls: &PyType, path: &PyAny) -> PyResult<Self> {
        Ok(Self { inner: Project::from_path(utils::pathbuf_from_any(path)?) })
    }
}

#[pyclass(module = "umm")]
pub struct PyDocsGrader { inner: DocsGrader }

#[pymethods]
impl PyDocsGrader {
    #[new] fn py_new() -> Self { Self { inner: DocsGrader::default() } }

    fn project<'py>(mut slf: PyRefMut<'py, Self>, proj: PyRef<'py, PyProject>) -> PyResult<PyRefMut<'py, Self>> {
        slf.inner.project(proj.inner.clone());
        Ok(slf)
    }
    fn files<'py>(mut slf: PyRefMut<'py, Self>, files: &PyAny) -> PyResult<PyRefMut<'py, Self>> {
        slf.inner.files(utils::vec_of_str(files)?);
        Ok(slf)
    }
    fn out_of<'py>(mut slf: PyRefMut<'py, Self>, points: f64) -> PyResult<PyRefMut<'py, Self>> {
        slf.inner.out_of(points);
        Ok(slf)
    }
    fn req_name<'py>(mut slf: PyRefMut<'py, Self>, name: String) -> PyResult<PyRefMut<'py, Self>> {
        slf.inner.req_name(name);
        Ok(slf)
    }
    fn run(&mut self, py: Python<'_>) -> PyResult<PyGradeResult> {
        let r = py.allow_threads(|| self.inner.run()).map_err(utils::to_pyerr)?;
        Ok(PyGradeResult::from(r))
    }
}

#[pyclass(module = "umm")]
pub struct PyGradeResult { /* fields */ }

#[pymethods]
impl PyGradeResult {
    // getters...
}

#[pymodule]
fn umm(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyProject>()?;
    m.add_class::<PyDocsGrader>()?;
    m.add_class::<PyGradeResult>()?;
    // add others...
    Ok(())
}
```

---

## 11) “What to expose” decision rubric

For each Rust type you find:

* **Is it a *configuration* or *domain* concept?** → Expose.
* **Is it an internal helper?** → Hide behind higher‑level methods; don’t expose unless needed for power users.
* **Does it carry lifetimes/borrows?** → Wrap into an owned, Python‑safe facade.
* **Is it generic over traits the Python user won’t understand?** → Provide concrete versions at the boundary.

---

## 12) Final checklists

### Python API consistency

* [ ] All builders chain: return `PyRefMut<Self>`
* [ ] All long ops use `py.allow_threads`
* [ ] All paths accept `str | os.PathLike`
* [ ] All collections accept any Python sequence
* [ ] Enum arguments accept enum or validated string
* [ ] All errors map to project exceptions
* [ ] Docstrings & type hints present

### Testing coverage

* [ ] One golden test per grader comparing Rust vs Python result
* [ ] Callback paths covered (success + exception)
* [ ] Unicode / weird filenames in path conversions
* [ ] Empty/invalid inputs (nice messages)
* [ ] Concurrency sanity (GIL released)

---

## 13) Where to go deeper (optional, but nice)

* **Macro‑generating wrappers**: write a `#[py_builder]` proc‑macro that, given a Rust impl block, emits the PyO3 `PyRefMut` chainers automatically.
* **Streaming logs to Python**: expose a `.on_log(callable)` that receives structured events (`{"phase":"compile","line":"..."}`).
* **Timeouts / cancellation**: add `.cancel_token()` or `.timeout()` builders and wire to Python `asyncio` if desired (with a separate async module).

---

### TL;DR

1. **Inventory** your public graders & builders (ripgrep snippets above).
2. **Wrap** them with PyO3 using the `PyRefMut<Self> -> PyRefMut<Self>` chain pattern.
3. **Convert** Python types carefully (paths, sequences, enums, callbacks).
4. **Validate parity** with golden tests and surface checks.
5. **Polish ergonomics** (docstrings, type hints, exceptions).

Follow this guide and you’ll have a Python layer that feels native and friendly, while all the heavy lifting stays robustly in Rust.
