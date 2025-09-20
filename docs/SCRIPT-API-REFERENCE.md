# Script API Reference (Rune) — with Rhai → Rune Mapping

Status: Draft (target API with MVP notes)
Script API Version: 0.1.0

Scope

- This is the canonical reference for the script-visible API of `umm-next` (Rune).
- It also maps the legacy Rhai surface (from `umm`) to the new modules so porting is auditable.
- Stability tags:
  - Stable: implemented in MVP and intended to remain.
  - Preview: targeted for near-term implementation; names are stable.
  - Experimental: subject to change.

Contents

- Module map (Rune)
- Prelude (re-exports)
- Module reference (functions, types, examples)
- Rhai → Rune mapping table
- Feature flags & notes

---

## Module Map (Rune)

- `prelude` (Preview) — convenience re-exports for common scripting
- `umm` (Stable) — host utilities: printing, simple reporting, helpers
- `java` (Stable/Preview) — Java language backend: project, parser, query, graders
- `python` (Stable/Preview) — Python language backend (future parity): project, parser?, query, graders

Types commonly used across modules live in `types.rs` and are exposed to Rune as `Any`.

---

## Prelude (Preview)

Re-exports for concise scripts.

Exports (target):

- `project(root: &str) -> java::Project` (alias to `java::discover`)
- `grade_docs`, `grade_by_tests` (wrappers over builders)
- `show_results(results: Vec<GradeResult>, cfg?: Map) -> Summary` (see umm)
- `Grade`, `GradeResult`, `Summary`

Example (target):

```rune
use prelude::*;

pub fn main() {
  let p = project(".");
  let r1 = grade_docs(p, ["pkg.LinkedTree"], 10.0, "1", 3.0)?;
  let r2 = grade_by_tests(p, ["pkg.LinkedTreeTest"], ["pkg.LinkedTreeTest#testSize"], 20.0, "2")?;
  let sum = show_results([r1, r2], #{ pass_threshold: 0.7 });
  if sum.passed { umm::print("PASS") } else { umm::print("FAIL") }
}
```

---

## umm Module

Stable (MVP):

- `show_test_report(rep: TestReport)` — compact table of cases and pass counts.
- `filter_cases(rep: TestReport, predicate: fn(passed: bool) -> bool) -> TestReport` — closure-friendly filtering.
- Printing helpers for demos: `print`, `print_title`, `print_kv`, `print_kv_num`, `print_project`, `print_doc_report`.

Preview (target):

- `show_results(results: Vec<GradeResult>, cfg?: Map) -> Summary { total, out_of, passed }` — standardized grade table and optional Gradescope JSON output.
- `generate_single_feedback(result: GradeResult) -> String` (feature: feedback/db)
- `generate_feedback(results: Vec<GradeResult>) -> ()` (feature: feedback/db)
- `use_active_retrieval()`, `use_heuristic_retrieval()` (toggle retrieval strategy for prompt context)

Example (MVP):

```rune
pub fn main() {
  let p = python::discover(".");
  let tr = python::test(p, "tests", []);
  let kept = umm::filter_cases(tr, fn keep(passed) { passed });
  umm::show_test_report(kept);
}
```

---

## java Module

Stable (MVP):

- `discover(root: String) -> Project`
- `run(project: Project, entry: String, input: Option<String>) -> RunOutput`
- `test(project: Project, target: String, names: Vec<String>) -> TestReport`
- `doc_check(project: Project, target: String) -> DocReport`

Preview (target):

- Types visible to scripts:
  - `Project` — discovered project model (root, language)
  - `File`, `FileType` — file metadata and kind (Interface | Class | ClassWithMain | Test)
  - `Parser` — script-exposed parser (legacy parity), with:
    - `new_parser(source_code: String) -> Parser`
    - `code(&mut self) -> String`
    - `set_code(&mut self, code: String) -> ()`
    - `query(&mut self, q: &str) -> Array<HashMap<String,String>>`

### java::query (Preview)

- `Query::new()` builder with:
  - `set_source(&str) -> Self`
  - `query(&str) -> Self`
  - `capture(&str) -> Self`
  - Constraints: `must_match_at_least_once()`, `must_match_exactly(n)`, `must_not_match()`
  - `filter(f: Function) -> Self` (script closure)
  - `run_query() -> Result<Vec<String>, String>`

Example:

```rune
pub fn main() {
  let src = "class A { void m(){} }\nclass B { void n(){} }";
  let q = java::query::Query::new().set_source(src).query("method_declaration").must_match_at_least_once();
  umm::print_lines_result(q.run_query());
}
```

### java::graders (Preview)

- `DocsGrader`
  - `DocsGrader::new()` / `java::graders::docs_grader()`
  - `set_out_of(f64) -> Self`, `set_req_name(&str) -> Self`, getters
  - `run(project: Project, files: Vec<String>) -> TestReport`
  - `run_checked(project: Project, files: Vec<String>) -> Result<TestReport, GraderError>`
- `ByUnitTestGrader`, `UnitTestGrader` (feature: mutation), `ByHiddenTestGrader` (feature: net), `DiffGrader`, `QueryGrader` — consistent builder surfaces; return `Result<GradeResult, GraderError>`

Example:

```rune
pub fn main() {
  let p = java::discover(".");
  let g = java::graders::docs_grader().set_out_of(10.0).set_req_name("Docs");
  let rep = g.run(p, ["pkg.LinkedTree"]);
  umm::show_test_report(rep);
}
```

Example (MVP):

```rune
pub fn main() {
  let p = java::discover(".");
  let rep = java::test(p, "ExampleTests", []);
  umm::show_test_report(rep);
}
```

Example (target — parser):

```rune
pub fn main() {
  let mut jp = java::Parser::new_parser("class A { void m() {} }");
  let code = jp.code();
  let matches = jp.query("(method_declaration name: (identifier) @name)");
  umm::print_kv("code_len", code.len().to_string());
  umm::print_lines(matches.iter().map(|m| m.get("name").unwrap()).collect());
}
```

---

## python Module (Stable/Preview)

- `discover(root: String) -> Project`
- `run(project: Project, entry: String, input: Option<String>) -> RunOutput`
- `test(project: Project, target: String, names: Vec<String>) -> TestReport`
- `doc_check(project: Project, target: String) -> DocReport`

Preview (target):

- `python::graders` and `python::query` mirroring the Java shapes, using pytest/unittest and Python-appropriate queries.

---

---

---

## Types (Common)

Stable (MVP):

- `Project { root: String, language: String }`
- `RunOutput { status: i32, stdout: String, stderr: String }`
- `TestCase { name: String, passed: bool, message?: String }`
- `TestReport { found: u32, passed: u32, cases: Vec<TestCase> }`
- `DocIssue { file: String, line?: u32, message: String }`
- `DocReport { issues: Vec<DocIssue>, score: f32 }`

Preview (target):

- `Grade { grade: f64, out_of: f64 }`
- `GradeResult { requirement: String, grade: Grade, reason: String }`
- `Summary { total: f64, out_of: f64, passed: bool }`

---

## Rhai → Rune Mapping (Legacy → Target)

Types

- `JavaFileType` → `java::FileType` (Preview)
- `Project (JavaProject)` → `java::Project` (Stable)
- `File (JavaFile)` → `java::File` (Preview)
- `Parser (JavaParser)` → `java::Parser` (Preview; legacy parity kept)
- `Query` → `java::query::Query` (Stable/Preview)
- `QueryGrader` → `java::graders::QueryGrader` (Preview)
- `DocsGrader` → `java::graders::DocsGrader` (Stable)
- `ByUnitTestGrader` → `java::graders::ByUnitTestGrader` (Preview)
- `UnitTestGrader` → `java::graders::UnitTestGrader` (Preview; feature: mutation)
- `ByHiddenTestGrader` → `java::graders::ByHiddenTestGrader` (Preview; feature: net)
- `DiffGrader` → `java::graders::DiffGrader` (Preview)
- `Grade`, `GradeResult` → same names (Preview)

Functions

- `clean()` → `umm::clean()` (Preview; may be CLI-only)
- `show_results(..)` → `umm::show_results(..)` (Preview)
- `generate_single_feedback(..)` → `umm::generate_single_feedback(..)` (Preview; feature: feedback/db)
- `generate_feedback(..)` → `umm::generate_feedback(..)` (Preview; feature: feedback/db)
- `use_active_retrieval()` / `use_heuristic_retrieval()` → `umm::use_active_retrieval()` / `umm::use_heuristic_retrieval()` (Preview)

Notes

- Where a Rune name is Preview, the intent and naming are stable but implementation may land in phases.
- Legacy scripts relying on `JavaParser` should move to `java::Parser` (kept for parity) or the higher-level `query::Query` where practical.

---

## Feature Flags & Notes (Target)

- `feedback/db` — exposes feedback functions; requires `SUPABASE_URL`, `SUPABASE_ANON_KEY`.
- `gradescope` — enables JSON writer in `show_results`.
- `treesitter` — AST-backed query match results.
- `mutation` — PiTest-based mutation testing grader.
- `net` — networked features (hidden tests download, possibly remote script lookup).

Environment (only required when feature is used)

- Feedback: `SUPABASE_URL`, `SUPABASE_ANON_KEY`
- SLO (if kept): `OPENAI_ENDPOINT`, `OPENAI_API_KEY_SLO`, `OPENAI_MODEL`

---

## Acceptance

- Every symbol labeled Stable exists in the code and has at least one runnable example under `umm-next/scripts/`.
- Preview symbols have names frozen here and will gain examples/tests as they land.
