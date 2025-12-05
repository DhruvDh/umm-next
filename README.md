# umm

- [umm](#umm)
  - [Introduction](#introduction)
  - [Documentation](#documentation)
  - [Installation](#installation)
  - [Auto-grading](#auto-grading)
    - [Introduction](#introduction-1)
    - [Sample grading script](#sample-grading-script)
    - [Output](#output)
  - [Configuration](#configuration)
  - [License](#license)

## Introduction

A java build tool for novices, that doubles as a scriptable autograder.

## Documentation

Rustdoc can be found at https://umm-docs.pages.dev/umm/

## Installation

You would need rust installed, ideally the nightly toolchain. You can visit https://rustup.rs/ to find out how to install this on your computer, just make sure you install the "nightly" toolchain instead of stable.

On Linux, Windows Subsystem for Linux (WSL), and Mac you should be able to run `curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly` on a terminal to install the nightly toolchain for rust.

Once you are done, just type `cargo install --git=https://github.com/DhruvDh/umm.git`, and it should compile and install it on your system.

## Auto-grading

`umm` now runs grading flows written in [Rune](https://rune-rs.github.io/). Ship a script with an async `main` function and execute it with `umm java grade path/to/script.rn`. A minimal example lives in `examples/sample.rn`.

The Rune surface leans on the [`bon`](https://docs.rs/bon) builders exposed from the Rust graders (e.g., `DocsGrader::builder().project(...).files([...]).run().await?`). See `examples/sample.rn` for the canonical shape. **We do not currently register additional Rust helper functions into Rune beyond the graders’ builder surfaces.**

Here are the currently exposed helpers:
> **This is an in-progress draft.**
>
> `method_name(argument_types) -> return_type`

- `ProjectPaths` — workspace layout helper.
  - `new_project_paths()` -> `ProjectPathsBuilder` with setters `root_dir`, `source_dir`, `build_dir`, `test_dir`, `lib_dir`, `umm_dir`, `report_dir`, and `build()`.

- `Project` — discovered Java workspace.
  - `new_project() -> Project` — discovers files under the default layout.
  - `new_project_from_paths(ProjectPaths) -> Project` — uses explicit paths.
  - `info()` — prints a JSON description of the project.

- Graders (all are async; call `.run().await?`):
  - `new_docs_grader()` — `.project(...)`, `.files([...])`, `.req_name(...)`, `.out_of(...)`, optional `.penalty(...)`.
  - `new_by_unit_test_grader()` — `.project(...)`, `.test_files([...])`, `.expected_tests([...])`, `.req_name(...)`, `.out_of(...)`.
  - `new_unit_test_grader()` — `.project(...)`, `.target_test([...])`, `.target_class([...])`, `.excluded_methods([...])`, `.avoid_calls_to([...])`, `.req_name(...)`, `.out_of(...)`.
  - `new_by_hidden_test_grader()` — `.url(...)`, `.test_class_name(...)`, `.req_name(...)`, `.out_of(...)`.
  - `new_diff_grader()` — `.project(...)`, `.file("Main")`, `.cases([(expected, Option::<&str>::None)])`, optional `.ignore_case(...)` / `.preserve_whitespace(...)`, `.req_name(...)`, `.out_of(...)`.
  - `new_query_grader()` — `.project(...)`, `.file(...)`, `.queries_with_capture([(query, capture)])` or `.queries([...])`, `.constraint(QueryConstraint::...)`, `.reason(...)`, `.req_name(...)`, `.out_of(...)`.

- Results and helpers
  - `grade_all([GradeResult]) -> Vec<GradeResult>` — combine grader outputs.
  - `show_results(Vec<GradeResult>)` — render Gradescope-style output.
  - `GradeResult::prompt()` — returns serialized prompt JSON when present.

Run mutation testing outside the Codex sandbox to confirm pass/fail.

### Sample grading script

This script is a sample. The script uses several graders, each with its specific function, to evaluate the project and assign a grade.

The first grader, `new_docs_grader()`, is used to evaluate the project's documentation. It takes the project as input, as well as a list of files to be graded, and assigns a grade out of 5 points, with a penalty of 1 point for poor documentation.

The second grader, `new_diff_grader()`, runs the student's program and checks stdout against an expected string.

The third grader, `new_by_unit_test_grader()`, runs visible JUnit tests and checks that expected tests are present.

The fourth grader, `new_query_grader()`, runs a tree-sitter query to ensure the source contains a loop.

The fifth grader, `new_unit_test_grader()`, runs mutation tests (PIT) against selected targets.

The sixth grader, `new_by_hidden_test_grader()`, runs hidden JUnit tests fetched from a URL.

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
};

pub async fn main() {
    let project = new_project()?;

    let req_1 = new_docs_grader()
        .project(project.clone())
        .files(["pyramid_scheme.LinkedTree"])
        .out_of(5.0)
        .req_name("1")
        .penalty(1.0)
        .run()
        .await?;

    let req_2 = new_diff_grader()
        .project(project.clone())
        .file("Main")
        .req_name("2")
        .out_of(5.0)
        .cases([("Hello from Rune\n", None)])
        .run()
        .await?;

    let req_3 = new_by_unit_test_grader()
        .project(project.clone())
        .test_files(["pyramid_scheme.LinkedTreeTest"])
        .expected_tests([
            "pyramid_scheme.LinkedTreeTest#testGetRootElement",
            "pyramid_scheme.LinkedTreeTest#testAddChild",
            "pyramid_scheme.LinkedTreeTest#testFindNode",
            "pyramid_scheme.LinkedTreeTest#testContains",
            "pyramid_scheme.LinkedTreeTest#testSize",
        ])
        .out_of(5.0)
        .req_name("3")
        .run()
        .await?;

    let req_4 = new_query_grader()
        .project(project.clone())
        .file("Main")
        .queries_with_capture([("((for_statement) @loop)", "loop")])
        .out_of(5.0)
        .req_name("4")
        .reason("Should contain a for loop")
        .run()
        .await?;

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

    let req_6 = new_by_hidden_test_grader()
        .url("https://www.dropbox.com/s/47jd1jru1f1i0cc/ABCTest.java?raw=1")
        .test_class_name("ABCTest")
        .out_of(5.0)
        .req_name("6")
        .run()
        .await?;

    let reqs = grade_all([req_1, req_2, req_3, req_4, req_5, req_6])?;

    show_results(reqs.clone())?;

    let total: f64 = reqs.iter().map(|r| r.score()).sum();
    let out_of: f64 = reqs.iter().map(|r| r.out_of()).sum();

    if total > 0.7 * out_of {
        println!("p;{}", total as i64);
    } else {
        println!("np");
    }

    Ok(())
}
```
### Output 

```
╭──────────────────────────────────────────────────────────╮
│                  SAMPLE SCRIPT OUTPUT                    │
╰──────────────────────────────────────────────────────────╯
┌────────────────────────────────────────────────────────────┬
│        Check javadoc for pyramid_scheme.LinkedTree         │
├────────────────────────────────────────────────────────────┼
│           File           │ Line │         Message          │
├──────────────────────────┼──────┼──────────────────────────┤
│ ./src/pyramid_scheme/Lin │  14  │   no main description    │
│       kedTree.java       │      │                          │
├──────────────────────────┼──────┼──────────────────────────┤
│ ./src/pyramid_scheme/Lin │  15  │ no description for @para │
│       kedTree.java       │      │            m             │
├──────────────────────────┼──────┼──────────────────────────┤
│ ./src/pyramid_scheme/Lin │  29  │ no description for @para │
│       kedTree.java       │      │            m             │
├──────────────────────────┼──────┼──────────────────────────┤
│ ./src/pyramid_scheme/Lin │  56  │  Error: unknown tag: T   │
│       kedTree.java       │      │                          │
├──────────────────────────┼──────┼──────────────────────────┤
│ ./src/pyramid_scheme/Lin │  72  │ no description for @thro │
│       kedTree.java       │      │            ws            │
├──────────────────────────┼──────┼──────────────────────────┤
│ ./src/pyramid_scheme/Lin │ 251  │ no description for @para │
│       kedTree.java       │      │            m             │
├──────────────────────────┼──────┼──────────────────────────┤
│                     -18 due to 6 nits                      │
└────────────────────────────────────────────────────────────┴

Running Mutation tests -
11:37:54 PM PIT >> INFO : Verbose logging is disabled. If you encounter a problem, please enable it before reporting an issue.
11:37:54 PM PIT >> INFO : Incremental analysis reduced number of mutations by 0
11:37:54 PM PIT >> INFO : Created  1 mutation test units in pre scan
11:37:54 PM PIT >> INFO : Sending 1 test classes to minion
11:37:54 PM PIT >> INFO : Sent tests to minion
11:37:54 PM PIT >> SEVERE : Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testSize(pyramid_scheme.LinkedTreeTest)]] did not pass without mutation.
11:37:54 PM PIT >> SEVERE : Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testAddChild(pyramid_scheme.LinkedTreeTest)]] did not pass without mutation.
11:37:54 PM PIT >> SEVERE : Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testFindNode(pyramid_scheme.LinkedTreeTest)]] did not pass without mutation.
11:37:54 PM PIT >> SEVERE : Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testContains(pyramid_scheme.LinkedTreeTest)]] did not pass without mutation.
11:37:54 PM PIT >> SEVERE : Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testGetRootElement(pyramid_scheme.LinkedTreeTest)]] did not pass without mutation.
11:37:54 PM PIT >> INFO : Calculated coverage in 0 seconds.
11:37:54 PM PIT >> SEVERE : Tests failing without mutation: 
Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testSize(pyramid_scheme.LinkedTreeTest)]]
Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testAddChild(pyramid_scheme.LinkedTreeTest)]]
Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testFindNode(pyramid_scheme.LinkedTreeTest)]]
Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testContains(pyramid_scheme.LinkedTreeTest)]]
Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testGetRootElement(pyramid_scheme.LinkedTreeTest)]]
Exception in thread "main" org.pitest.help.PitHelpError: 5 tests did not pass without mutation when calculating line coverage. Mutation testing requires a green suite.
See http://pitest.org for more details.
	at org.pitest.coverage.execute.DefaultCoverageGenerator.verifyBuildSuitableForMutationTesting(DefaultCoverageGenerator.java:115)
	at org.pitest.coverage.execute.DefaultCoverageGenerator.calculateCoverage(DefaultCoverageGenerator.java:97)
	at org.pitest.coverage.execute.DefaultCoverageGenerator.calculateCoverage(DefaultCoverageGenerator.java:52)
	at org.pitest.mutationtest.tooling.MutationCoverage.runAnalysis(MutationCoverage.java:148)
	at org.pitest.mutationtest.tooling.MutationCoverage.runReport(MutationCoverage.java:138)
	at org.pitest.mutationtest.tooling.EntryPoint.execute(EntryPoint.java:129)
	at org.pitest.mutationtest.tooling.EntryPoint.execute(EntryPoint.java:57)
	at org.pitest.mutationtest.commandline.MutationCoverageReport.runReport(MutationCoverageReport.java:98)
	at org.pitest.mutationtest.commandline.MutationCoverageReport.main(MutationCoverageReport.java:45)
/
┌────────────────────────────────────────────────────────────┬
│       Check javadoc for pyramid_scheme.PyramidScheme       │
├────────────────────────────────────────────────────────────┼
│           File           │ Line │         Message          │
├──────────────────────────┼──────┼──────────────────────────┤
│ ./src/pyramid_scheme/Pyr │  10  │ Error: unknown tag: Pers │
│     amidScheme.java      │      │            on            │
├──────────────────────────┼──────┼──────────────────────────┤
│ ./src/pyramid_scheme/Pyr │  18  │        no comment        │
│     amidScheme.java      │      │                          │
├──────────────────────────┼──────┼──────────────────────────┤
│ ./src/pyramid_scheme/Pyr │  19  │        no comment        │
│     amidScheme.java      │      │                          │
├──────────────────────────┼──────┼──────────────────────────┤
│ ./src/pyramid_scheme/Pyr │ 165  │ no description for @thro │
│     amidScheme.java      │      │            ws            │
├──────────────────────────┼──────┼──────────────────────────┤
│ ./src/pyramid_scheme/Pyr │ 241  │ no description for @retu │
│     amidScheme.java      │      │            rn            │
├──────────────────────────┼──────┼──────────────────────────┤
│                     -15 due to 5 nits                      │
└────────────────────────────────────────────────────────────┴

┌─────────────────────────────────────────────────────┬
│                  Grading Overview                   │
├─────────────────────────────────────────────────────┼
│ Requirement │   Grade    │          Reason          │
├─────────────┼────────────┼──────────────────────────┤
│      1      │    0/10    │        See above.        │
├─────────────┼────────────┼──────────────────────────┤
│      2      │ 0.00/20.00 │   - 0/5 tests passing.   │
├─────────────┼────────────┼──────────────────────────┤
│      2      │    0/20    │ Something went wrong whi │
│             │            │ le running mutation test │
│             │            │       s, skipping.       │
├─────────────┼────────────┼──────────────────────────┤
│      3      │    0/10    │        See above.        │
├─────────────┼────────────┼──────────────────────────┤
│      3      │ 0.00/30.00 │   - 0/3 tests passing.   │
├─────────────┼────────────┼──────────────────────────┤
│      4      │ 0.00/30.00 │   - 0/5 tests passing.   │
├─────────────┼────────────┼──────────────────────────┤
│                 Total: 0.00/120.00                  │
└─────────────────────────────────────────────────────┴

f;0
```

## Configuration

- `OPENAI_ENDPOINT`: Base API URL (e.g., `https://api.openai.com/v1`). Required for SLO feedback.
- `OPENAI_API_KEY_SLO`: API key used for SLO feedback requests. Required for SLO feedback.
- `OPENAI_MODEL`: Model name for SLO feedback (e.g., `gpt-4.1`). Required for SLO feedback.
- `OPENAI_TEMPERATURE`: Optional float. If set and valid, included in Chat Completions requests; otherwise omitted.
- `OPENAI_TOP_P`: Optional float. If set and valid, included in Chat Completions requests; otherwise omitted.
- `OPENAI_REASONING_EFFORT`: Optional string, one of `low`, `medium`, `high`. Defaults to `medium` when not set.
- `SUPABASE_URL`: Supabase project URL (base, e.g., `https://<project>.supabase.co`). Required. The program exits with an error if missing.
- `SUPABASE_ANON_KEY`: Supabase anon key. Required. The program exits with an error if missing.

Notes
- `OPENAI_TEMPERATURE` and `OPENAI_TOP_P` are only sent if provided; there is no default implicit value passed.
- `OPENAI_REASONING_EFFORT` always applies a value; when not set, it defaults to `medium`.
- `SUPABASE_URL` is converted to a Postgrest endpoint by appending `/rest/v1`.
- If `SUPABASE_URL` or `SUPABASE_ANON_KEY` is not set, the program prints a helpful error and exits.

Setup
- Copy `.env.example` to `.env` and fill in values.
- Existing OS environment variables take precedence over `.env`; unset values are read from `.env` if present.
## License

See `license.html` for a list of all licenses used in this project.
