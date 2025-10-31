# Repository Context And Migration Notes

> **Status — Updated 2025-09-28**
>
> - **CLI `grade`:** Disabled on **main**. The embedded Python prototype exists **only** on the `try-python-scripting` branch and remains under evaluation.
> - **Scripting:** **Decision pending.** Python bindings (embedded runtime) are under trial on `try-python-scripting`. Rune design notes are retained **as deferred reference** until a decision is made.
> - **Rhai:** Entry flow removed; residual types compile but are inert without the Rhai entrypoint.
> - **Module layout:** Java sources live under `src/java/*`; configuration and prompts live in `src/config.rs`; `src/constants.rs` is intentionally empty.
> - **Active retrieval toggle:** Managed with an `std::sync::atomic::AtomicBool` stored in `config::ConfigState`.

## Table of Contents

- [Repository Context And Migration Notes](#repository-context-and-migration-notes)
  - [Table of Contents](#table-of-contents)
  - [Onboarding Quickstart (main)](#onboarding-quickstart-main)
  - [CLI Contract (main)](#cli-contract-main)
  - [Environment Variables (main)](#environment-variables-main)
  - [High-Level Overview](#high-level-overview)
  - [Agent Operating Rules](#agent-operating-rules)
  - [Glossary](#glossary)
  - [Major Changes Completed](#major-changes-completed)
  - [Module Map (Authoritative)](#module-map-authoritative)
  - [Important Code References](#important-code-references)
  - [CLI Behavior (Post-Refactor)](#cli-behavior-post-refactor)
  - [Paths \& Configuration Model](#paths--configuration-model)
  - [Java Analysis Pipeline (At a Glance)](#java-analysis-pipeline-at-a-glance)
  - [Scripting Strategy (Decision Record)](#scripting-strategy-decision-record)
    - [Option snapshots](#option-snapshots)
    - [Deferred — Rune Integration Sketch](#deferred--rune-integration-sketch)
  - [Prompts, Env, and Global Config](#prompts-env-and-global-config)
  - [Design Rationale \& Invariants](#design-rationale--invariants)
  - [Definition of Done (main)](#definition-of-done-main)
  - [Cleanup Checklist](#cleanup-checklist)
  - [Plan (Keep Updated)](#plan-keep-updated)
    - [Slice-Copy Workflow (Repeatable)](#slice-copy-workflow-repeatable)
  - [Known Gaps / Open Items](#known-gaps--open-items)
  - [How To Continue (Concrete Next Steps)](#how-to-continue-concrete-next-steps)
  - [Gotchas](#gotchas)
  - [Quick Test Checklist](#quick-test-checklist)
  - [Contact Points (Authoritative)](#contact-points-authoritative)
  - [Doc Maintenance Commands](#doc-maintenance-commands)
  - [Appendices (Dev-only)](#appendices-dev-only)
    - [Appendix A — Python Prototype Notes (Branch Only)](#appendix-a--python-prototype-notes-branch-only)
      - [Branch \& Goal](#branch--goal)
      - [When to use which workflow](#when-to-use-which-workflow)
      - [Progress Snapshot (2025-09-27)](#progress-snapshot-2025-09-27)
      - [Python Surface Inventory](#python-surface-inventory)
      - [Open Questions \& Risks](#open-questions--risks)
      - [Build \& Linking Notes (macOS arm64)](#build--linking-notes-macos-arm64)
      - [Validation Tips](#validation-tips)
      - [Coverage Gaps](#coverage-gaps)
      - [Decision Reminder](#decision-reminder)
  - [Doc Change Log](#doc-change-log)

## Onboarding Quickstart (main)

1. **Clone and hydrate fixtures**
   - `git clone <repo-url>`
   - `git submodule update --init --recursive`
2. **Build and lint**
   - `cargo check`
   - `cargo fmt && cargo clippy --all-targets`
3. **Run core commands** *(requires JDK + JUnit jars on your classpath)*
   - `umm java run <ClassWithMain>`
   - `umm java test <TestClass> [tests...]`
   - `umm java doc-check <Class>`
   - `umm grade <...>` → expect the “temporarily unavailable” message on `main`.
4. **Read the code in this order**
   - Paths: `src/java/paths.rs`
   - Config/runtime: `src/config.rs`
   - Graders: `src/java/grade/*`
   - CLI wiring: `src/main.rs`

## CLI Contract (main)

| Command                     | Inputs                            | Side-effects                                | Exit conditions                                |
|-----------------------------|-----------------------------------|----------------------------------------------|------------------------------------------------|
| `umm java run <ClassWithMain>`   | Java class with `main`            | Compiles and runs via `Project::run`          | `0` on success; non-zero on compile/run failure |
| `umm java check <Class>`         | Java class name                   | Compiles and prints diagnostics               | `0` on success; non-zero on compiler errors     |
| `umm java test <TestClass> …`    | Test class, optional test names   | Runs JUnit on existing classpath              | `0` on pass; non-zero on failing tests          |
| `umm java doc-check <Class>`     | Java class name                   | Runs `javac -Xdoclint` for documentation lint | `0` on clean; non-zero on warnings/errors       |
| `umm grade <...>` (main)    | Any args                          | None; prints disabled message                 | Always prints “grade is temporarily unavailable”|

## Environment Variables (main)

| Variable                  | Purpose                                  | Default / behavior                                        | Consumed in               |
|---------------------------|------------------------------------------|-----------------------------------------------------------|---------------------------|
| `SUPABASE_URL`            | Supabase PostgREST endpoint               | Optional; only required when publishing feedback          | `src/config.rs` (`SupabaseEnv`)
| `SUPABASE_ANON_KEY`       | Supabase PostgREST anon key               | Optional; only required when publishing feedback          | `src/config.rs` (`SupabaseEnv`)
| `OPENAI_API_KEY` (+ deps) | OpenAI feedback / SLO requests            | Optional; loaders error only when those features are used | `src/config.rs` (`OpenAiEnv`)
| `UMM_RETRIEVAL_ENDPOINT`  | Active retrieval backend URL              | Defaults to historical Deno endpoint                      | `src/config.rs`
| `UMM_COURSE`              | Course identifier surfaced in metadata    | Defaults to `ITSC 2214`                                   | `src/config.rs`
| `UMM_TERM`                | Academic term surfaced in metadata        | Defaults to `Fall 2022`                                   | `src/config.rs`
| `UMM_JAVAC_TIMEOUT_SECS`  | Max seconds allowed for `javac` invocations | Defaults to `30`; larger values risk long hangs           | `src/config.rs`
| `UMM_JAVA_TIMEOUT_SECS`   | Max seconds allowed for `java` / JUnit runs | Defaults to `60`; raise for long-running submissions      | `src/config.rs`
| `JAVA_HOME` / `PATH`      | Locate `javac` / `java` for CLI commands  | Must resolve to a working JDK in the environment          | External toolchain        |

## High-Level Overview

- Goal: Remove Rhai-era coupling and land on a stable scripting surface with per-project configuration driven by `ProjectPaths`.
- The tool grades Java submissions: it compiles, runs, and tests code, then uses LLM-powered feedback.
- Key crates and libraries (main): `tokio`, `reqwest`, `tree-sitter` (Java grammar), `bpaf`, `serde`, `axum`, `postgrest`.
  - *Trial (branch-only):* embedded Python runtime via PyO3/PyOxidizer — see Appendix.

## Agent Operating Rules

- Treat this document as the single source of architecture truth; update it whenever logic or roadmap shifts.
- Update the **Module Map**, **Cleanup Checklist**, and **Plan** sections when you change structure or priorities.
- Wait for user sign-off on proposed edits before touching repository files.
- Keep `context.md` untracked in version control; regenerate the sections you alter for handoff.
- Use the review mindset: focus on ergonomics for “data bag” modules and on separation of concerns for graders and parsers.

## Glossary

- **Rhai** — previous scripting engine; entry points removed; residual types compile but are inert without the Rhai runtime.
- **Rune** — deferred scripting design retained for future reference; not active on `main`.
- **Embedded Python** — trial feature on `try-python-scripting` bundling a Python runtime inside the CLI to run `umm grade <script.py>`.
- **ProjectPaths** — instance-scoped path model providing `root/src/build/test/lib/.umm` accessors; eliminates global path state.
- **SLO** — service level objective prompts/payloads used when emitting higher fidelity feedback.
- **PITest** — Java mutation testing integration that writes `test_reports/mutations.csv` when enabled.

## Major Changes Completed

- Removed the `umm_derive` proc-macro crate and eliminated generated `_script` wrappers across the codebase.
- Disabled the Rhai script entry path; `lib::grade` returns a clear “temporarily unavailable” error on main.
- Deleted VSCode tasks/settings generators; manual cleanup remains available via
  `Project::clean_paths`, but the CLI `clean` command has been removed.
- Introduced `ProjectPaths` and routed all compile/run/test logic through instance-scoped paths; removed legacy globals from `src/constants.rs`.
- Added `src/config.rs` to centralize runtime bootstrapping, prompt loading, Supabase access (cached via `state::InitCell<Postgrest>`), shared HTTP client, and the active-retrieval toggle (`AtomicBool`).
- Removed the bundled JAR download workflow; `Project` no longer fetches artifacts and instead respects whatever is on the classpath.
- Switched Supabase and OpenAI usage to lazy initialization so commands that do not need them run without credentials.
- Improved grader feedback rendering: penalties print inline and degrade gracefully when external services are unavailable.
- Hardened file/runtime ergonomics post-Rhai removal, including accurate `FileType` classification and safer snippet rendering helpers.

## Module Map (Authoritative)

- **Scope note:** The Module Map lists **main** only. Branch-only files (e.g., the Python prototype) are documented in the Appendix.

- `src/java/mod.rs` — Module root re-exporting the Java subsystems (`file`, `parser`, `paths`, `project`, `grade`).
- `src/java/config.rs` — Java-specific configuration bundle (prompts, retrieval defaults, timeouts).
- `src/java/paths.rs` — `ProjectPaths` definition plus accessors for `root`, `src`, `build`, `test`, `lib`, `.umm`, and separators.
- `src/java/parser.rs` — Tree-sitter Java parser wrapper; owns `Parser`, per-file `Tree` caching, and helpers for executing SCM query patterns.
- `src/java/file.rs` — `FileType`, `File`, `JavaFileError`, and compile/run/test/doc-check orchestration atop `ProjectPaths`.
- `src/java/project.rs` — Project discovery and submission description utilities; coordinates `ProjectPaths` usage (JAR downloads removed).
- `src/java/grade/`
  - `context.rs` — Retrieval/context builder (heuristic windows, snippet formatting).
  - `diff.rs` — Diff-based grader wiring against `GradeResult`.
  - `docs.rs` — Doclint-driven grader.
  - `feedback.rs` — FEEDBACK file + Supabase/OpenAI persistence helpers.
  - `gradescope.rs` — Gradescope payload generation and SLO prompts.
  - `query.rs` — Tree-sitter query graders; filter predicates now use native Rust closures and Vec outputs.
  - `results.rs` — `GradeResult`, `Grade`, `LineRef`, and supporting types.
  - `tests.rs` — Unit/hidden test graders and PIT hooks.
  - `diagnostics.rs` — Diagnostic structs shared across graders.
  - `mod.rs` — Grader module exports.
- `src/java/queries/` — String resources / helpers for Tree-sitter SCM patterns.
- `src/java/util.rs` — Java-specific toolchain and path helpers (`classpath`, `sourcepath`).
- `src/util.rs` — Shared helpers (`umm_path`, `find_files`).
- `src/lib.rs` — Crate root exporting configuration, Java helpers, process utilities, retrieval, scripting runtime, and shared types.
- `src/main.rs` — CLI wiring via `bpaf`; dispatches to library commands and executes Rune grading scripts.
- `src/config.rs` — Runtime/env bootstrap: loads prompts, Supabase metadata, HTTP client, retrieval endpoint, and active-retrieval flag.
- `src/retrieval.rs` — Retrieval modes (`Full`, `Heuristic`, `Active`) and the `RetrievalFormatter` trait implemented by language modules.
- `src/scripting/mod.rs` — Rune VM bootstrapper (`Context`, `Unit`, `Vm`) plus the entrypoint used by `umm java grade`.
- `src/scripting/java.rs` — `umm::java` Rune module: builders for Docs/Unit/Diff graders, `Project` helpers, and `show_results`.
- `fixtures/java/` — Java fixtures for integration tests; initialize submodules with `git submodule update --init --recursive`.
- `examples/sample.rn` — Reference Rune script demonstrating Docs/Unit/Diff graders and `show_results`.

## Important Code References

- Paths & project layout: `src/java/paths.rs`, `src/java/project.rs`, `src/java/mod.rs`
- Classpath/sourcepath helpers: `src/java/util.rs`
- CLI wiring: `src/main.rs`
- Graders & feedback: `src/java/grade/*`
- Retrieval heuristics: `src/retrieval.rs`, `src/java/grade/context.rs`
- Env/services/prompts/runtime: `src/config.rs`

## CLI Behavior (Post-Refactor)

- `java run <ClassWithMain>` / `java check <Class>` / `java test <TestClass> [tests...]` / `java doc-check <Class>`:
    - Operate via `Project::new()` and instance-scoped `ProjectPaths`.
    - Assume required JUnit jars are already on the classpath (no downloads).
- `grade` (main):
    - Executes a Rune script via `scripting::run_file`, expecting an async `pub fn main() -> Result<(), String>`.
    - Surface script or grader failures with contextual error messages.

## Paths & Configuration Model

- `ProjectPaths` instances supply all path derivations; avoid global statics.
- Helpers like `classpath(&ProjectPaths)` and `sourcepath(&ProjectPaths)` live in `src/java/util.rs`.
- Configuration lives in `src/config.rs`, which wraps the shared runtime, HTTP client, prompt catalog, and Supabase metadata; it lazily caches the PostgREST client via a `state::InitCell<Postgrest>` and tracks active retrieval with an `AtomicBool`.

## Java Analysis Pipeline (At a Glance)

1. `Project::new()` discovers Java files and attaches `ProjectPaths`.
2. `File` objects parse sources through `Parser::new(code)`.
3. Graders query parsed trees using patterns from `src/java/queries.rs`.
4. Grader modules emit `GradeResult` values consumed by CLI feedback.

## Scripting Strategy (Decision Record)

- **Decision**: Rune is the primary scripting surface on `main`. The CLI executes `.rn` files directly through `scripting::run_file`.
- **Implementation**:
  - `Context::with_default_modules()` boots the Rune standard library; `umm::java` registers project helpers and grader builders.
  - Scripts expose an async `pub fn main() -> Result<(), String>` and compose `DocsGrader`, `ByUnitTestGrader`, and `DiffGrader` builders.
  - Results are rendered with `show_results`, matching the CLI table output.
- **Legacy**: The embedded-Python prototype remains on `try-python-scripting` for reference but is no longer authoritative.
- **Next steps**:
  - Expose additional graders (hidden tests, mutation) and feedback generators.
  - Offer query-graders with Rune predicates once filters migrate off Rhai-specific APIs.

## Prompts, Env, and Global Config

- Core handles live in `src/config.rs`; they are wrapped inside `ConfigState` and cached via `Arc<ConfigState>`.
- Shared APIs (selected):
  - `config::runtime() -> Arc<Runtime>` — shared Tokio runtime.
  - `config::http_client() -> reqwest::Client` — shared HTTP client (proxy disabled for sandbox compatibility).
  - `config::java_config() -> JavaConfigRef` — read-only access to the Java config bundle (prompts, timeouts).
  - `config::java_prompts() -> JavaPromptsRef` — convenience wrapper for the Java prompt catalog.
  - `config::postgrest_client() -> Option<Postgrest>` — lazily caches the Supabase PostgREST client using `state::InitCell<Postgrest>`.
  - `config::retrieval_endpoint() -> String` — returns the configured active-retrieval endpoint.
  - `config::heuristic_defaults()` / `set_heuristic_defaults(...)` — read/update snippet heuristics.
  - `config::set_active_retrieval(enabled: bool)` / `config::active_retrieval_enabled() -> bool` — atomically manage the active-retrieval flag (`AtomicBool`).
- Retrieval helpers reuse the shared `reqwest::Client` and pull `UMM_RETRIEVAL_ENDPOINT` from environment, defaulting to the historical Deno service.
- Snippet heuristics live in `HeuristicConfig`; builders update them via the config setters.

## Design Rationale & Invariants

- Paths must remain instance-scoped; avoid reintroducing globals.
- `grade` stays disabled until the scripting story is ready; any interim work must keep the failure mode obvious.
- Grader snippet formatting should flow through `render_snippet` in `src/java/grade/context.rs`.

## Definition of Done (main)

- [ ] `cargo fmt && cargo clippy --all-targets` run cleanly.
- [ ] `umm java run/test/doc-check` succeed against a sample project with JDK + JUnit jars available.
- [ ] Status banner is updated (date + scripting decision) when behavior changes.
- [ ] Module Map lists only files present on `main` (no branch-only paths).
- [ ] Active-retrieval behavior documented as `AtomicBool` with helpers (`set_active_retrieval`, `active_retrieval_enabled`).
- [ ] No references to pre-split paths (`src/java.rs`, `src/grade.rs`).
- [ ] README/docs updated when user-visible behavior shifts.

## Cleanup Checklist

> Work Mode: We are operating in a file-by-file cleanup cadence; cross-file work found during passes is captured as backlog and prioritized separately.

- [x] `src/constants.rs` — Shared tree-sitter queries and retrieval toggles pulled into module scope; file now intentionally empty.
- [x] `src/config.rs` — Runtime/env bootstrap with cached PostgREST handle (`state::InitCell<Postgrest>`), shared HTTP client, and `AtomicBool` toggle for active retrieval.
- [x] `src/util.rs` — Path utilities (`classpath` / `sourcepath`) migrated to accept `&ProjectPaths`.
- [x] `src/java/paths.rs` — Instance-scoped path model adopted across the project.
- [x] `src/java/parser.rs` — Tree-sitter wrapper now surfaces errors via `anyhow`, caches capture indices, and exposes a fallible `set_code`.
- [x] `src/java/grade/diagnostics.rs` — Diagnostic structs now capture severity/result enums and expose typed helpers.
- [x] `src/java/grade/results.rs` — Base grading types updated; Rhai-era borrowing removed.
- [x] `src/java/grade/context.rs` — Retrieval/context builder refactored and tested.

Queue — Next File Passes (rolling 3–5)

- [ ] `src/java/grade/query.rs`
  - Goal: remove Rhai types (`AST`, `Array`, `FnPtr`, `SCRIPT_AST`) and model filters as typed predicates/closures; keep public behavior and error messages stable.
  - Non-goals: change query semantics or SCM captures.
  - Acceptance: compiles; `cargo clippy --all-targets` clean; existing query graders behave unchanged on fixtures; no Rhai types in public API.
- [ ] `src/java/grade/tests.rs`
  - Goal: replace `rhai::Array` with `Vec<String>` for `test_files`/`expected_tests` and tighten mismatch reporting; keep retrieval prompts identical.
  - Acceptance: identical outputs on fixtures; clearer missing/unexpected test messages; clippy clean.
- [ ] `src/java/grade/gradescope.rs`
  - Goal: remove `rhai::{Array, Map}`; use typed structs end-to-end; ensure emitted JSON stays identical.
  - Acceptance: JSON shape/values stable on sample results; add minimal doc comments to public structs.
- [ ] `src/java/grade/feedback.rs`
  - Goal: drop `rhai::Array` in `generate_feedback`; operate on typed `Vec<GradeResult>`; keep Supabase flow as-is.
  - Acceptance: FEEDBACK file content unchanged for penalties; clippy clean.
- [ ] `src/parsers.rs`
  - Goal: align PEG grammars with typed severities/results; improve Windows path note; add small tests for edge paths.
  - Acceptance: tests cover missing filename cases and PIT CSV variances.

Follow‑Ups (per-pass discoveries)

- [x] `src/java/file.rs` — decomposed `File::new` into helpers (`parse_source`, `detect_file_identity`, `collect_test_methods`, interface/class section builders) and clarified docs; push_block already skips empty collections so no extra allocation cleanup needed.
- [ ] `src/java/project.rs` — honor path overrides (CLI/env), remove discovery panics, tighten runtime spawning/cache logic.
- [x] `src/java/grade/diff.rs` — Console diffs now use `owo-colors` while prompts stay ANSI-free; further regression coverage deferred.
- [ ] `src/java/grade/docs.rs` — finish guard rails for missing filenames in javac output; ensure prompt truncation/tables remain consistent.
- [ ] `src/java/grade/mod.rs`, `src/java/mod.rs` — audit exports/visibility after grader refactors.
- [ ] `src/lib.rs`, `src/main.rs` — revisit once scripting path lands; re-enable `grade` behind a feature gate.

Recent Cleanups (reference)

- 2025-10-09 — diff grader prompts/typed cases modernized.
- 2025-10-04 — javac diagnostics guarded against missing filenames.
- 2025-09-30 — diagnostic severities and PIT results typed.
- 2025-09-28 — parser error handling tightened; `context.md` status/plan refreshed; `.gitignore` updated to keep `context.md` untracked.

## Plan (Keep Updated)

- [x] Move prompt/env configuration into `src/config.rs` with a cached PostgREST handle and AtomicBool-backed retrieval toggle.
- [x] Split `src/java` into a folder module and re-home path helpers on `ProjectPaths`.
- [x] Split `grade.rs` into `src/java/grade/` submodules using the slice-copy workflow.
- [x] Retire legacy CLI surfaces (`create-submission`, `check-health`, `serve_project_code`).
- [ ] Replace remaining Rhai types (`rhai::Array`, `FnPtr`, `SCRIPT_AST`) with native equivalents.
- [ ] Decide on the long-term scripting path (Rune vs embedded Python trial) and re-enable `grade` behind a feature flag.
- [ ] Harden embedded Python runtime pipeline (document PyOxidizer setup, add smoke test for `umm grade script.py`).
- [ ] Expand `ProjectPaths` customization (alternative roots, multi-module support).
- [ ] Refresh public docs (`README`, `docs/`) once scripting direction is final.

### Slice-Copy Workflow (Repeatable)

1. Move the target source into the destination `mod.rs` temporarily.
2. Use a throwaway script to slice exact text ranges into new files (anchor on struct/impl headers or doc comments).
3. Remove the sliced blocks from `mod.rs`, then add `mod` declarations and `pub use` re-exports.
4. Fix imports/visibility with the narrowest scope (`pub(crate)` when possible) and adjust relative paths for `include_str!` assets.
5. Run `cargo fmt` and `cargo clippy --all-targets` to validate the split.

## Known Gaps / Open Items

- Decide between Rune and Python once the branch evaluation concludes; update the Status banner immediately afterward.
- Improve Python bindings ergonomics if the trial continues (remove Rhai callback shims, finalize error mapping).
- Determine packaging strategy for any embedded runtime (CI, distribution, cross-platform story).
- Expand retrieval configuration exposure (builder-style overrides for heuristics and endpoints).
- Track outstanding PEG/parser refactors needed before Rune work resumes.
- Diff Grader Modernization — Detailed Notes
  - Status: initial clean-up applied — guards use logical `&&`, prompts route through `build_context_message` with user/system roles fixed, plain-text prompt bodies append the offending file’s source, diff failures surface an actionable reason, and stdin is always piped to avoid blocking.
  - Rationale: `src/java/grade/diff.rs` predates the new retrieval/context APIs and still couples UI concerns (ANSI colors) to prompts while depending on Rhai containers. Modernizing it improves correctness, consistency across graders, and unblocks the Rhai removal.
  - Remaining follow-ups:
    - Provide compatibility shims if external scripts still rely on the old `set_expected` / `set_input` Rhai setters.
    - Replace the `colored` crate usage with `owo-colors` (or similar) so terminal colour support is auto-detected without manual env toggles; ensure prompts remain colour-free regardless of the library.
    - Add regression coverage (unit/integration) for the diff grader, especially zero-input cases to catch stdin regressions.
    - Investigate migrating other graders (`tests.rs`, `docs.rs`, etc.) off `rhai::Array`/`FnPtr` once the compatibility layer is settled.
  - Non-goals:
    - Changing diff algorithm/semantics (still Patience + unicode word granularity).
    - Re-enabling the top-level `grade` command or altering CLI surfaces.
  - Acceptance criteria:
    - `cargo check`, `cargo fmt`, and `cargo clippy --all-targets` pass.
    - On mismatches, stderr shows a colorized local diff; the stored prompt contains no ANSI codes and uses correct message roles.
    - On success, returns a full‑credit `GradeResult` with a clear “Got expected output” reason.
    - No change in diff semantics or grading thresholds.

## How To Continue (Concrete Next Steps)

1. Config extension: expose mutation hooks on `config` for prompts/client overrides and document usage.
2. Rhai removal: replace `rhai::Array`/`FnPtr` usage in graders, delete `SCRIPT_AST` once query filtering is redesigned.
3. Scripting prototype: gate the selected scripting surface (Rune or Python) behind a feature and ship a minimal `grade` flow.
4. Project paths: allow CLI/env overrides for alternate roots and multi-module layouts.
5. Documentation: update README/docs to describe the path model and scripting status once the prototype stabilizes.

## Gotchas

- Avoid reintroducing global path state—everything should route through `ProjectPaths`.
- `Project::new()` leverages the process runtime; be mindful of handle cloning before spawning tasks.
- JUnit/PITest expectations rely on the caller’s classpath; ensure mutation runs can write `test_reports/mutations.csv` under the project root.
- Legacy `clean()` is retained only as an erroring stub; use
  `Project::clean_paths` if manual cleanup is required.

## Quick Test Checklist

- `cargo check` — should pass without warnings.
- `cargo fmt` — format before handoff.
- `cargo clippy --all-targets` — minimum lint bar.
- `umm java run <ClassWithMain>` — runs the Java class with discovered paths.
- `umm java test <TestClass> [tests...]` — uses existing classpath; ensure JUnit jars are present locally.
- `umm java doc-check <Class>` — runs `javac -Xdoclint`.
- `umm grade <...>` — returns the disabled-message on main; use the branch prototype only for the Python trial.

## Contact Points (Authoritative)

- Paths & layout: `src/java/paths.rs`, `src/java/project.rs`, `src/java/mod.rs`
- Classpath/sourcepath & IO: `src/util.rs`
- CLI wiring: `src/main.rs`
- Graders & feedback: `src/java/grade/*`
- Env/services/prompts/runtime: `src/config.rs`

## Doc Maintenance Commands

```bash
# Find stale, pre-split file references
rg -n "src/java.rs|src/grade.rs" context.md

# Ensure active-retrieval wording stays accurate
rg -n "Active retrieval|AtomicBool|InitCell|USE_ACTIVE_RETRIEVAL" context.md

# Confirm no lingering jar-download references
rg -n "jar download|JAR download|serve_project_code" context.md

# Detect duplicate layout headings
rg -n "Module Map|Current Module Layout|Project File Map" context.md
```

## Appendices (Dev-only)

### Appendix A — Python Prototype Notes (Branch Only)

*Developer only; none of this ships on `main`.*

#### Branch & Goal

- Branch: `try-python-scripting` (not merged).
- Objective: evaluate an embedded Python runtime so `umm grade <script.py>` can run without an external interpreter.

#### When to use which workflow

- **Embedded runtime (PyOxidizer)** — primary target for shipping a bundled interpreter inside the CLI binary.
- **Extension module (maturin/`python/tests`)** — local development convenience for iterating on PyO3 bindings.

#### Progress Snapshot (2025-09-27)

- Phase 1–3 complete: API inventory, surface design, build scaffolding.
- Phase 4–5 in progress: conversion helpers, error layer, grader bindings (`Diff`, `ByUnitTest`, `Query`).
- Phase 6 (validation/docs) queued pending branch decision.

#### Python Surface Inventory

- Wrappers: `PyProject`, `PyGradeResult`, `PyDocsGrader`, `PyDiffGrader`, `PyByUnitTestGrader`, `PyQueryGrader`.
- Remaining work: expose hidden-test/UnitTest graders, expand query helpers beyond `method_invocations_with_name`, map domain errors precisely, and remove Rhai-backed `FnPtr` filters.

#### Open Questions & Risks

- Packaging/distribution: maturin vs embedded runtime; CI/wheel story unresolved.
- Security & sandboxing for Python graders.
- Cross-platform support once PyOxidizer is part of the build.

#### Build & Linking Notes (macOS arm64)

- `build.rs` regenerates PyOxidizer artifacts when `--features python` is enabled; set `PYOXIDIZER_CMD` if PyOxidizer is not on `PATH`.
- Provide a local `pyo3-config.toml` pointing at a Homebrew Python framework (`/opt/homebrew/opt/python@3.13/...`) and export `PYO3_CONFIG_FILE` before building.
- `cargo build --features python` plus symlinking `libumm.dylib` to `umm.cpython-<ver>-darwin.dylib` lets CPython import the module without maturin.

#### Validation Tips

- Example script: `python/examples/arraylist_graders.py` (requires Java toolchain in PATH).
- Pytests: `PYTHONPATH=target/debug uv run --with pytest python -m pytest python/tests` (fails in restricted sandboxes; run locally when possible).

#### Coverage Gaps

- No wrappers for `ByHiddenTestGrader`/`UnitTestGrader` yet.
- `PyQueryGrader` lacks the broader helper set (loop/conditional/class queries).
- Conversion utilities still rely on `rhai::Array`; refactor graders to concrete Rust collections before merging.

#### Decision Reminder

- Treat these notes as exploratory. Mainline `grade` stays disabled until Rune vs Python decision is made and documented in the Status banner.

---

## Doc Change Log

- 2025-10-16: Relocated Java-only prompt/query assets and parser helpers into
  `src/java/`, moved classpath/sourcepath utilities alongside them, and scoped
  config prompts under a `JavaConfig` bundle (`java_prompts()` accessor retained);
  left TODO breadcrumbs on
  `ProjectPaths` / `Project` to surface configurable workspace layouts when a
  typed builder lands.
- 2025-10-15: Documented the `src/java/file.rs` refactor—`File::new` now delegates to
  helper functions (`parse_source`, `detect_file_identity`, `collect_test_methods`,
  `build_description`, plus interface/class section helpers) to keep construction,
  retrieval summaries, and testing-focused logic separate.
- 2025-09-28: Added onboarding quickstart, CLI contract, env/glossary tables, clarified config/AtomicBool behavior, tightened scripting decision record + appendix, and documented maintenance commands.
