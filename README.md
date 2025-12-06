# umm

- [umm](#umm)
  - [Introduction](#introduction)
  - [Documentation](#documentation)
  - [Installation](#installation)
  - [Auto-grading](#auto-grading)
    - [Sample grading script (Java)](#sample-grading-script-java)
    - [Sample grading script (Python)](#sample-grading-script-python)
    - [Output](#output)
  - [Configuration](#configuration)
  - [License](#license)
  - [Scripting API Reference](#scripting-api-reference)

## Introduction

A build tool for novices, that doubles as a scriptable autograder for Java and Python.

## Documentation

You can generate the API documentation locally:
```sh
cargo doc --open
```

## Installation

You would need rust installed, ideally the nightly toolchain. You can visit https://rustup.rs/ to find out how to install this on your computer, just make sure you install the "nightly" toolchain instead of stable.

On Linux, Windows Subsystem for Linux (WSL), and Mac you should be able to run `curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly` on a terminal to install the nightly toolchain for rust.

Once you are done, just type `cargo install --git=https://github.com/DhruvDh/umm-next.git`, and it should compile and install it on your system.

If you intend to use `umm` for Python grading, you must also install [`uv`](https://docs.astral.sh/uv/). `umm` relies on `uv` to manage virtual environments and dependencies.

## Auto-grading

`umm` now runs grading flows written in [Rune](https://rune-rs.github.io/). Ship a script with an async `main` function and execute it with `umm java grade path/to/script.rn` or `umm python grade path/to/script.rn`.

### Sample grading script (Java)

This script demonstrates a comprehensive Java grading flow: documentation checking, output comparison, unit tests, tree-sitter structure queries, mutation testing, and hidden tests.

```rust
use umm::java::{
    grade_all,
    new_by_hidden_test_grader,
    new_by_unit_test_grader,
    new_diff_grader,
    new_docs_grader,
    new_project,
    new_query_grader,
    new_unit_test_grader,
    show_results,
    QueryConstraint,
};

pub async fn main() {
    let project = new_project()?;

    // 1. Check Documentation (JavaDoc)
    let req_1 = new_docs_grader()
        .project(project.clone())
        .files(["pyramid_scheme.LinkedTree"])
        .out_of(5.0)
        .req_name("1")
        .penalty(1.0)
        .run()
        .await?;

    // 2. Check Standard Output (Diff)
    let req_2 = new_diff_grader()
        .project(project.clone())
        .file("Main")
        .req_name("2")
        .out_of(5.0)
        .cases([("Hello from Rune\n", None)])
        .run()
        .await?;

    // 3. Run Visible Unit Tests
    let req_3 = new_by_unit_test_grader()
        .project(project.clone())
        .test_files(["pyramid_scheme.LinkedTreeTest"])
        .expected_tests([
            "pyramid_scheme.LinkedTreeTest#testGetRootElement",
            "pyramid_scheme.LinkedTreeTest#testAddChild",
        ])
        .out_of(5.0)
        .req_name("3")
        .run()
        .await?;

    // 4. Structural Code Query (Tree-sitter)
    let req_4 = new_query_grader()
        .project(project.clone())
        .file("Main")
        .queries_with_capture([("((for_statement) @loop)", "loop")])
        .constraint(QueryConstraint::must_match_at_least_once())
        .out_of(5.0)
        .req_name("4")
        .reason("Should contain a for loop")
        .run()
        .await?;

    // 5. Mutation Testing (PITest)
    let req_5 = new_unit_test_grader()
        .project(project.clone())
        .target_test(["pyramid_scheme.LinkedTreeTest"])
        .target_class(["pyramid_scheme.LinkedTree"])
        .excluded_methods([])
        .avoid_calls_to([])
        .req_name("5")
        .out_of(5.0)
        .run()
        .await?;

    // 6. Hidden Tests (Download & Run)
    let req_6 = new_by_hidden_test_grader()
        .url("https://www.dropbox.com/s/47jd1jru1f1i0cc/ABCTest.java?raw=1")
        .test_class_name("ABCTest")
        .out_of(5.0)
        .req_name("6")
        .run()
        .await?;

    show_results([req_1, req_2, req_3, req_4, req_5, req_6])?;
    Ok(())
}
```

### Sample grading script (Python)

This script demonstrates grading a Python project, including setting up `uv` execution environments, structural analysis, and output testing.

```rust
use umm::python::{
    new_project,
    new_query_grader,
    new_diff_grader,
    show_results,
    grade_all
};

pub async fn main() {
    let project = new_project()?;

    // 1. Structural Check: Ensure list comprehension is used
    let structure = new_query_grader()
        .project(project.clone())
        .file("assignment")
        .req_name("structure")
        .out_of(10.0)
        .uses_list_comprehension()
        .reason("Must use a list comprehension for efficiency.")
        .run()
        .await?;

    // 2. Behavior Check: Input/Output
    let io = new_diff_grader()
        .project(project.clone())
        .file("assignment")
        .req_name("behavior")
        .out_of(10.0)
        .expect("Result: 42\n")
        .expect_with_input("Result: 84\n", "double 42")
        .run()
        .await?;

    show_results([structure, io])?;
    Ok(())
}
```

### Output

`umm` generates rich, colorful output in the terminal. Below is an example of what the console output might look like (including documentation nits, mutation test failure logs, and the final grade summary).

```text
┌────────────────────────────────────────────────────────────┬
│        Check javadoc for pyramid_scheme.LinkedTree         │
├──────────────────────────┬──────┬──────────────────────────┤
│           File           │ Line │         Message          │
├──────────────────────────┼──────┼──────────────────────────┤
│ ./src/pyramid_scheme/Lin │  14  │   no main description    │
│       kedTree.java       │      │                          │
├──────────────────────────┼──────┼──────────────────────────┤
│ ./src/pyramid_scheme/Lin │  15  │ no description for @para │
│       kedTree.java       │      │            m             │
├──────────────────────────┼──────┼──────────────────────────┤
│                     -2.0 due to 2 nits                     │
└────────────────────────────────────────────────────────────┴

Running Mutation tests -
11:37:54 PM PIT >> INFO : Sending 1 test classes to minion
11:37:54 PM PIT >> SEVERE : Description [testClass=pyramid_scheme.LinkedTreeTest, name=[test:testSize]] did not pass without mutation.
Exception in thread "main" org.pitest.help.PitHelpError: 5 tests did not pass without mutation...

┌─────────────┬────────────┬──────────────────────────┐
│ Requirement │   Grade    │          Reason          │
├─────────────┼────────────┼──────────────────────────┤
│      1      │ 3.00/5.00  │        See above.        │
├─────────────┼────────────┼──────────────────────────┤
│      2      │ 5.00/5.00  │   Got expected output    │
├─────────────┼────────────┼──────────────────────────┤
│      3      │ 5.00/5.00  │   2/2 tests passed       │
├─────────────┼────────────┼──────────────────────────┤
│      4      │ 5.00/5.00  │  Matches found for query │
├─────────────┼────────────┼──────────────────────────┤
│      5      │ 0.00/5.00  │ Something went wrong whi │
│             │            │ le running mutation test │
│             │            │       s, skipping.       │
├─────────────┼────────────┼──────────────────────────┤
│      6      │ 5.00/5.00  │    All tests passed      │
├─────────────┼────────────┼──────────────────────────┤
│                 Total: 23.00/30.00                  │
└─────────────────────────────────────────────────────┴
```

## Configuration

- `OPENAI_ENDPOINT`: Base API URL (e.g., `https://api.openai.com/v1`). Required for SLO feedback.
- `OPENAI_API_KEY_SLO`: API key used for SLO feedback requests. Required for SLO feedback.
- `OPENAI_MODEL`: Model name for SLO feedback (e.g., `gpt-4.1`). Required for SLO feedback.
- `OPENAI_TEMPERATURE`: Optional float. If set and valid, included in Chat Completions requests; otherwise omitted.
- `OPENAI_TOP_P`: Optional float. If set and valid, included in Chat Completions requests; otherwise omitted.
- `OPENAI_REASONING_EFFORT`: Optional string, one of `low`, `medium`, `high`. Defaults to `medium` when not set.
- `SUPABASE_URL`: Supabase project URL (base, e.g., `https://<project>.supabase.co`). Usage is optional, required only if you want to upload feedback.
- `SUPABASE_ANON_KEY`: Supabase anon key. Usage is optional, required only if you want to upload feedback.

**Notes**:
- `OPENAI_TEMPERATURE` and `OPENAI_TOP_P` are only sent if provided; there is no default implicit value passed.
- `OPENAI_REASONING_EFFORT` always applies a value; when not set, it defaults to `medium`.
- `SUPABASE_URL` is converted to a Postgrest endpoint by appending `/rest/v1`.

**Setup**:
- Create a `.env` file and fill in values.
- Existing OS environment variables take precedence over `.env`; unset values are read from `.env` if present.

## License

This project is licensed under the MIT License.

## Scripting API Reference

The scripting API is exposed in Rune via the `umm` module.

### Java Grading (`umm::java`)

#### Project Management

*   `new_project() -> Result<Project>`
*   `new_project_from_paths(paths: ProjectPaths) -> Result<Project>`
*   `new_project_paths() -> ProjectPathsBuilder`

**`ProjectPathsBuilder`**:
*   `.root_dir(path: String)`
*   `.source_dir(path: String)`
*   `.build_dir(path: String)`
*   `.test_dir(path: String)`
*   `.lib_dir(path: String)`
*   `.umm_dir(path: String)`
*   `.report_dir(path: String)`
*   `.build() -> Result<ProjectPaths>`

---

#### 1. Docs Grader

Checks for the presence of Javadoc on specified files.

*   `new_docs_grader() -> DocsGraderBuilder`

**Builder Methods**:
*   `.project(project: Project)` (**Required**)
*   `.files(files: Vec<String>)`: Source files to check (e.g. `["List", "Node"]`).
*   `.req_name(name: String)` (**Required**)
*   `.out_of(score: f64)` (**Required**)
*   `.penalty(deduction: f64)`: Points deducted per missing doc.
*   `.run() -> Result<GradeResult>`

**Usage**:
```rust
let docs = new_docs_grader()
    .project(project.clone())
    .files(["ArrayList"])
    .req_name("javadoc")
    .out_of(5.0)
    .penalty(1.0)
    .run()
    .await?;
```

**Sample Output**:
```text
┌────────────────────────────────────────────────────────────┬
│                    Check javadoc for ArrayList             │
├──────────────────────────┬──────┬──────────────────────────┤
│           File           │ Line │         Message          │
├──────────────────────────┼──────┼──────────────────────────┤
│ .../src/ArrayList.java   │  14  │   no main description    │
├──────────────────────────┼──────┼──────────────────────────┤
│                     -2 due to 2 nits                       │
└────────────────────────────────────────────────────────────┴
```

---

#### 2. Diff Grader

Compiles and runs a file, comparing standard output to expected strings.

*   `new_diff_grader() -> DiffGraderBuilder`

**Builder Methods**:
*   `.project(project: Project)` (**Required**)
*   `.file(main_class: String)` (**Required**)
*   `.req_name(name: String)` (**Required**)
*   `.out_of(score: f64)` (**Required**)
*   `.cases(cases: Vec<(String, Option<String>)>)`: List of `(expected_output, optional_input)`.
*   `.ignore_case(ignore: bool)`
*   `.preserve_whitespace(preserve: bool)`
*   `.run() -> Result<GradeResult>`

**Usage**:
```rust
let diff = new_diff_grader()
    .project(project.clone())
    .file("Main")
    .req_name("behavior")
    .out_of(10.0)
    .cases([
        ("Hello World\n", None),           // Expect "Hello World"
        ("Hello Alice\n", Some("Alice")),  // Input: "Alice"
    ])
    .run()
    .await?;
```

**Sample Output**:
```text
┌─────────────┬─────────────┬─────────────────────┐
│ Requirement │    Grade    │ Reason              │
├─────────────┼─────────────┼─────────────────────┤
│ behavior    │ 10.00/10.00 │ Got expected output │
└─────────────┴─────────────┴─────────────────────┘
```

---

#### 3. Unit Test Grader (Visible)

Runs existing JUnit tests found in the project.

*   `new_by_unit_test_grader() -> ByUnitTestGraderBuilder`

**Builder Methods**:
*   `.project(project: Project)` (**Required**)
*   `.test_files(files: Vec<String>)`: Test classes to execute.
*   `.expected_tests(tests: Vec<String>)`: Specific test methods required (e.g., `["Test#method"]`).
*   `.req_name(name: String)` (**Required**)
*   `.out_of(score: f64)` (**Required**)
*   `.run() -> Result<GradeResult>`

**Usage**:
```rust
let tests = new_by_unit_test_grader()
    .project(project.clone())
    .test_files(["MainTest"])
    .expected_tests(["MainTest#testAdd", "MainTest#testRemove"])
    .req_name("tests")
    .out_of(20.0)
    .run()
    .await?;
```

**Sample Output**:
```text
┌─────────────┬─────────────┬──────────────────────┐
│ Requirement │    Grade    │ Reason               │
├─────────────┼─────────────┼──────────────────────┤
│ tests       │ 20.00/20.00 │ 2/2 tests passed     │
└─────────────┴─────────────┴──────────────────────┘
```

---

#### 4. Mutation Grader

Runs mutation testing (PIT) to evaluate legacy verification quality.

*   `new_unit_test_grader() -> UnitTestGraderBuilder`

**Builder Methods**:
*   `.project(project: Project)` (**Required**)
*   `.target_test(tests: Vec<String>)`: Tests to run against mutants.
*   `.target_class(classes: Vec<String>)`: Classes to mutate.
*   `.excluded_methods(methods: Vec<String>)`
*   `.avoid_calls_to(classes: Vec<String>)`
*   `.req_name(name: String)` (**Required**)
*   `.out_of(score: f64)` (**Required**)
*   `.run() -> Result<GradeResult>`

**Usage**:
```rust
let mutation = new_unit_test_grader()
    .project(project.clone())
    .target_test(["pyramid_scheme.LinkedTreeTest"])
    .target_class(["pyramid_scheme.LinkedTree"])
    .req_name("mutation")
    .out_of(10.0)
    .run()
    .await?;
```

**Sample Output**:
```text
Running Mutation tests -
...
PIT >> SEVERE : Description [testClass=LinkedTreeTest, name=testSize] did not pass without mutation.
Exception in thread "main": 5 tests did not pass without mutation...
┌─────────────┬───────────┬────────────────────────────────────────┐
│ Requirement │   Grade   │                 Reason                 │
├─────────────┼───────────┼────────────────────────────────────────┤
│ mutation    │ 0.00/10.00│ Something went wrong while running...  │
└─────────────┴───────────┴────────────────────────────────────────┘
```

---

#### 5. Hidden Test Grader

Downloads a test file from a URL and runs it against the student's code.

*   `new_by_hidden_test_grader() -> ByHiddenTestGraderBuilder`

**Builder Methods**:
*   `.url(url: String)` (**Required**): URL to download the test file.
*   `.test_class_name(name: String)` (**Required**): Name of the test class.
*   `.req_name(name: String)` (**Required**)
*   `.out_of(score: f64)` (**Required**)
*   `.run() -> Result<GradeResult>`

**Usage**:
```rust
let hidden = new_by_hidden_test_grader()
    .url("https://example.com/HiddenTest.java")
    .test_class_name("HiddenTest")
    .req_name("hidden")
    .out_of(10.0)
    .run()
    .await?;
```

**Sample Output**:
```text
┌─────────────┬─────────────┬─────────────────────┐
│ Requirement │    Grade    │ Reason              │
├─────────────┼─────────────┼─────────────────────┤
│ hidden      │ 10.00/10.00 │ All tests passed    │
└─────────────┴─────────────┴─────────────────────┘
```

---

#### 6. Query Grader

Checks for structural requirements using Tree-sitter queries.

*   `new_query_grader() -> QueryGraderBuilder`

**Builder Methods**:
*   `.project(project: Project)` (**Required**)
*   `.file(filename: String)` (**Required**)
*   `.req_name(name: String)` (**Required**)
*   `.out_of(score: f64)` (**Required**)
*   `.queries(queries: Vec<String>)`: Raw Tree-sitter queries.
*   `.queries_with_capture(queries: Vec<(String, String)>)`: Queries with explicit capture names.
*   `.constraint(constraint: QueryConstraint)`
*   `.reason(message: String)`: Failure message.
*   `.run() -> Result<GradeResult>`

**`QueryConstraint`**:
*   `QueryConstraint::must_match_at_least_once()`
*   `QueryConstraint::must_match_exactly_n(n: usize)`
*   `QueryConstraint::must_not_match()`

**Usage**:
```rust
let structure = new_query_grader()
    .project(project.clone())
    .file("Main")
    .req_name("structure")
    .out_of(5.0)
    .queries_with_capture([("((for_statement) @loop)", "loop")])
    .reason("Must contain a for loop")
    .run()
    .await?;
```

**Sample Output**:
```text
┌─────────────┬───────────┬─────────────────────┐
│ Requirement │ Grade     │ Reason              │
├─────────────┼───────────┼─────────────────────┤
│ structure   │ 5.00/5.00 │ Must contain loop   │
└─────────────┴───────────┴─────────────────────┘
```

---

### Python Grading (`umm::python`)

#### Project Management

*   `new_project() -> Result<Project>`
*   `new_project_from_paths(paths: ProjectPaths) -> Result<Project>`
*   `new_project_from_paths_with_context(paths: ProjectPaths, ctx: RunContext) -> Result<Project>`
*   `new_project_paths() -> ProjectPathsBuilder`
*   `new_run_context() -> RunContextBuilder`

**`ProjectPathsBuilder`**:
*   `.root_dir(path: String)`
*   `.source_dir(path: String)`
*   `.test_dir(path: String)`
*   `.venv_dir(path: String)`
*   `.data_dir(path: String)`
*   `.umm_dir(path: String)`
*   `.report_dir(path: String)`
*   `.build() -> Result<ProjectPaths>`

**`RunContextBuilder`**:
*   `.root_dir(path: String)`
*   `.working_dir(path: String)`
*   `.pythonpath(paths: Vec<String>)`: Set `PYTHONPATH` explicitly.
*   `.env_path(path: String)`: Path to `.venv`.
*   `.overlay(dep: String)`: Add ephemeral dependency (e.g. `"pytest"`).
*   `.overlays(deps: Vec<String>)`
*   `.locked(is_locked: bool)`: Utilize `uv run --locked`.
*   `.no_config(no: bool)`: Skip `uv` config loading.
*   `.no_env_file(no: bool)`: Skip `.env` loading.
*   `.build() -> Result<RunContext>`

---

#### 1. Diff Grader

Runs a Python script and checks output.

*   `new_diff_grader() -> DiffGraderBuilder`

**Builder Methods**:
*   `.project(project: Project)` (**Required**)
*   `.file(script: String)` (**Required**)
*   `.req_name(name: String)` (**Required**)
*   `.out_of(score: f64)` (**Required**)
*   `.expect(output: String)`: Add a simple test case.
*   `.expect_with_input(output: String, input: String)`: Add a test case with stdin.
*   `.cases(cases: Vec<(String, Option<String>)>)`: Bulk add cases.
*   `.ignore_case(ignore: bool)`
*   `.preserve_whitespace(preserve: bool)`
*   `.run() -> Result<GradeResult>`

**Usage**:
```rust
let io = new_diff_grader()
    .project(project.clone())
    .file("main")
    .req_name("io-check")
    .out_of(10.0)
    .expect("Expected Output\n")
    .expect_with_input("Expected with input\n", "input")
    .run()
    .await?;
```

**Sample Output**:
```text
┌─────────────┬─────────────┬─────────────────────┐
│ Requirement │    Grade    │ Reason              │
├─────────────┼─────────────┼─────────────────────┤
│ io-check    │ 10.00/10.00 │ Got expected output │
└─────────────┴─────────────┴─────────────────────┘
```

---

#### 2. Query Grader

Structural analysis for Python.

*   `new_query_grader() -> QueryGraderBuilder`

**Builder Methods**:
*   `.project(project: Project)` (**Required**)
*   `.file(filename: String)` (**Required**)
*   `.req_name(name: String)` (**Required**)
*   `.out_of(score: f64)` (**Required**)
*   `.queries(queries: Vec<String>)`
*   `.queries_with_capture(queries: Vec<(String, String)>)`
*   `.constraint(constraint: QueryConstraint)`
*   `.reason(message: String)`
*   `.run() -> Result<GradeResult>`

**Convenience Methods**:
*   `.function_with_name(name: String)`
*   `.class_with_name(name: String)`
*   `.imports_module(name: String)`
*   `.imports_from(module: String)`
*   `.defines_function(name: String)`
*   `.uses_list_comprehension()`
*   `.uses_dict_comprehension()`
*   `.uses_set_comprehension()`
*   `.uses_generator_expression()`
*   `.uses_for_loop()`
*   `.uses_while_loop()`
*   `.uses_if_statement()`
*   `.uses_try_except()`
*   `.uses_with_statement()`

*   `.uses_lambda()`
*   `.uses_decorator()`
*   `.uses_yield()`
*   `.uses_assert()`
*   `.uses_raise()`

**Usage**:
```rust
let check = new_query_grader()
    .project(project.clone())
    .file("assignment")
    .req_name("python-idioms")
    .out_of(5.0)
    .uses_list_comprehension() // Must use [x for x in y]
    .uses_with_statement()     // Must use 'with open(...) ...'
    .run()
    .await?;
```

**Sample Output**:
```text
┌───────────────┬───────────┬──────────────────────────────┐
│ Requirement   │ Grade     │ Reason                       │
├───────────────┼───────────┼──────────────────────────────┤
│ python-idioms │ 5.00/5.00 │ Matches found for queries... │
└───────────────┴───────────┴──────────────────────────────┘
```

---

#### 3. Test Grader (Pytest)

Runs `pytest` suite.

*   `new_test_grader() -> TestGraderBuilder`

**Builder Methods**:
*   `.project(project: Project)` (**Required**)
*   `.test_files(files: Vec<String>)`: Tests to run (e.g. `["tests/test_x.py"]`).
*   `.req_name(name: String)`
*   `.out_of(score: f64)`
*   `.run() -> Result<GradeResult>`

**Usage**:
```rust
let pytest = new_test_grader()
    .project(project.clone())
    .test_files(["tests/test_assignment.py"])
    .req_name("pytest")
    .out_of(10.0)
    .run()
    .await?;
```

**Sample Output**:
```text
┌─────────────┬─────────────┬──────────────────────────┐
│ Requirement │    Grade    │ Reason                   │
├─────────────┼─────────────┼──────────────────────────┤
│ pytest      │ 10.00/10.00 │ 10/10 tests passed       │
└─────────────┴─────────────┴──────────────────────────┘
```

---

#### 4. Docs Grader

Checks for docstrings in Python files.

*   `new_docs_grader() -> DocsGraderBuilder`

**Builder Methods**:
*   `.project(project: Project)` (**Required**)
*   `.files(files: Vec<String>)`: Files to check for docstrings.
*   `.req_name(name: String)`
*   `.out_of(score: f64)`
*   `.penalty(deduction: f64)`
*   `.run() -> Result<GradeResult>`

**Usage**:
```rust
let docs = new_docs_grader()
    .project(project.clone())
    .files(["main.py", "utils.py"])
    .req_name("docs")
    .out_of(5.0)
    .run()
    .await?;
```

---

#### 5. Code Review Grader (LLM)

Uses LLM to provide code review feedback.

*   `new_code_review_grader() -> CodeReviewGraderBuilder`

**Builder Methods**:
*   `.project(project: Project)` (**Required**)
*   `.files(files: Vec<String>)`: Files to review.
*   `.instructions_path(path: String)`: Assistant instructions.
*   `.weekly_context_path(path: String)`: Context file.
*   `.req_name(name: String)`
*   `.out_of(score: f64)`
*   `.execute_files(exec: bool)`: Whether to run code during review.
*   `.run() -> Result<GradeResult>`

**Usage**:
```rust
let review = new_code_review_grader()
    .project(project.clone())
    .files(["main.py"])
    .instructions_path("instructions.md")
    .req_name("review")
    .out_of(10.0)
    .run()
    .await?;
```

---

### Common Utilities (`umm::gradescope`)

Shared configuration and output tools.

#### Structs & Enums

*   **`GradescopeOutputFormat`**:
    *   `GradescopeOutputFormat::text()`
    *   `GradescopeOutputFormat::html()`
    *   `GradescopeOutputFormat::simple_format()`
    *   `GradescopeOutputFormat::md()`
    *   `GradescopeOutputFormat::ansi()`
*   **`GradescopeVisibility`**:
    *   `GradescopeVisibility::hidden()`
    *   `GradescopeVisibility::after_due_date()`
    *   `GradescopeVisibility::after_published()`
    *   `GradescopeVisibility::visible()`

#### Functions

*   `show_results(results: Vec<GradeResult>) -> Result<()>`: Display results using default config.
*   `show_results_with_config(results: Vec<GradeResult>, config: GradescopeConfig) -> Result<()>`: Display using custom config.

#### `GradescopeConfigBuilder`

Created via `GradescopeConfig::builder()`.

*   `.source_files(files: Vec<String>)`
*   `.test_files(files: Vec<String>)`
*   `.project_title(title: String)`
*   `.project_description(desc: String)`
*   `.pass_threshold(score: f64)`
*   `.show_table(show: bool)`
*   `.results_json(emit: bool)`: Toggle `results.json` output.
*   `.feedback(emit: bool)`: Toggle Supabase feedback.
*   `.debug(emit: bool)`: Write `results.json` locally for debugging.
*   `.enabled_slos(slos: Vec<String>)`: Whitelist specific SLOs.
*   `.build() -> GradescopeConfig`
