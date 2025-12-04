# Bon Migration & Builder Ergonomics Plan

## Why
- Reduce handwritten setter boilerplate and runtime “missing field” errors by leaning on bon’s typestate builders (compile-time required/optional checks, `maybe_` setters, partial application).
- Replace the remaining `typed-builder` usage with bon for consistency and smaller deps.
- Align the public grader API with the documented fluent style (sample Rune script) while keeping serde/output shapes stable.
- Keep configuration surfaces (paths, projects) flexible with builder defaults instead of TODOs.

## What (checkboxes = planned execution steps)

### 1) Replace typed-builder DTOs with bon
- [x] `src/java/grade/diagnostics.rs`: `JavacDiagnostic`, `MutationDiagnostic`
  - Use `#[derive(Builder)]`, `#[builder(on(String, into))]`; no serde shape changes.
- [x] `src/java/grade/gradescope.rs`: `GradescopeSubmission`, `GradescopeTestCase`, `GradescopeLeaderboardEntry`
  - `#[builder(on(String, into))]`, `#[builder(default)]` for all options.
  - Consider `on(Vec<_>, with = FromIterator::from_iter)` for iterable-friendly setters.
- [x] Remove `typed-builder` dependency after the swap (Cargo + lock).

### 2) Recast graders as bon builders (keep current `.run()` names via `finish_fn`)
- [x] Diff (`src/java/grade/diff.rs`): bon builder with `.case(...)` helper and builder-level `.run()`.
- [x] Docs (`src/java/grade/docs.rs`): builder with `#[builder(default = 3.0)] penalty`, required `project`, `files`, `req_name`, `out_of`, builder `.run()`.
- [x] ByUnitTest (`src/java/grade/tests.rs`): builder; required `project`, `test_files`, `req_name`, `out_of`; optional `expected_tests`; builder `.run()`.
- [x] UnitTest (mutation) (`tests.rs`): builder; required `req_name`, `out_of`, `target_test`, `target_class`; optional ignore lists; builder `.run()`.
- [x] ByHiddenTest (`tests.rs`): builder; required `url`, `test_class_name`, `req_name`, `out_of`; builder `.run()`.
- [x] QueryGrader (`src/java/grade/query.rs`): builder with `.run()`.
- [x] Grade/GradeResult (`src/java/grade/results.rs`): small builder to replace ad-hoc setters; keep tabled/serde behavior.
- [x] For each builder: applied `on(String, into)` and `default`/`required` as appropriate; `.run()` helpers on builder types instead of `finish_fn`.
- [x] Decide per-field `overwritable` (feature already enabled) only where re-setting is useful (e.g., `cases`, `test_files`).
  - Decision: keep overwritable **off** everywhere; builders remain single-set to preserve typestate guarantees and avoid silent overwrites.

### 3) Builders for workspace plumbing
- [x] `ProjectPaths` (`src/java/paths.rs`): builder-style `project_paths(...)` with optional overrides; defaults computed from `root_dir`.
- [x] `Project` (`src/java/project.rs`): builder-style `project(...)` accepting optional `ProjectPaths`.

### 4) Update construction call sites & docs
- [x] `src/java/parsers.rs` usages of diagnostics/mutation builders (bon builders keep the same surface).
- [x] Gradescope assembly sites in `gradescope.rs` that call `.builder()` (bon builders keep the same surface).
- [x] Example script `examples/sample.rn` to match the new builder API.
- [x] Any README/context snippets referencing the old fluent graders.
  - README now points Rune users to the bon builder surface; context.md to be refreshed separately.

### 5) Bon attribute polish & helper usage
- [x] Use iterable-friendly setters on list fields (custom `with` closures accepting `IntoIterator<Item = impl Into<String>>` for tests, files, mutation lists; `with = FromIterator::from_iter` for Gradescope cases/tags).
- [x] Consider `with` closures for small conversions (e.g., stripping `.mutators.` in `MutationDiagnostic`, building canned queries in `QueryGrader`).
  - Implemented mutator normalization in the builder; QueryGrader left as-is to avoid altering public semantics.
- [x] Keep `state_mod` default (no cross-module typestate exposure) unless we add custom builder methods that need visibility.
- [x] `builder(getter)` on grader builders where it aids inspection/debugging; enabled across graders/DTOs to simplify debugging and tests.
- [x] Bonus macros: adopted `bon::vec!` where it removes `.to_string()` noise (project description, feedback assembly).

### 6) Quality gates
- [x] `cargo fmt`
- [x] `cargo clippy --all-targets`
- [x] Smoke checks: added `tests/bon_builders_smoke.rs` to exercise builder ergonomics (diff/doc/gradescope), plus existing parser suite via `cargo test -- --list`.
- [x] Update `context.md` (untracked) after structural/API changes.

### 7) Considered, likely no-change
- [x] Retrieval/config/process primitives (`ConfigState`, `HeuristicConfig`, `OpenAiEnv`, `process::Collected`): simple structs already constructed once; bon adds little and would churn unrelated code.
- [x] Tree-sitter `Parser`, `File`, `Project` core logic: keep hand-written APIs to avoid compile-cost bloat and preserve current semantics; only add builders where noted above (`ProjectPaths`, `Project` entry).

### 8) Second-pass hygiene and docs signals
- [x] Enforce `#![warn(missing_docs)]` and `#![warn(clippy::missing_docs_in_private_items)]` at the top of every `src/` file to keep API coverage honest while we refactor.
- [x] Re-run `cargo fmt`, `cargo clippy --all-targets`, and `cargo test -- --list` after the lint-attribute sweep to ensure no regressions snuck in.
- [x] Reconfirm sample Rune script and README snippets align with the bon builders (single-step `.run()` finish functions, iterable setters).

### 9) Review-note fixes (kept API/docs promises intact)
- [x] Restore `serde`/`tabled` derives on `Grade` / `GradeResult` so JSON exporters and tables continue to work.
- [x] Clone `req_name` in `DocsGrader::grade_docs` and related paths to avoid moving out of `&self` (fixes the compilation error noted in review).
- [x] Add `DiffGrader::case(...)` helper plus `DiffCase` re-export; switch `cases` to a multi-arg `with` closure so callers don’t need two setters or manual structs.
- [x] Keep builder ergonomics consistent: update all `GradeResult` constructions to use the bon builder with `maybe_prompt`, and align `examples/sample.rn` with infallible `.run()` surfaces.

## Notes & Tradeoffs
- Preserve public/serde shapes; avoid breaking Rune sample behavior.
- `experimental-overwritable` is enabled; apply narrowly to avoid weakening guarantees.
- `implied-bounds` is on; useful if custom builder methods over generics appear (e.g., Query extensions), otherwise harmless.
- Redundant hand-written getters/setters removed from graders/DTOs; bon-generated builders + field access are now the single API surface.

## Decisions on earlier questions
- Compatibility shims: **we won’t carry legacy setter names**; callers will use the bon-generated surfaces exclusively. This keeps the API clean and reduces maintenance.
- `builder(getter)`: **default to enabling it on the new grader/builders** so tests and debugging can peek at already-set fields. If a specific builder’s public surface would get noisy, we can opt that one out explicitly.
- DiffGrader cases: **keep `DiffCase` internal but expose ergonomic setters**—a multi-arg `cases` `with`-closure that accepts tuples and a `.case(...)` convenience for one-at-a-time addition. Both feed the same vector and still finish with `.run()`, matching the documented API.
