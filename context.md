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
   - `umm run <ClassWithMain>`
   - `umm test <TestClass> [tests...]`
   - `umm doc-check <Class>`
   - `umm grade <...>` → expect the “temporarily unavailable” message on `main`.
4. **Read the code in this order**
   - Paths: `src/java/paths.rs`
   - Config/runtime: `src/config.rs`
   - Graders: `src/java/grade/*`
   - CLI wiring: `src/main.rs`

## CLI Contract (main)

| Command                     | Inputs                            | Side-effects                                | Exit conditions                                |
|-----------------------------|-----------------------------------|----------------------------------------------|------------------------------------------------|
| `umm run <ClassWithMain>`   | Java class with `main`            | Compiles and runs via `Project::run`          | `0` on success; non-zero on compile/run failure |
| `umm check <Class>`         | Java class name                   | Compiles and prints diagnostics               | `0` on success; non-zero on compiler errors     |
| `umm test <TestClass> …`    | Test class, optional test names   | Runs JUnit on existing classpath              | `0` on pass; non-zero on failing tests          |
| `umm doc-check <Class>`     | Java class name                   | Runs `javac -Xdoclint` for documentation lint | `0` on clean; non-zero on warnings/errors       |
| `umm clean`                 | —                                 | Removes build/lib dirs and `.vscode/*`        | `0` on success                                  |
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
- Deleted VSCode tasks/settings generators while keeping `clean()` support for removing leftovers.
- Introduced `ProjectPaths` and routed all compile/run/test logic through instance-scoped paths; removed legacy globals from `src/constants.rs`.
- Added `src/config.rs` to centralize runtime bootstrapping, prompt loading, Supabase access (cached via `state::InitCell<Postgrest>`), shared HTTP client, and the active-retrieval toggle (`AtomicBool`).
- Removed the bundled JAR download workflow; `Project` no longer fetches artifacts and instead respects whatever is on the classpath.
- Switched Supabase and OpenAI usage to lazy initialization so commands that do not need them run without credentials.
- Improved grader feedback rendering: penalties print inline and degrade gracefully when external services are unavailable.
- Hardened file/runtime ergonomics post-Rhai removal, including accurate `FileType` classification and safer snippet rendering helpers.

## Module Map (Authoritative)

- **Scope note:** The Module Map lists **main** only. Branch-only files (e.g., the Python prototype) are documented in the Appendix.

- `src/java/mod.rs` — Module root re-exporting the Java subsystems (`file`, `parser`, `paths`, `project`, `grade`).
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
  - `query.rs` — Tree-sitter query graders; still uses Rhai `FnPtr` filters pending refactor.
  - `results.rs` — `GradeResult`, `Grade`, `LineRef`, and supporting types.
  - `tests.rs` — Unit/hidden test graders and PIT hooks.
  - `diagnostics.rs` — Diagnostic structs shared across graders.
  - `mod.rs` — Grader module exports.
- `src/java/queries.rs` — String resources / helpers for Tree-sitter SCM patterns.
- `src/util.rs` — Shared path and filesystem helpers (`classpath`, `sourcepath`, discovery utilities).
- `src/lib.rs` — Library API exposing `clean()` and the disabled `grade()` entry; surfaces helpers consumed by the CLI.
- `src/main.rs` — CLI wiring via `bpaf`; dispatches to library commands and prints the disabled `grade` message.
- `src/config.rs` — Runtime/env bootstrap: loads prompts, Supabase metadata, HTTP client, retrieval endpoint, and active-retrieval flag.
- `src/retrieval.rs` — Retrieval modes (`Full`, `Heuristic`, `Active`) and the `RetrievalFormatter` trait implemented by language modules.
- `fixtures/java/` — Java fixtures for integration tests; initialize submodules with `git submodule update --init --recursive`.

## Important Code References

- Paths & project layout: `src/java/paths.rs`, `src/java/project.rs`, `src/java/mod.rs`
- Classpath/sourcepath helpers: `src/util.rs`
- CLI wiring: `src/main.rs`
- Graders & feedback: `src/java/grade/*`
- Retrieval heuristics: `src/retrieval.rs`, `src/java/grade/context.rs`
- Env/services/prompts/runtime: `src/config.rs`
- Cleanup helpers: `src/lib.rs` (`clean()`)

## CLI Behavior (Post-Refactor)

- `run <ClassWithMain>` / `check <Class>` / `test <TestClass> [tests...]` / `doc-check <Class>`:
  - Operate via `Project::new()` and instance-scoped `ProjectPaths`.
  - Assume required JUnit jars are already on the classpath (no downloads).
- `clean`:
  - Removes build/lib directories and `.vscode/*` artifacts using `ProjectPaths`.
- `grade` (main):
  - Returns a clear “temporarily unavailable” error by design.
  - See **Scripting Strategy** for the branch-only Python prototype.

## Paths & Configuration Model

- `ProjectPaths` instances supply all path derivations; avoid global statics.
- Helpers like `classpath(&ProjectPaths)` and `sourcepath(&ProjectPaths)` live in `src/util.rs`.
- Configuration lives in `src/config.rs`, which wraps the shared runtime, HTTP client, prompt catalog, and Supabase metadata; it lazily caches the PostgREST client via a `state::InitCell<Postgrest>` and tracks active retrieval with an `AtomicBool`.

## Java Analysis Pipeline (At a Glance)

1. `Project::new()` discovers Java files and attaches `ProjectPaths`.
2. `File` objects parse sources through `Parser::new(code)`.
3. Graders query parsed trees using patterns from `src/java/queries.rs`.
4. Grader modules emit `GradeResult` values consumed by CLI feedback.

## Scripting Strategy (Decision Record)

- **Problem**: Re-enable `umm grade <script.py>` without reviving the Rhai engine.
- **Options considered**:
  1. **Rune runtime** — native scripting via Rune (design retained for reference; deferred).
  2. **Embedded Python runtime** — bundle Python into the Rust binary (trial on `try-python-scripting`).
- **Current state**: Decision pending; `main` keeps `grade` disabled while the Python prototype lives on its branch.
- **Next milestone**: Ship a minimal `grade` flow behind a feature flag with a smoke test, compare against the Rune design, then choose the path.

### Option snapshots

- **Embedded Python (trial)** — see Appendix A for dev-only build/test notes (PyO3 + PyOxidizer + maturin helpers).
- **Rune (deferred)** — retain the integration sketch below to avoid losing the design context.

### Deferred — Rune Integration Sketch

- Rune remains the long-term scripting target; design notes retained for continuity.
- Do not resurrect Rhai helpers; defer implementation until after the Python evaluation concludes.

## Prompts, Env, and Global Config

- Core handles live in `src/config.rs`; they are wrapped inside `ConfigState` and cached via `Arc<ConfigState>`.
- Shared APIs (selected):
  - `config::runtime() -> Arc<Runtime>` — shared Tokio runtime.
  - `config::http_client() -> reqwest::Client` — shared HTTP client (proxy disabled for sandbox compatibility).
  - `config::prompts() -> PromptsRef` — read-only access to the loaded prompt catalog.
  - `config::postgrest_client() -> Option<Postgrest>` — lazily caches the Supabase PostgREST client using `state::InitCell<Postgrest>`.
  - `config::retrieval_endpoint() -> String` — returns the configured active-retrieval endpoint.
  - `config::heuristic_defaults()` / `set_heuristic_defaults(...)` — read/update snippet heuristics.
  - `config::set_active_retrieval(enabled: bool)` / `config::active_retrieval_enabled() -> bool` — atomically manage the active-retrieval flag (`AtomicBool`).
- Retrieval helpers reuse the shared `reqwest::Client` and pull `UMM_RETRIEVAL_ENDPOINT` from environment, defaulting to the historical Deno service.
- Snippet heuristics live in `HeuristicConfig`; builders update them via the config setters.

## Design Rationale & Invariants

- Paths must remain instance-scoped; avoid reintroducing globals.
- CLI cleanup is limited to removing artifacts—no automatic generation of editor configs.
- `grade` stays disabled until the scripting story is ready; any interim work must keep the failure mode obvious.
- Grader snippet formatting should flow through `render_snippet` in `src/java/grade/context.rs`.

## Definition of Done (main)

- [ ] `cargo fmt && cargo clippy --all-targets` run cleanly.
- [ ] `umm run/test/doc-check/clean` succeed against a sample project with JDK + JUnit jars available.
- [ ] Status banner is updated (date + scripting decision) when behavior changes.
- [ ] Module Map lists only files present on `main` (no branch-only paths).
- [ ] Active-retrieval behavior documented as `AtomicBool` with helpers (`set_active_retrieval`, `active_retrieval_enabled`).
- [ ] No references to pre-split paths (`src/java.rs`, `src/grade.rs`).
- [ ] README/docs updated when user-visible behavior shifts.

## Cleanup Checklist

- [x] `src/constants.rs` — Shared tree-sitter queries and retrieval toggles pulled into module scope; file now intentionally empty.
- [x] `src/config.rs` — Runtime/env bootstrap with cached PostgREST handle (`state::InitCell<Postgrest>`), shared HTTP client, and `AtomicBool` toggle for active retrieval.
- [x] `src/util.rs` — Path utilities (`classpath` / `sourcepath`) migrated to accept `&ProjectPaths`.
- [x] `src/java/paths.rs` — Instance-scoped path model adopted across the project.
- [x] `src/java/parser.rs` — Tree-sitter wrapper now surfaces errors via `anyhow`, caches capture indices, and exposes a fallible `set_code`.
- [x] `src/java/grade/diagnostics.rs` — Diagnostic structs now capture severity/result enums and expose typed helpers.
- [ ] `src/parsers.rs` — PEG parsers require alignment with the new diagnostics model.
- [ ] `src/java/file.rs` — Compile/run/test/doc-check flow still carries legacy ergonomics.
- [ ] `src/java/project.rs` — Project orchestration to be simplified now that JAR downloads are gone.
- [x] `src/java/grade/results.rs` — Base grading types updated; Rhai-era borrowing removed.
- [x] `src/java/grade/context.rs` — Retrieval/context builder refactored and tested.
- [ ] `src/java/grade/query.rs` — Still coupled to `FnPtr`/Rhai filters; needs redesign.
- [ ] `src/java/grade/tests.rs` — Unit/hidden test graders pending cleanup after context changes.
- [ ] `src/java/grade/diff.rs` — Diff grader needs modernization to match new context APIs.
- [ ] `src/java/grade/docs.rs` — Documentation grader awaiting parser/context follow-up.
- [ ] `src/java/grade/feedback.rs` — Feedback persistence layer still reflects Supabase-first flows.
- [ ] `src/java/grade/gradescope.rs` — Gradescope orchestration requires cleanup after feedback changes.
- [ ] `src/java/grade/mod.rs` — Exports to be revisited once grader internals settle.
- [ ] `src/java/mod.rs` — Module wiring needs a post-split audit for visibility and exports.
- [ ] `src/lib.rs` — Library surface still hosts temporary guards around disabled `grade`.
- [ ] `src/main.rs` — CLI entry to be revisited once scripting path lands.

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
- `clean()` still removes `.vscode` artifacts but does not recreate them.

## Quick Test Checklist

- `cargo check` — should pass without warnings.
- `cargo fmt` — format before handoff.
- `cargo clippy --all-targets` — minimum lint bar.
- `umm run <ClassWithMain>` — runs the Java class with discovered paths.
- `umm test <TestClass> [tests...]` — uses existing classpath; ensure JUnit jars are present locally.
- `umm doc-check <Class>` — runs `javac -Xdoclint`.
- `umm grade <...>` — returns the disabled-message on main; use the branch prototype only for the Python trial.

## Contact Points (Authoritative)

- Paths & layout: `src/java/paths.rs`, `src/java/project.rs`, `src/java/mod.rs`
- Classpath/sourcepath & IO: `src/util.rs`
- CLI wiring: `src/main.rs`
- Graders & feedback: `src/java/grade/*`
- Env/services/prompts/runtime: `src/config.rs`
- Cleanup helpers: `src/lib.rs` (`clean()`)

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

- 2025-09-28: Added onboarding quickstart, CLI contract, env/glossary tables, clarified config/AtomicBool behavior, tightened scripting decision record + appendix, and documented maintenance commands.
