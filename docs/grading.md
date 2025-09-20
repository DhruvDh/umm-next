# Grading System Overview

This document explains how grading works in this repository, how built‑in graders are structured and invoked, how feedback is generated, how to enable SLO feedback and Gradescope output, and how to add a new grader. It is aimed at contributors writing graders in Rust and instructors writing Rhai grading scripts.

## Overview

- Core components live in `src/`:
  - `src/java.rs`: Java project discovery, compile/run/test, doclint, and tree‑sitter parsing.
  - `src/grade.rs`: All grader types, `Grade`/`GradeResult`, feedback generation, Gradescope output, SLO integration, query helpers.
  - `src/lib.rs`: Builds the Rhai engine, registers all custom types and helpers.
  - `src/main.rs`: CLI; `umm grade <NAME|PATH>` loads a Rhai grading script (local or from Supabase) and runs it.
- Rhai scripts in `grading/` show how to compose graders and produce results.

At a high level, a Rhai script builds one or more graders with a builder‑style API, calls `.run()`, aggregates `GradeResult`s, and calls `show_results([...], config)` and optionally `generate_feedback([...])`.

## Types and Results

- `Grade { grade: f64, out_of: f64 }` — represents the earned score and the maximum.
- `GradeResult { requirement, grade, reason, prompt }` — a single requirement’s outcome:
  - `requirement: String` — requirement ID or name.
  - `grade: Grade` — the `grade/out_of` pair.
  - `reason: String` — human‑readable summary (penalties, test counts, etc.).
  - `prompt: Option<Vec<ChatCompletionRequestMessage>>` — messages later sent to the feedback service; built by graders as needed.

`GradeResult` is displayed in a table by `show_results`, serialized to Gradescope JSON if enabled, and can be sent to the feedback service via `generate_feedback`/`generate_single_feedback`.

## Built‑in Graders

All graders are defined in `src/grade.rs` and exported to Rhai using `CustomType` plus the `#[generate_rhai_variant(Fallible)]` macro to generate script‑friendly wrappers.

### 1) DocsGrader — `grade_docs`

Checks for Javadoc issues using `javac -Xdoclint` on specified class names.

- Inputs: `project`, `files` (`Array<String>` of class names), `out_of`, `req_name`, `penalty` (default 3.0 per diagnostic).
- Behavior:
  - Runs doclint for each file; parses compiler output into diagnostics.
  - Grade = `max(out_of - penalty * num_diags, 0)`.
  - On compilation failure: returns `0/out_of` and attaches a `prompt` with system message, compiler stacktrace, and source context using `get_source_context(...)`.
- Rhai usage:
  ```rhai
  let req = new_docs_grader()
      .project(project)
      .files(["pkg.ClassName"]) // class name
      .out_of(10.0)
      .req_name("1")
      .penalty(3.0)
      .run();
  ```

### 2) ByUnitTestGrader — `grade_by_tests`

Runs JUnit tests and grades proportionally by pass rate.

- Inputs: `project`, `test_files` (`Array<String>` of test class names), optional `expected_tests` (`Array<String>` of fully‑qualified `Class#method`), `out_of`, `req_name`.
- Behavior:
  - Optionally validates discovered test methods vs `expected_tests`, adding reasons if mismatched; if reasons exist, tests are not run and grade is `0/out_of` with a `prompt`.
  - Else runs each test class via the project API; parses `passed/total` and sums across files; grade = `(total_passed/total) * out_of`.
  - On failed tests/compile/runtime errors: attaches a `prompt` with (a) system message, (b) cleaned stacktrace/user message, (c) source context via `get_source_context(...)`.
  - Active retrieval: if enabled via `use_active_retrieval()`, the grader can replace heuristics with context chosen by a small external tool call; see “Context Extraction”.
- Rhai usage:
  ```rhai
  let req = new_by_unit_test_grader()
      .project(project)
      .test_files(["pkg.TestClass"])
      .expected_tests(["pkg.TestClass#testFoo"]) // optional
      .out_of(20.0)
      .req_name("2")
      .run();
  ```

### 3) UnitTestGrader (Mutation Testing) — `grade_unit_tests`

Runs PiTest to evaluate the quality of student‑written tests.

- Inputs: `req_name`, `out_of`, `target_test` (test classes), `target_class` (mutated classes), `excluded_methods`, `avoid_calls_to`.
- Behavior:
  - Invokes PiTest; reads `test_reports/mutations.csv`.
  - Each `SURVIVED` mutation = 4 points penalty.
  - On failure output: prints a summary and attaches `prompt` with system message, mutation results, and code context; otherwise returns full score.
- Notes:
  - PiTest expects all tests to pass on the baseline run; if baseline fails, the prompt explains that mutation testing requires a green suite first.

### 4) ByHiddenTestGrader — `grade_by_hidden_tests`

Downloads a hidden test file, runs `ByUnitTestGrader`, then cleans up.

- Inputs: `url` (source of hidden test class), `test_class_name`, `out_of`, `req_name`.
- Behavior: Fetches the test file, writes `./<test_class_name>.java`, constructs a `Project`, delegates to `ByUnitTestGrader`, removes the temp file, returns the result.

### 5) DiffGrader — `grade_by_diff`

Runs a class with input(s) and diffs stdout versus expected output(s).

- Inputs: `project`, `file` (class to run), `expected` (Array<String>), `input` (Array<String>), `ignore_case` (bool), `req_name`, `out_of`.
- Behavior:
  - Validates equal lengths of `expected` and `input` arrays; runs the class for each input; normalizes expected/actual if `ignore_case`.
  - For any mismatch, produces a colorized inline diff and accumulates mismatches into a `prompt` with the source file contents.
  - If no mismatches, awards full score.

### 6) QueryGrader — `grade_by_query`

Evaluates source structure using tree‑sitter queries.

- Compose queries using helpers: `method_body_with_name`, `main_method`, `class_body_with_name`, `local_variables*`, `if_statements`, `for_loops`, `while_loops`, `method_invocations*`, or ad‑hoc `.query(q).capture(c).filter(fn)`.
- Constraint modes: `must_match_at_least_once` (default), `must_match_exactly_n_times(n)`, `must_not_match`.
- Provide a meaningful `.reason("...")` to explain the requirement to students.
- Behavior:
  - Executes the first query against the selected file, then successively refines matches by running subsequent queries over matched code slices.
  - On syntax/query errors: returns `0/out_of` with a `prompt` explaining the issue (helps students when grading fails due to malformed code).
  - Depending on the constraint and results, awards `out_of` or `0` and may attach a `prompt`.
- Rhai usage:
  ```rhai
  let req = new_query_grader()
      .project(project)
      .file("pkg.ClassName")
      .req_name("Structure: main present")
      .out_of(5.0)
      .main_method()
      .must_match_at_least_once()
      .reason("Your submission must implement a main method.")
      .run();
  ```

## Context Extraction and Prompts

Graders build `prompt` messages to enable helpful automated feedback:

- `get_source_context<T: Into<LineRef>>(line_refs, project, start_offset, num_lines, max_line_refs, try_use_active_retrieval, active_retrieval_context)`
  - Collects relevant snippets around diagnostic lines, numbers them, and also extracts related method bodies across files when referenced.
  - If `try_use_active_retrieval` is true, it first attempts “active retrieval”.
- Active Retrieval (`get_active_retrieval_context`):
  - Packs prior auto‑grader output and a synopsis into a small prompt, calls an external selection function, and uses the returned `(class, method)` pairs to embed those exact method bodies as context.
  - Toggle at script time with `use_active_retrieval()` or `use_heuristic_retrieval()` (both available in Rhai).

All long strings shared in prompts are truncated to `PROMPT_TRUNCATE` characters to control payload size.

## Feedback Generation

Two paths exist:

1) Per‑requirement links via Supabase (default):
   - `generate_single_feedback(result: &GradeResult)`
     - If `grade < out_of`, inserts into the Supabase `prompts` table:
       - `messages`: the `prompt` built during grading
       - `requirement_name`, `reason`, `grade` (e.g. “8.00/10.00”), `status: not_started`
     - Returns a URL string: `https://feedback.dhruvdh.com/<id>`.
     - If no penalty, returns a generic message.
   - `generate_feedback(results: Array)`
     - Writes a `FEEDBACK` file: a heading + per‑requirement lines returned by `generate_single_feedback`.

2) Inline in Gradescope JSON (optional):
   - When `show_results` runs with `results_json: true` and `feedback: true`, each Gradescope test case gets the feedback URL text inline in the `output` field.

Required env for Supabase:
- `SUPABASE_URL`, `SUPABASE_ANON_KEY` (see `.env.example`).

## SLO Feedback (Optional)

When enabled and the student passes the assignment (by a threshold), the tool can produce additional “SLO feedback” as a no‑score Gradescope test case.

- Enable by passing flags to `show_results` via `gradescope_config`:
  - `slo_algorithmic_solutions`, `slo_code_readability`, `slo_comments`, `slo_error_handling`, `slo_logic`, `slo_naming_conventions`, `slo_oop_programming`, `slo_syntax`, `slo_testing` (booleans, default false).
  - Also supply `source_files`, `test_files`, `project_title`, `project_description`, and optionally `pass_threshold` (default 0.7).
- The code composes curated system prompts per SLO (see `src/prompts/slos/*`) and includes only the relevant file code to keep costs low.
- Each SLO request uses Chat Completions with env:
  - `OPENAI_ENDPOINT`, `OPENAI_API_KEY_SLO`, `OPENAI_MODEL` (required for SLO feedback),
  - Optional: `OPENAI_TEMPERATURE`, `OPENAI_TOP_P`, `OPENAI_REASONING_EFFORT`.
- Output is aggregated by `generate_combined_slo_report` into a single Gradescope test case.

## Gradescope Output (Optional)

`show_results(results, gradescope_config)` can emit `/autograder/results/results.json` compatible with Gradescope.

Important `gradescope_config` keys:
- `show_table: bool` — show the terminal table (default true).
- `results_json: bool` — write JSON (default false).
- `feedback: bool` — include feedback text per test case (default false).
- `debug: bool` — write to `./results.json` instead of `/autograder/...` (default false).
- `pass_threshold: f64` — pass cutoff (default 0.7); used for pass/fail status and SLO gating.
- `source_files`, `test_files`, `project_title`, `project_description`, `slo_*` — see SLO section above.

Example:
```rhai
show_results(reqs, #{
  results_json: true,
  feedback: true,
  debug: true,
  pass_threshold: 0.7,
  project_title: "Shopping List",
  project_description: "Implement ArrayList‑backed shopping list.",
  source_files: ["Shopping.ShoppingListArrayList"],
  test_files: ["Shopping.ShoppingListArrayListTest"],
  slo_code_readability: true,
  slo_testing: true,
});
```

## Writing Rhai Grading Scripts

Patterns to follow:
- Always start by constructing the project: `let project = new_java_project();`
- Compose graders via builder pattern and finish with `.run()`.
- Aggregate results and call `show_results`.
- Optionally call `generate_feedback([...])` for the subset of requirements where human‑readable feedback is helpful and permissible.

Examples:
- `grading/sample.rhai` — multiple graders (docs, unit tests, hidden tests, mutation testing) and thresholding.
- `grading/projects/shoppinglist/shoppinglist.rhai` — applied example with conditional feedback generation.

Minimal example:
```rhai
let project = new_java_project();
let r1 = new_by_unit_test_grader()
  .project(project)
  .test_files(["pkg.FooTest"]) 
  .out_of(20.0)
  .req_name("Unit Tests")
  .run();
show_results([r1]);
```

## Adding a New Grader (Rust)

Use this checklist to integrate a new grader into Rhai and the CLI.

1. Define a struct in `src/grade.rs`:
   - Prefer `#[derive(Clone, Default)]`.
   - Fields typically include: `req_name: String`, `out_of: f64`, `project: Project`, and any config.
2. Implement the grading method:
   - Return `anyhow::Result<GradeResult>`.
   - On success: set `requirement`, `grade: Grade::new(score, out_of)`, and a brief `reason`.
   - On penalties or failures: attach a `prompt` with context using `get_source_context(...)`; keep strings under `PROMPT_TRUNCATE`.
3. Add `#[generate_rhai_variant(Fallible)]` to the grading method:
   - This generates `*_script` wrappers that return `Result<_, Box<EvalAltResult>>` for use in Rhai.
4. Implement `CustomType` for the struct in `src/grade.rs`:
   - Register builder setters/getters: `.with_fn("field", Self::field)` and `.with_fn("field", Self::set_field)`.
   - Provide `.with_fn("new_your_grader", Self::default)` and `.with_fn("run", Self::your_method_script)`.
5. Register the type in `create_engine()` (`src/lib.rs`):
   - Add `.build_type::<YourGrader>()` to the engine pipeline.
6. Add example usage to a Rhai script under `grading/` (optional but recommended).

Template snippet:
```rust
#[derive(Clone, Default)]
pub struct MyGrader {
    pub req_name: String,
    pub out_of: f64,
    pub project: Project,
    // more fields...
}

impl MyGrader {
    pub fn req_name(&mut self) -> String { self.req_name.clone() }
    pub fn set_req_name(mut self, v: String) -> Self { self.req_name = v; self }
    pub fn out_of(&mut self) -> f64 { self.out_of }
    pub fn set_out_of(mut self, v: f64) -> Self { self.out_of = v; self }
    pub fn project(&mut self) -> Project { self.project.clone() }
    pub fn set_project(mut self, p: Project) -> Self { self.project = p; self }

    #[generate_rhai_variant(Fallible)]
    pub fn grade_my_way(&mut self) -> Result<GradeResult> {
        // ... do work ...
        Ok(GradeResult {
            requirement: self.req_name.clone(),
            grade: Grade::new(self.out_of /* or computed */, self.out_of),
            reason: "...".into(),
            prompt: None, // or Some(vec![..messages..])
        })
    }
}

impl CustomType for MyGrader {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("MyGrader")
            .with_fn("req_name", Self::req_name)
            .with_fn("req_name", Self::set_req_name)
            .with_fn("out_of", Self::out_of)
            .with_fn("out_of", Self::set_out_of)
            .with_fn("project", Self::project)
            .with_fn("project", Self::set_project)
            .with_fn("new_my_grader", Self::default)
            .with_fn("run", Self::grade_my_way_script);
    }
}
```

## Environment and Configuration

See `.env.example` for full list. Key variables:

- Supabase (required for feedback URL flow):
  - `SUPABASE_URL`, `SUPABASE_ANON_KEY`
- SLO Feedback (optional):
  - `OPENAI_ENDPOINT`, `OPENAI_API_KEY_SLO`, `OPENAI_MODEL`
  - Optional: `OPENAI_TEMPERATURE`, `OPENAI_TOP_P`, `OPENAI_REASONING_EFFORT`
- Misc:
  - `PROMPT_TRUNCATE = 15000` (constant in code)

If `SUPABASE_URL`/`SUPABASE_ANON_KEY` are missing, the app exits early with an error message.

## Troubleshooting and Gotchas

- Long outputs are truncated in prompts — keep reasons concise.
- When `expected_tests` is specified, ByUnitTestGrader will not run tests if there are mismatches; fix “not found” and “unexpected” test names first.
- Mutation testing requires a green test suite; PiTest will fail fast and the prompt explains why.
- Hidden tests download a Java file into the project root temporarily; failures still clean up the temp file.
- QueryGrader: always provide a `.reason(...)` so students see a meaningful note if constraints fail.
- Windows path escaping in stacktraces is normalized for readability in prompts.

## Quick Reference: Rhai API (selected)

- Project: `new_java_project()`, `.identify(name)`, file `.test([...])`, `.doc_check()`, file `.query(query)`.
- Graders:
  - `new_docs_grader() ... .run()`
  - `new_by_unit_test_grader() ... .run()`
  - `new_unit_test_grader() ... .run()`
  - `new_by_hidden_test_grader() ... .run()`
  - `new_diff_grader() ... .run()`
  - `new_query_grader() ... .run()`
- Output/Feedback: `show_results(results, config)`, `generate_feedback(results)`
- Retrieval mode: `use_active_retrieval()`, `use_heuristic_retrieval()`

## End‑to‑End Examples

See:
- `grading/sample.rhai` — comprehensive example with multiple graders.
- `grading/projects/shoppinglist/shoppinglist.rhai` — applied example with conditional feedback generation.

