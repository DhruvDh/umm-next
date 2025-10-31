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

The notes below document the legacy Rhai prototype and will be updated as the Rune API settles.

### Introduction

Rhai is a lightweight embeddable scripting language meant to make it easy to use your Rust-written structs, their methods, and your functions dynamically without the need for recompilation.

Here are some structs (classes) to help you get going -
> **This is an in-progress draft.**
> 
> `method_name(arguement_types) -> return_type`

- `JavaProject` - Struct representing a Java project.
  - `new_java_project() -> JavaProject` - initializes a java project by discovering files in the source path directory. Also downloads jar files if needed.
  - `identify(String) -> JavaFile`  -  attempts to identify the correct file from the project from a partial or fully formed name as expected by a java compiler. Returns a `JavaFile` of the identified file in the project.
  - `files() -> Array` - returns an array of `JavaFiles` discovered in the project
  - `info()` - prints a minified JSON representation of all the files in the project

- `JavaFile` - a file in the discovered project representing any class, interface, or test.
  - `new_java_file() -> JavaFile` - a constructor, is not meant to be used inside a script. `JavaFile`s should be discovered by the project.
  - `check()` - checks for compiler errors, and reports them on stdout/stderr. Also ensures a corresponding `.class` file is present in the target directory after a `check()` completes.
  - `doc_check() -> String` - asks javac for documentation lints using the `-Xdoclint` flag. Returns compiler output as a String. There is a parser that can help parse this output which is not currently exposed.
  - `run()` - runs the file, and prints output to stdout/stderr.
  - `query(String) -> Array` -> accepts a Treesitter query as a string (use backticks for multiline strings), and returns an Array of [Object Maps](https://rhai.rs/book/language/object-maps.html) (dictionary). Each element of the array represents one match, and each object map contains captured variable names as the key, and captured values as the value.
  - `test(Array) -> String` - Can be called on JUnit test files. It takes in an Array of strings representing test method names. These test methods must exist within this test file. Returns output from JUnit as a string.
  - `kind() -> JavaFileType` - returns the kind of file (Class, Interface, ClassWithMain, Test)
  - `file_name() -> String` - returns the name of the file.
  - `path() -> String` - returns the relative path to the file as a string.
  - `test_methods() -> Array` - Can be called on JUnit test files. It returns an Array of test_method names discovered in the file.

- `JavaParser` - a wrapper around a treesitter parser. There should not be a need to use this, most of the time what you want to do is call `query()` on a `JavaFile`.
  - `new_java_parser()` - a constructor, not meant to be used inside a script. It is ideal if you use `JavaFile`'s `query()`.
  - `code() -> String` - returns the source code the parser is working with.
  - `set_code(String)` - a setter for the source code the parser is working with.
  - `query(String) -> Vec<Dict>` - Currently this method returns a value that cannot be used inside a rhai script, please use `JavaFile`'s `query(String)` instead.

- `Grade` - A struct representing a grade.
  - `new_grade(float, float) -> Grade` - takes the actual grade received, and the maximum grade as floating point numbers, and returns a `Grade`.
  - `from_string(String) -> Grade` - takes a string in this format - `"80/100"` and returns a new `Grade`.
  - `grade() -> float` - a getter for the grade recieved.
  - `grade(float)` - a setter for the grade received.
  - `out_of() -> float` - a getter for the maximum grade.
  - `out_of(float)` a setter for the maximum grade.
  - `to_string()` - returns the grade in this format as a string - `"80/100"`.

### Sample grading script

This script is a sample. The script uses several graders, each with its specific function, to evaluate the project and assign a grade.

The first grader, `new_docs_grader()`, is used to evaluate the project's documentation. It takes the project as input, as well as a list of files to be graded, and assigns a grade out of 10 points, with a penalty of 3 points for poor documentation.

The second grader, `new_by_unit_test_grader()`, is used to evaluate the project's unit tests. It takes the project, a list of test files, and a list of expected tests as input, and assigns a grade out of 20 points.

The third grader, `new_unit_test_grader()`, is also used to evaluate the project's unit tests. It takes different inputs than the second grader, such as the names of the target test and class, and a list of excluded methods and avoided calls. It also assigns a grade out of 20 points.

The fourth and fifth graders are similar to the first and second graders but are used to evaluate a different set of files and tests.

The sixth grader, `new_by_hidden_test_grader()`, is used to evaluate the project's performance on hidden tests. It takes the URL of the hidden test files, the name of the test class, and the requirements it is grading and assigns a grade out of 30 points.

```rust
let project = new_java_project();

let req_1 = new_docs_grader()
    .project(project)
    .files(["pyramid_scheme.LinkedTree"])
    .out_of(10.0)
    .req_name("1")
    .penalty(3.0)
    .run();

let req_2 = new_by_unit_test_grader()
    .project(project)
    .test_files(["pyramid_scheme.LinkedTreeTest"])
    .expected_tests([
        "pyramid_scheme.LinkedTreeTest#testGetRootElement",
        "pyramid_scheme.LinkedTreeTest#testAddChild",
        "pyramid_scheme.LinkedTreeTest#testFindNode",
        "pyramid_scheme.LinkedTreeTest#testContains",
        "pyramid_scheme.LinkedTreeTest#testSize",
    ])
    .out_of(20.0)
    .req_name("2")
    .run();

let req_3 = new_unit_test_grader()
    .req_name("2")
    .out_of(20.0)
    .target_test(["pyramid_scheme.LinkedTreeTest"])
    .target_class(["pyramid_scheme.LinkedTree"])
    .excluded_methods([])
    .avoid_calls_to([])
    .run();

let req_4 = new_docs_grader()
    .project(project)
    .files(["pyramid_scheme.PyramidScheme"])
    .out_of(10.0)
    .req_name("3")
    .penalty(3.0)
    .run();

let req_5 = new_by_unit_test_grader()
    .project(project)
    .test_files(["pyramid_scheme.PyramidSchemeTest"])
    .expected_tests([
        "pyramid_scheme.PyramidSchemeTest#testWhoBenefits",
        "pyramid_scheme.PyramidSchemeTest#testAddChild",
        "pyramid_scheme.PyramidSchemeTest#testInitiateCollapse",
    ])
    .out_of(30.0)
    .req_name("3")
    .run();

let req_6 = new_by_hidden_test_grader()
    .url("https://www.dropbox.com/s/47jd1jru1f1i0cc/ABCTest.java?raw=1")
    .test_class_name("ABCTest")
    .out_of(30.0)
    .req_name("4")
    .run();

let reqs = [req_1, req_2, req_3, req_4, req_5, req_6];

// arguements: 
// - array of grade results
show_results(reqs);

let total = 0.0;
let out_of = 0.0;
for req in reqs {
    total = total + req.grade();
    out_of = out_of + req.out_of();
}

if total > (0.7 * out_of) {
    print("p;" + total.to_int())
} else {
    print("np")
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
