# Agent Checklist

Agents working in this repository must read this file before making changes.
Use it as a quick-reference checklist to stay aligned with project expectations.

## 1. Before You Start

- Open `context.md` for the latest architectural notes and roadmap (“Project
  File Map” and “Plan” sections must be current). If it feels outdated, update
  it as you work.
- Review the current git diff and outstanding TODOs in code or `context.md`.

## 2. Working Guidelines

- Prefer instance-scoped configuration (e.g., `ProjectPaths`) over global
  statics. Do **not** re-introduce the removed `lazy_static` path constants.
- Rhai support is being removed. Do not resurrect Rhai helpers; instead push the
  design towards the planned Rune integration.
- For linting/tests: run `cargo fmt` and `cargo clippy --all-targets` before you
  hand back work. Use `cargo check` as needed, but clippy is the minimum bar.
- Keep `context.md` untracked (Git ignores it) and update it whenever you make
  structural/code-architecture changes.
- When the work naturally splits into a commit, suggest or create a concise,
  descriptive commit message (e.g., “refactor: route project paths through
  ProjectPaths”).
- Treat the user as the sole gatekeeper for version-control changes. Do not
  apply edits beyond exploratory inspection without explicit approval, and
  confirm proposed code adjustments before modifying files.

## 3. During the Task

- If the task is non-trivial, outline a short plan before large edits unless the
  user explicitly skips it.
- Ask clarifying questions when requirements are ambiguous.
- Keep changes focused; avoid drive-by refactors unless they’re necessary or
  cleared by the user.

## 4. Handoff & Wrap-Up

- Run `cargo fmt` and `cargo clippy --all-targets` and ensure they pass.
- Update `context.md` with:
  - Any new architectural decisions.
  - Changes to the “Project File Map” or “Plan”.
  - Notes future agents need to know.
- Summarize the work in your final response, referencing files with
  `path:line` format where edits occurred.

Following this checklist keeps the repository consistent and makes handoffs to
future agents seamless.
