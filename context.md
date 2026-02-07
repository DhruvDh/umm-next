# Repository Context (Operational Snapshot)

This document is the current-state operating reference for this repository root. It is intentionally not a migration diary. It records what the system does now, which constraints are active now, and which work should happen next.

## Status (2026-02-07)

The repository is in a post-remediation state where both Java and Python grading flows execute Rune scripts through language-specific grade commands. The source of truth for command shape is `src/main.rs`, the source of truth for runtime configuration is `src/config.rs`, and the source of truth for scripting module installation is `src/scripting/rune/mod.rs` with module declarations in `src/scripting/rune/modules/mod.rs`.

The central architectural decision is simple: the CLI is language-scoped (`java` and `python`), grading entrypoints are script-path only, and runtime behavior is controlled through `ConfigState` rather than scattered environment reads.

## CLI Contract (Authoritative)

The CLI is defined in `src/main.rs`.

| Command | Arguments | Behavior |
|---|---|---|
| `umm java run` | `FILENAME` | Runs a Java file with a `main` method. |
| `umm java check` | `FILENAME` | Checks Java syntax/compile diagnostics. |
| `umm java test` | `FILENAME [TESTNAME...]` | Runs JUnit tests (optionally scoped to named tests). |
| `umm java doc-check` | `FILENAME` | Runs Java doc checks. |
| `umm java grade` | `SCRIPT_PATH` | Executes a Rune grading script for Java workflows. |
| `umm java info` | none | Prints Java project metadata as JSON. |
| `umm python run` | `FILENAME` | Runs a Python file. |
| `umm python check` | `FILENAME` | Checks Python syntax. |
| `umm python test` | `FILENAME` | Runs pytest on a test file. |
| `umm python lint` | `[FILENAME...]` | Runs Ruff lint, defaulting to project root if omitted. |
| `umm python format` | `[FILENAME...]` | Runs Black format, defaulting to project root if omitted. |
| `umm python grade` | `SCRIPT_PATH` | Executes a Rune grading script for Python workflows. |
| `umm python info` | none | Prints Python project metadata as JSON. |
| `umm update` | none | Updates the `umm` binary from releases. |

A top-level grade command is not part of the current interface.

## Environment Variables (Authoritative)

The environment contract below is derived from `src/config.rs`.

| Variable | Purpose | Default / Fallback |
|---|---|---|
| `SUPABASE_URL` | Supabase base URL for feedback persistence | Optional; feature is skipped when unset or empty. |
| `SUPABASE_ANON_KEY` | Supabase API key for PostgREST writes | Optional; feature is skipped when unset or empty. |
| `OPENAI_ENDPOINT` | Base URL for OpenAI-compatible API | Required only when OpenAI-backed grading features are invoked. |
| `OPENAI_API_KEY_SLO` | API key for OpenAI-backed SLO/code-review calls | Required only when those features are invoked. |
| `OPENAI_MODEL` | Model identifier | Required only when OpenAI-backed flows are invoked. |
| `OPENAI_TEMPERATURE` | Optional temperature override | Omitted when missing/invalid. |
| `OPENAI_TOP_P` | Optional top-p override | Omitted when missing/invalid. |
| `OPENAI_REASONING_EFFORT` | Reasoning effort hint | Defaults to `medium` when missing/unrecognized. |
| `UMM_COURSE` | Course metadata | Defaults to `ITSC 2214`. |
| `UMM_TERM` | Term metadata | Defaults to `Fall 2022`. |
| `UMM_RETRIEVAL_ENDPOINT` | Active retrieval service endpoint | Empty or whitespace values are treated as unset and fall back to `DEFAULT_RETRIEVAL_ENDPOINT`. |
| `UMM_JAVAC_TIMEOUT_SECS` | Timeout for `javac` work | Defaults to `30` seconds. |
| `UMM_JAVA_TIMEOUT_SECS` | Timeout for Java/JUnit execution | Defaults to `60` seconds. |
| `UMM_PYTHON_TIMEOUT_SECS` | General Python execution timeout | Defaults to `60` seconds. |
| `UMM_PYTHON_LINT_TIMEOUT_SECS` | Python lint timeout | Defaults to `30` seconds. |
| `UMM_PYTHON_TEST_TIMEOUT_SECS` | Python test timeout | Defaults to `120` seconds. |

## Runtime/Config Model

The runtime model centers on `ConfigState` in `src/config.rs`. `ConfigState` owns the shared HTTP client, lazy PostgREST client initialization, Java and Python config bundles, retrieval toggles, and timeout surfaces. This avoids configuration drift because call sites read from a single state object through accessor functions such as `python_lint_timeout()`, `python_test_timeout()`, and `retrieval_endpoint()`.

Scripting execution is centralized in `src/scripting/mod.rs` and module installation is centralized in `src/scripting/rune/mod.rs`. The installed Rune namespace is `umm::{java, python, gradescope, config, retrieval}` according to `src/scripting/rune/modules/mod.rs`.

This architecture keeps command handling thin in the CLI and pushes behavior into versioned, testable Rust modules.

## Grading Reliability Semantics (Current)

Java Gradescope artifact generation is designed to degrade gracefully for external-service faults when a valid `results.json` can still be emitted. In `src/java/grade/gradescope.rs`, failures in feedback/SLO subflows are captured as warnings, surfaced in per-test output text, summarized in submission-level output, and serialized into `extra_data.warnings`.

Python code-review grading is designed to fail closed when structured model output is invalid. In `src/python/grade/code_review.rs`, model responses are parsed into a structured decision schema, a repair retry is attempted once on parse failure, and persistent parse failure yields a zero score with explicit failure rationale. Bounds checks in fallback JSON extraction prevent panic on malformed brace ordering.

## Project File Map (Authoritative)

Map inclusion rule: include paths only when they define architecture boundaries, runtime entrypoints, scripting installation, or grading behavior contracts. Use shell discovery (`ls --tree`, `find`, `rg --files`) for exhaustive listings.

- CLI and runtime boundary (why it matters: this is the command contract and global behavior surface):
  - `src/main.rs`
  - `src/lib.rs`
  - `src/config.rs`
  - `src/process.rs`
  - `src/retrieval.rs`

- Scripting installation boundary (why it matters: this determines which `umm::*` APIs scripts can call):
  - `src/scripting/mod.rs`
  - `src/scripting/rune/mod.rs`
  - `src/scripting/rune/modules/mod.rs`
  - `src/scripting/rune/modules/java.rs`
  - `src/scripting/rune/modules/python.rs`
  - `src/scripting/rune/modules/gradescope.rs`
  - `src/scripting/rune/modules/config.rs`
  - `src/scripting/rune/modules/retrieval.rs`

- Java architecture root and grading surfaces (why it matters: these files define Java project discovery, parsing, and score/report generation):
  - `src/java/mod.rs`
  - `src/java/project.rs`
  - `src/java/file.rs`
  - `src/java/parsers.rs`
  - `src/java/grade/mod.rs`
  - `src/java/grade/results.rs`
  - `src/java/grade/gradescope.rs`
  - `src/java/grade/feedback.rs`
  - `src/java/grade/context.rs`

- Python architecture root and grading surfaces (why it matters: these files define Python project discovery, runtime execution, and pass/fail grading semantics):
  - `src/python/mod.rs`
  - `src/python/project.rs`
  - `src/python/file.rs`
  - `src/python/config.rs`
  - `src/python/grade/mod.rs`
  - `src/python/grade/results.rs`
  - `src/python/grade/code_review.rs`
  - `src/python/grade/context.rs`

## Current Risks

The architecture is stable, but four active risks remain and should shape near-term work. First, external dependencies (OpenAI/Supabase/networked retrieval) remain failure-prone, so warning telemetry and fallback messages must stay regression-tested. Second, parser behavior now includes Windows-path handling in Java diagnostics, but this remains a compatibility surface that can regress quietly without fixture coverage expansion. Third, model-output contracts in Python code review are more robust than before but still rely on prompt/schema discipline, so parser and retry behavior must remain tightly tested. Fourth, command and documentation drift can return quickly because the CLI surface is broad across two language trees.

## Plan

This roadmap is intentionally execution-oriented and tied to concrete outcomes.

1. Owner: Maintainer responsible for grading runtime. Deliverable: harden resilience telemetry in Gradescope JSON.
Outcome: warning codes and scopes remain stable and machine-consumable, and regressions are caught by integration tests.
Acceptance: warnings appear in `tests[].output`, submission `output`, and `extra_data.warnings` for simulated external failures.

2. Owner: Maintainer responsible for Python grading. Deliverable: expand structured-response robustness tests in code-review grading.
Outcome: malformed model output cannot crash grading and always follows retry then fail-closed semantics.
Acceptance: integration tests cover malformed ordering, missing braces, and non-JSON payloads with deterministic zero-score outcomes after retry exhaustion.

3. Owner: Maintainer responsible for cross-platform diagnostics. Deliverable: strengthen Java parser fixtures for path and diagnostic edge cases.
Outcome: diagnostic parsing remains stable across Unix-style and Windows-style path formats without platform regressions.
Acceptance: parser tests include representative drive-letter and mixed-separator cases and preserve existing Unix behavior.

4. Owner: Maintainer responsible for doc-runtime consistency. Deliverable: add a lightweight doc consistency check workflow.
Outcome: command and environment tables in `context.md` stay aligned with `main.rs` and `config.rs`.
Acceptance: pre-merge check confirms documented commands/env variables still exist in code.

## Working Rules For Future Updates

When architecture changes, update this file in the same pull request. Keep this document source-aligned: if a claim is not directly verifiable in repository code, remove it. Keep the narrative focused on current behavior, current risk, and next work. Do not append prototype archives or historical migration notes here.

## Quick Validation Checklist

Run these before handing off substantial changes:

- `cargo fmt`
- `cargo clippy --all-targets`
- `cargo test`
- `cargo test --test gradescope_resilience_tests`

For command-surface spot checks:

- `umm java grade SCRIPT_PATH`
- `umm python grade SCRIPT_PATH`

## Doc Change Log

- 2026-02-07: Rewrote `context.md` as an operational snapshot, aligned command/config contracts to live code, added authoritative `Project File Map` and execution-oriented `Plan`, and removed legacy prototype/migration narrative.
