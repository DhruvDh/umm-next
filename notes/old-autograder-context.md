<current-context>
  <folder path=".">
    <file-contents path="./README.md" name="README.md">
# umm

- [umm](#umm)
  - [Introduction](#introduction)
  - [Documentation](#documentation)
  - [Installation](#installation)
  - [Auto-grading](#auto-grading)
    - [Introduction](#introduction-1)
    - [Sample grading script](#sample-grading-script)
    - [Output](#output)
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

Also allows for running auto-grading scripts based on [Rhai](https://rhai.rs/book/about/index.html).

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
## License

See `license.html` for a list of all licenses used in this project.

    </file-contents>
  </folder>

  <folder path="./src">
    <file-contents path="./src/constants.rs" name="constants.rs">
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use lazy_static::lazy_static;
use postgrest::Postgrest;
use rhai::AST;
use state::InitCell;

// TODO: replace with https://lib.rs/crates/state
lazy_static! {
    /// Path to project root
    pub static ref ROOT_DIR: PathBuf = PathBuf::from(".");
    /// Directory for source files
    pub static ref SOURCE_DIR: PathBuf = PathBuf::from(".").join("src");
    /// Directory to store compiler artifacts
    pub static ref BUILD_DIR: PathBuf = PathBuf::from(".").join("target");
    /// Directory for test files
    pub static ref TEST_DIR: PathBuf = PathBuf::from(".").join("test");
    /// Directory for libraries, jars
    pub static ref LIB_DIR: PathBuf = PathBuf::from(".").join("lib");
    /// Directory for `umm` artifacts
    pub static ref UMM_DIR: PathBuf = PathBuf::from(".").join(".umm");
    /// Platform specific separator character for javac paths
    pub static ref SEPARATOR: &'static str = if cfg!(windows) { ";" } else { ":" };
    /// Supabase public api key
    pub static ref SUPABASE_KEY: String = String::from("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InV5YW5jenRtempsZWtvamVwcm9qIiwicm9sZSI6ImFub24iLCJpYXQiOjE2NjA4NDA1NzgsImV4cCI6MTk3NjQxNjU3OH0.yMvOYM0AM61v6MRsHUSgO0BPrQHTde2AiKzE0b4H4lo");
    /// PostGrest client
    pub static ref POSTGREST_CLIENT: Postgrest = Postgrest::new("https://uyancztmzjlekojeproj.supabase.co/rest/v1")
            .insert_header("apiKey", SUPABASE_KEY.clone());
    /// Runtime
    pub static ref RUNTIME: tokio::runtime::Runtime = tokio::runtime::Runtime::new().unwrap();
    /// ChatGPT System Message intro
    pub static ref SYSTEM_MESSAGE_INTRO: String = include_str!("prompts/system_message_intro.md").into();
    /// ChatGPT System Message outro
    pub static ref SYSTEM_MESSAGE_OUTRO: String = include_str!("prompts/system_message_outro.md").into();
    /// Entire ChatGPT System Message
    pub static ref SYSTEM_MESSAGE: String = format!("{}\n{}", *SYSTEM_MESSAGE_INTRO, *SYSTEM_MESSAGE_OUTRO);
    /// Retrieval System Message intro
    pub static ref RETRIEVAL_MESSAGE_INTRO: String = include_str!("prompts/retrieval_system_message_intro.md").into();
    /// Retrieval System Message outro
    pub static ref RETRIEVAL_MESSAGE_OUTRO: String = include_str!("prompts/retrieval_system_message_outro.md").into();
    /// Rhai script as a AST, behind an mutex.
    pub static ref SCRIPT_AST: Arc<Mutex<AST>> = Arc::new(Mutex::new(AST::empty()));
    /// System Message for Algorithmic Solutions SLO
    pub static ref ALGORITHMIC_SOLUTIONS_SLO: String = format!(include_str!("prompts/slos/system_message_intro.md"), SLO_DESCRIPTION = include_str!("prompts/slos/algorithmic_solutions_quant.md"));
    /// System Message for Code Readability SLO
    pub static ref CODE_READABILITY_SLO: String = format!(include_str!("prompts/slos/system_message_intro.md"), SLO_DESCRIPTION = include_str!("prompts/slos/code_readability_written_com.md"));
    /// System Message for Comments Written SLO
    pub static ref COMMENTS_WRITTEN_SLO: String = format!(include_str!("prompts/slos/system_message_intro.md"), SLO_DESCRIPTION = include_str!("prompts/slos/comments_written_com.md"));
    /// System Message for Error Handling SLO
    pub static ref ERROR_HANDLING_SLO: String = format!(include_str!("prompts/slos/system_message_intro.md"), SLO_DESCRIPTION = include_str!("prompts/slos/error_handling_verification.md"));
    /// System Message for Logic SLO
    pub static ref LOGIC_SLO: String = format!(include_str!("prompts/slos/system_message_intro.md"), SLO_DESCRIPTION = include_str!("prompts/slos/logic_programming.md"));
    /// System Message for Naming Conventions SLO
    pub static ref NAMING_CONVENTIONS_SLO: String = format!(include_str!("prompts/slos/system_message_intro.md"), SLO_DESCRIPTION = include_str!("prompts/slos/naming_written_com.md"));
    /// System Message for Object Oriented Programming SLO
    pub static ref OBJECT_ORIENTED_PROGRAMMING_SLO: String = format!(include_str!("prompts/slos/system_message_intro.md"), SLO_DESCRIPTION = include_str!("prompts/slos/oop_programming.md"));
    /// System Message for Syntax SLO
    pub static ref SYNTAX_SLO: String = format!(include_str!("prompts/slos/system_message_intro.md"), SLO_DESCRIPTION = include_str!("prompts/slos/syntax_programming.md"));
    /// System Message for Testing SLO
    pub static ref TESTING_SLO: String = format!(include_str!("prompts/slos/system_message_intro.md"), SLO_DESCRIPTION = include_str!("prompts/slos/testing_verification.md"));
}

/// Current term. TODO: Move this to init script
pub const TERM: &str = "Fall 2022";

/// Current course. TODO: Move this to init script
pub const COURSE: &str = "ITSC 2214";

/// Prompt truncation length
pub const PROMPT_TRUNCATE: usize = 15000;

/// file name for JUnit platform console standard jar
pub const JUNIT_PLATFORM: &str = "junit-platform-console-standalone-1.9.0-RC1.jar";

/// Tree-sitter query that returns imports made
/// * `path`: java name of the import as it appears in the source code.
/// * `asterisk`: true if the import path ends in an asterisk
pub const IMPORT_QUERY: &str = include_str!("queries/import.scm");

/// Tree-sitter query that returns name of the package
/// * `name`: name of the package
pub const PACKAGE_QUERY: &str = include_str!("queries/package.scm");

/// Tree-sitter query that returns name of the class
/// * `name`: name of the class
pub const CLASSNAME_QUERY: &str = include_str!("queries/class_name.scm");

/// Tree-sitter query that returns name of the interface
/// * `name`: name of the interface
pub const INTERFACENAME_QUERY: &str = include_str!("queries/interface_name.scm");

/// Tree-sitter query that returns name of the JUnit `@Test` annotated methods
/// * `name`: name of the test method
pub const TEST_ANNOTATION_QUERY: &str = include_str!("queries/test_annotation.scm");

/// Tree-sitter query to check the existence of a main method.
pub const MAIN_METHOD_QUERY: &str = include_str!("queries/main_method.scm");

/// Tree-sitter query that returns class declaration statements
/// * `className`: class name
/// * `typeParameters`: type parameters
/// * `interfaces`: interfaces
pub const CLASS_DECLARATION_QUERY: &str = include_str!("queries/class_declaration.scm");

/// * `field`: entire field declaration
pub const CLASS_FIELDS_QUERY: &str = include_str!("queries/class_fields.scm");

/// Tree-sitter query that returns class constructor signatures
/// * `modifier`: constructor modifiers
/// * `annotation`: constructor annotations
/// * `identifier`: constructor identifier
/// * `parameters`: constructor parameters
/// * `throws`: constructor throws
pub const CLASS_CONSTRUCTOR_QUERY: &str = include_str!("queries/class_constructors.scm");

/// Tree-sitter query that returns class method signatures
/// * `modifier`: method modifiers
/// * `annotation`: method annotations
/// * `returnType`: method return type
/// * `identifier`: method identifier
/// * `parameters`: method parameters
/// * `throws`: method throws
pub const CLASS_METHOD_QUERY: &str = include_str!("queries/class_methods.scm");

/// Tree-sitter query that returns interface declaration statements
/// * `identifier`: interface name
/// * `parameters`: type parameters
/// * `extends`: extends interfaces
pub const INTERFACE_DECLARATION_QUERY: &str = include_str!("queries/interface_declaration.scm");

/// Tree-sitter query that returns interface constants
/// * `constant`: entire constant declaration
pub const INTERFACE_CONSTANTS_QUERY: &str = include_str!("queries/interface_constants.scm");

/// Tree-sitter query that returns interface methods signatures
/// * `signature`: entire method signature
pub const INTERFACE_METHODS_QUERY: &str = include_str!("queries/interface_methods.scm");

/// Tree-sitter query that returns method call identifiers
/// * `name`: method call identifier
pub const METHOD_CALL_QUERY: &str = include_str!("queries/method_invocation.scm");

/// Whether to use active retrieval or heuristic based retrieval
pub static USE_ACTIVE_RETRIEVAL: InitCell<bool> = InitCell::new();

    </file-contents>
    <file-contents path="./src/grade.rs" name="grade.rs">
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    collections::HashSet,
    fmt::Display,
    fs,
    io::{BufRead, BufReader, Write},
    ops::RangeInclusive,
    process::Command,
};

use anyhow::{anyhow, ensure, Context, Result};
use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequest,
        CreateChatCompletionResponse,
    },
    Client as OpenAIClient,
};
use colored::Colorize;
use itertools::Itertools;
use rhai::FnPtr;
#[allow(deprecated)]
use rhai::{Array, CustomType, Dynamic, EvalAltResult};
use serde::{Deserialize, Serialize};
use similar::{utils::diff_unicode_words, Algorithm, ChangeTag};
use snailquote::unescape;
use tabled::{
    display::ExpandedDisplay, object::Rows, Alignment, Modify, Panel, TableIteratorExt, Tabled,
    Width,
};
use typed_builder::TypedBuilder;
use umm_derive::generate_rhai_variant;

use crate::{
    constants::{
        ALGORITHMIC_SOLUTIONS_SLO, CODE_READABILITY_SLO, COMMENTS_WRITTEN_SLO, ERROR_HANDLING_SLO,
        LOGIC_SLO, METHOD_CALL_QUERY, NAMING_CONVENTIONS_SLO, OBJECT_ORIENTED_PROGRAMMING_SLO,
        POSTGREST_CLIENT, PROMPT_TRUNCATE, RETRIEVAL_MESSAGE_INTRO, ROOT_DIR, RUNTIME, SCRIPT_AST,
        SOURCE_DIR, SYNTAX_SLO, SYSTEM_MESSAGE, TESTING_SLO, USE_ACTIVE_RETRIEVAL,
    },
    create_engine,
    java::{File, FileType, JavaFileError, Parser, Project},
    parsers::parser,
    util::{classpath, java_path},
    Dict,
};
#[derive(Debug, Hash, PartialEq, Eq)]
/// A struct representing a line in a stack trace
pub struct LineRef {
    /// The line number
    pub line_number: usize,
    /// The file name
    pub file_name:   String,
}

impl LineRef {
    /// Returns the file name
    pub fn file_name(&self) -> &str {
        self.file_name.as_ref()
    }
}

#[derive(Clone, Default)]
/// A struct representing a grade
pub struct Grade {
    /// The actual grade received
    pub grade:  f64,
    /// The maximum grade possible
    pub out_of: f64,
}

impl Grade {
    /// Creates a new grade -
    /// * `grade` - The actual grade received
    /// * `out_of` - The maximum grade possible
    pub fn new(grade: f64,
               out_of: f64)
               -> Self {
        Self { grade, out_of }
    }

    #[generate_rhai_variant(Impl, Fallible)]
    /// Creates a new grade from a string -
    /// * `grade_string` - A string in the format `grade/out_of`, eg. `10/20`
    pub fn grade_from_string(grade_string: String) -> Result<Grade> {
        let (grade, out_of) = grade_string.split_once('/').unwrap_or(("0", "0"));
        Ok(Grade::new(grade.parse::<f64>()
                           .context("Failed to parse grade")?,
                      out_of.parse::<f64>()
                            .context("Failed to parse out of")?))
    }

    /// a getter for the grade
    pub fn grade(&mut self) -> f64 {
        self.grade
    }

    /// a getter for the out_of
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// a setter for the grade
    pub fn set_grade(mut self,
                     grade: f64)
                     -> Self {
        self.grade = grade;
        self
    }

    /// a setter for the out_of
    pub fn set_out_of(mut self,
                      out_of: f64)
                      -> Self {
        self.grade = out_of;
        self
    }
}

impl Display for Grade {
    fn fmt(&self,
           f: &mut std::fmt::Formatter<'_>)
           -> std::fmt::Result {
        write!(f, "{:.2}/{:.2}", self.grade, self.out_of)
    }
}

#[derive(Tabled, Clone, Default)]
/// A struct to store grading results and display them
pub struct GradeResult {
    #[tabled(rename = "Requirement")]
    /// * `requirement`: refers to Requirement ID
    requirement: String,
    #[tabled(rename = "Grade")]
    /// * `grade`: grade received for above Requirement
    grade:       Grade,
    #[tabled(rename = "Reason")]
    /// * `reason`: the reason for penalties applied, if any
    reason:      String,
    #[tabled(skip)]
    /// * `prompt`: the prompt for the AI TA
    prompt:      Option<Vec<ChatCompletionRequestMessage>>,
}

impl GradeResult {
    /// a getter for Requirement
    pub fn requirement(&mut self) -> String {
        self.requirement.clone()
    }

    /// a setter for Requirement
    pub fn set_requirement(mut self,
                           requirement: String)
                           -> Self {
        self.requirement = requirement;
        self
    }

    /// a getter for Reason
    pub fn reason(&mut self) -> String {
        self.reason.clone()
    }

    /// a setter for Reason
    pub fn set_reason(mut self,
                      reason: String)
                      -> Self {
        self.reason = reason;
        self
    }

    /// a getter for the self.grade.grade
    pub fn grade(&mut self) -> f64 {
        self.grade.grade()
    }

    /// a getter for the self.grade.out_of
    pub fn out_of(&mut self) -> f64 {
        self.grade.out_of()
    }

    /// a setter for the self.grade.grade
    pub fn set_grade(mut self,
                     grade: f64)
                     -> Self {
        self.grade = self.grade.set_grade(grade);
        self
    }

    /// a setter for the self.grade.out_of
    pub fn set_out_of(mut self,
                      out_of: f64)
                      -> Self {
        self.grade = self.grade.set_out_of(out_of);
        self
    }

    /// a getter for the prompt
    pub fn prompt(&mut self) -> Option<Vec<ChatCompletionRequestMessage>> {
        self.prompt.clone()
    }

    /// a setter for the prompt
    pub fn set_prompt(mut self,
                      prompt: Option<Vec<ChatCompletionRequestMessage>>)
                      -> Self {
        self.prompt = prompt;
        self
    }
}

#[derive(Tabled, Serialize, Deserialize, TypedBuilder, Clone, Debug)]
#[builder(field_defaults(setter(into)))]
#[builder(doc)]
/// A struct representing a javac diagnostic message
pub struct JavacDiagnostic {
    /// * `path`: path to the file diagnostic is referring to
    #[tabled(rename = "File")]
    path:        String,
    /// * `file_name`: name of the file the diagnostic is about
    #[tabled(skip)]
    file_name:   String,
    /// * `line_number`: line number
    #[tabled(rename = "Line")]
    line_number: u32,
    /// * `is_error`: boolean value, is true if error or false if the diagnostic
    ///   is a warning
    #[tabled(skip)]
    is_error:    bool,
    /// * `message`: the diagnostic message
    #[tabled(rename = "Message")]
    message:     String,
}

impl JavacDiagnostic {
    /// Returns the file name
    pub fn file_name(&self) -> &str {
        self.file_name.as_ref()
    }
}

impl From<JavacDiagnostic> for LineRef {
    /// Converts a JavacDiagnostic to a LineRef
    fn from(val: JavacDiagnostic) -> Self {
        LineRef { file_name:   val.file_name,
                  line_number: val.line_number as usize, }
    }
}

#[derive(Tabled, Serialize, Deserialize, TypedBuilder, Clone)]
#[builder(field_defaults(setter(into)))]
#[builder(doc)]
/// A struct representing a PIT diagnostic message
pub struct MutationDiagnostic {
    /// * `mutator`: name of the mutator in question
    #[tabled(rename = "Mutation type")]
    mutator:          String,
    /// * `source_method`: name of the source method being mutated
    #[tabled(rename = "Source method mutated")]
    source_method:    String,
    /// * `line_number`: source line number where mutation occurred
    #[tabled(rename = "Line no. of mutation")]
    line_number:      u32,
    /// * `test_method`: name of the test examined
    #[tabled(rename = "Test examined")]
    test_method:      String,
    /// * `result`: result of mutation testing
    #[tabled(rename = "Result")]
    result:           String,
    /// * `source_file_name`: name of the source file
    #[tabled(skip)]
    source_file_name: String,
    /// * `test_file_name`: name of the test file
    #[tabled(skip)]
    test_file_name:   String,
}

impl From<MutationDiagnostic> for LineRef {
    /// Converts a MutationDiagnostic to a LineRef
    fn from(val: MutationDiagnostic) -> Self {
        LineRef { file_name:   val.source_file_name,
                  line_number: val.line_number as usize, }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// `RetrievalFunctionCallParams` is a struct that holds the parameters for a
/// retrieval function call.
struct RetrievalFunctionCallParams {
    /// A string that holds the name of the class.
    class_name:  String,
    ///  A string that holds the name of the method.
    method_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
/// `RetrievalFunctionCallParamsArray` is a struct that holds an array of
/// `RetrievalFunctionCallParams`.
struct RetrievalFunctionCallParamsArray {
    /// A vector of `RetrievalFunctionCallParams`.
    params: Vec<RetrievalFunctionCallParams>,
}

/// Retrieves the active context for a retrieval operation.
///
/// This function takes a reference to a `Project` and an optional `String` as
/// additional context. It ensures that the additional context is provided when
/// using active retrieval. It then prepares a series of
/// `ChatCompletionRequestMessage` and serializes them into a JSON string.
///
/// # Arguments
///
/// * `proj` - A reference to a `Project`.
/// * `additional_context` - An optional `String` that provides additional
///   context for the retrieval operation.
///
/// # Returns
///
/// * `Result<ChatCompletionRequestMessage>` - A `Result` that contains a
///   `ChatCompletionRequestMessage` if the operation was successful, or an
///   `Err` if it was not.
pub fn get_active_retrieval_context(proj: &Project,
                                    active_retrieval_context: Option<String>)
                                    -> Result<ChatCompletionRequestMessage> {
    ensure!(active_retrieval_context.is_some(),
            "Additional context must be provided when using active retrieval.");

    print!("Trying to decide what to share with AI for feedback...");

    let messages: Vec<ChatCompletionRequestMessage> = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(RETRIEVAL_MESSAGE_INTRO.to_string())
            .name("Instructor".to_string())
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(format!(
                "Here is the output (stdout and stderr) from running the auto-grader on my \
                 submission:\n```\n{}\n```",
                active_retrieval_context.unwrap()
            ))
            .name("Student".to_string())
            .build()?
            .into(),
        ChatCompletionRequestSystemMessageArgs::default()
            .content(format!(
                include_str!("prompts/retrieval_system_message_outro.md"),
                JAVA_FILE_NAMES = proj.files().iter().map(File::proper_name).join(", "),
                SYNTHESIZED_OUTLINE = proj.describe(),
            ))
            .name("Instructor".to_string())
            .build()?
            .into(),
    ];

    let messages = serde_json::to_string(&messages).expect("Failed to serialize messages array");

    let client = reqwest::blocking::Client::new();
    let response: CreateChatCompletionResponse =
        client.post("https://umm-feedback-openai-func.deno.dev/")
              .body(messages)
              .send()?
              .json()?;
    let response = response.choices[0].message.clone();
    println!(" done!");
    ensure!(response.tool_calls.is_some(),
            "No function call found in response.");
    let function_call_args: RetrievalFunctionCallParamsArray =
        serde_json::from_str(response.tool_calls
                                     .unwrap()
                                     .first()
                                     .unwrap()
                                     .function
                                     .arguments
                                     .as_str())?;

    let mut context = Vec::new();
    for function_call_arg in function_call_args.params {
        let file = proj.identify(&function_call_arg.class_name)?;
        let query = format!(include_str!("queries/method_body_with_name.scm"),
                            &function_call_arg.method_name);

        let res = file.query(&query)
                      .or_else(|_| Ok::<Vec<Dict>, anyhow::Error>(vec![]))
                      .unwrap();

        for r in res {
            let body = r.get("body").unwrap().to_string();
            context.push(format!("Method body from student's submission for `{}#{}`:",
                                 file.proper_name(),
                                 function_call_arg.method_name));
            context.push(format!("\n```\n{}\n```\n", body));
        }
    }

    Ok(ChatCompletionRequestSystemMessageArgs::default().content(context.join("\n"))
                                                        .name("Instructor".to_string())
                                                        .build()?
                                                        .into())
}

/// Returns a ChatCompletionRequestMessage with the given line references that
/// include contextual lines of code from the source
///
/// * `line_refs`: a vector of LineRef objects
/// * `proj`: a Project object
/// * `start_offset`: the number of lines of code to include before the line
/// * `num_lines`: the number of lines of code to include after the line
/// * `max_line_refs`: the maximum number of _processed_ line references to
///   include in the final message
/// * `try_use_active_retrieval`: whether to try to use active retrieval
/// * `additional_context`: additional context to use for
pub fn get_source_context<T: Into<LineRef>>(line_refs: Vec<T>,
                                            proj: Project,
                                            start_offset: usize,
                                            num_lines: usize,
                                            max_line_refs: usize,
                                            try_use_active_retrieval: bool,
                                            active_retrieval_context: Option<String>)
                                            -> Result<ChatCompletionRequestMessage> {
    if try_use_active_retrieval {
        match get_active_retrieval_context(&proj, active_retrieval_context) {
            Ok(message) => return Ok(message),
            Err(e) => {
                eprintln!("Failed to get active retrieval context: {e}");
            }
        }
    }

    let mut line_refs: Vec<(File, LineRef, RangeInclusive<usize>)> =
        line_refs.into_iter()
                 .flat_map(|x| {
                     let x = x.into();
                     let file = proj.identify(&x.file_name)?;
                     let start = match file.kind() {
                         FileType::Test => x.line_number.saturating_sub(num_lines),
                         _ => x.line_number.saturating_sub(start_offset),
                     };
                     let end = start + num_lines;
                     Ok::<(File, LineRef, RangeInclusive<usize>), anyhow::Error>((file,
                                                                                  x,
                                                                                  start..=end))
                 })
                 .collect();

    line_refs.sort_by(|lhs, rhs| {
                 rhs.1
                    .file_name
                    .cmp(&lhs.1.file_name)
                    .then(lhs.1.line_number.cmp(&rhs.1.line_number))
             });
    line_refs.dedup();

    let mut context = Vec::new();
    context.push(
                 "You cannot see all of the student's submission as you are an AI language \
                  model, with limited context length. Here are some snippets of code the \
                  stacktrace indicates might be relevant:
:\n".to_string(),
    );
    let end_ticks = "\n```\n".to_string();
    let mut methods: HashSet<String> = HashSet::new();

    line_refs.into_iter()
             .coalesce(|lhs, rhs| {
                 if lhs.0 == rhs.0 {
                     let lhs_start = *lhs.2.start();
                     let lhs_end = *lhs.2.end();
                     let rhs_start = *rhs.2.start();
                     let rhs_end = *rhs.2.end();
                     let expanded_range =
                         rhs_start.saturating_sub(num_lines)..=(rhs_end + num_lines);

                     if expanded_range.contains(&lhs_start) || expanded_range.contains(&lhs_end) {
                         Ok((lhs.0, lhs.1, lhs_start..=rhs_end))
                     } else {
                         Err((lhs, rhs))
                     }
                 } else {
                     Err((lhs, rhs))
                 }
             })
             .take(max_line_refs)
             .for_each(|(file, f, r)| {
                 let num_lines = r.size_hint().0;
                 let count = file.parser().code().lines().count();

                 let (f, r) = if num_lines as f32 >= 0.6 * (count as f32) {
                     (f, 0..=count)
                 } else {
                     (f, r)
                 };

                 context.push(format!("- Lines {} to {} from {} -\n```",
                                      *r.start(),
                                      *r.end(),
                                      f.file_name));

                 let width = (count as f32).log10().ceil() as usize;

                 let source_code_lines: Vec<String> =
                     file.parser().code().lines().map(String::from).collect();

                 let relevant_source = source_code_lines.clone()
                                                        .iter()
                                                        .skip(*r.start())
                                                        .take(num_lines)
                                                        .enumerate()
                                                        .map(|(line_n, x)| {
                                                            format!("{:width$}|{}",
                                                                    *r.start() + line_n,
                                                                    x).replace("\\\\", "\\")
                                                                      .replace("\\\"", "\"")
                                                        })
                                                        .collect::<Vec<String>>();

                 context.append(&mut (relevant_source.clone()));
                 context.push(end_ticks.clone());

                 match Parser::new(relevant_source.join("\n")) {
                     Ok(parser) => {
                         let method_names: Vec<Dict> =
                             parser.query(METHOD_CALL_QUERY)
                                   .or_else(|_| Ok::<Vec<Dict>, anyhow::Error>(vec![]))
                                   .unwrap();

                         for method in method_names {
                             let method_name = method.get("name").unwrap().to_string();
                             methods.insert(method_name.clone());

                             let query = format!(include_str!("queries/method_body_with_name.scm"),
                                                 &method_name);

                             for f in proj.files() {
                                 if *f.kind() == FileType::Class
                                    || *f.kind() == FileType::ClassWithMain
                                 {
                                     let res = f.query(&query)
                                                .or_else(|_| Ok::<Vec<Dict>, anyhow::Error>(vec![]))
                                                .unwrap();

                                     for r in res {
                                         let body = r.get("body").unwrap().to_string();
                                         let body_lines =
                                             body.lines().map(String::from).collect::<Vec<_>>();
                                         if body_lines.first().is_some() {
                                             let start_line_number =
                                                 source_code_lines.iter()
                                                                  .find_position(|x| {
                                                                      x.contains(body_lines.first()
                                                                                           .unwrap()
                                                                                           .trim())
                                                                  })
                                                                  .unwrap_or((0, &String::new()))
                                                                  .0;

                                             let body = body_lines.iter()
                                                                  .enumerate()
                                                                  .map(|(line_n, x)| {
                                                                      if start_line_number != 0 {
                                                                          format!("{:width$}|{}",
                                                                                  start_line_number
                                                                                  + line_n
                                                                                  + 1,
                                                                                  x)
                                                                      } else {
                                                                          x.to_string()
                                                                      }
                                                                  })
                                                                  .collect::<Vec<String>>()
                                                                  .join("\n");

                                             context.push(format!("Method body from student's \
                                                                   submission `{}#{}`:",
                                                                  f.proper_name(),
                                                                  method_name));
                                             context.push(format!("\n```\n{}\n```\n", body));
                                         }
                                     }
                                 }
                             }
                         }
                     }
                     Err(e) => {
                         eprintln!("Error parsing partial source context: {e}");
                     }
                 };
             });

    let mut context = context.join("\n");
    if context.len() > PROMPT_TRUNCATE {
        context.truncate(PROMPT_TRUNCATE);
        context.push_str("...[TRUNCATED]");
    }

    Ok(ChatCompletionRequestSystemMessageArgs::default().content(context)
                                                        .name("Instructor".to_string())
                                                        .build()?
                                                        .into())
}

#[derive(Clone, Default)]
/// A struct representing arguments to grade_docs function
pub struct DocsGrader {
    /// * `project`: the project to grade
    pub project:  Project,
    /// * `files`: the files to grade
    pub files:    Array,
    /// * `out_of`: the total points for the requirement
    pub out_of:   f64,
    /// * `req_name`: the name of the requirement
    pub req_name: String,
    /// * `penalty`: the penalty to apply for each instance of a violation.
    ///   Optional, default is 3
    pub penalty:  f64,
}

impl DocsGrader {
    /// Getter for project
    pub fn project(&mut self) -> Project {
        self.project.clone()
    }

    /// Setter for project
    pub fn set_project(mut self,
                       project: Project)
                       -> Self {
        self.project = project;
        self
    }

    /// Getter for files
    pub fn files(&mut self) -> Array {
        self.files.clone()
    }

    /// Setter for files
    pub fn set_files(mut self,
                     files: Array)
                     -> Self {
        self.files = files;
        self
    }

    /// Getter for out_of
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// Setter for out_of
    pub fn set_out_of(mut self,
                      out_of: f64)
                      -> Self {
        self.out_of = out_of;
        self
    }

    /// Getter for req_name
    pub fn req_name(&mut self) -> String {
        self.req_name.clone()
    }

    /// Setter for req_name
    pub fn set_req_name(mut self,
                        req_name: String)
                        -> Self {
        self.req_name = req_name;
        self
    }

    /// Getter for penalty
    pub fn penalty(&mut self) -> f64 {
        self.penalty
    }

    /// Setter for penalty
    pub fn set_penalty(mut self,
                       penalty: f64)
                       -> Self {
        self.penalty = penalty;
        self
    }

    /// Grades documentation by using the -Xdoclint javac flag.
    /// Scans javac output for generated warnings and grades accordingly.
    #[generate_rhai_variant(Fallible)]
    pub fn grade_docs(self) -> Result<GradeResult> {
        let mut diags = vec![];
        let mut all_diags = vec![];
        let files: Vec<String> =
            self.files
                .iter()
                .map(|f| match f.clone().into_string() {
                    Ok(n) => Ok(n),
                    Err(e) => Err(anyhow!("files array has something that's not a string: {}", e)),
                })
                .try_collect()?;
        let out_of = self.out_of;
        let mut outputs = vec![];
        for name in &files {
            let file = self.project.identify(name)?;
            let output = match file.doc_check() {
                Ok(o) => o,
                Err(JavaFileError::DuringCompilation { stacktrace, diags }) => {
                    let messages = vec![
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content(SYSTEM_MESSAGE.to_string())
                            .name("Instructor".to_string())
                            .build()?
                            .into(),
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(format!("Compiler error -\n```\n{}\n```", stacktrace))
                            .name("Student".to_string())
                            .build()?
                            .into(),
                        get_source_context(diags, self.project, 1, 3, 6, false, None)?,
                    ];

                    return Ok(GradeResult { requirement: self.req_name,
                                            grade:       Grade::new(0.0, out_of),
                                            reason:      String::from("See above."),
                                            prompt:      Some(messages), });
                }
                Err(e) => {
                    let messages = vec![
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content(SYSTEM_MESSAGE.to_string())
                            .name("Instructor".to_string())
                            .build()?
                            .into(),
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(format!("Unknown error -\n```\n{:?}\n```", e))
                            .name("Student".to_string())
                            .build()?
                            .into(),
                    ];

                    return Ok(GradeResult { requirement: self.req_name,
                                            grade:       Grade::new(0.0, out_of),
                                            reason:      String::from("See above."),
                                            prompt:      Some(messages), });
                }
            };
            outputs.push(output.clone());
            for line in output.lines() {
                let result = parser::parse_diag(line);
                match result {
                    Ok(res) => {
                        if file.file_name() == res.file_name {
                            diags.push(res.clone());
                        }
                        all_diags.push(res);
                    }
                    Err(_) => continue,
                }
            }
        }

        let penalty = diags.len() as f64 * self.penalty;
        let grade = if out_of - penalty > 0.0 {
            out_of - penalty
        } else {
            0.0
        };

        let num_diags = diags.len();
        eprintln!("{}",
                  diags.table()
                       .with(Panel::header(format!("Check javadoc for {}", files.join(", "))))
                       .with(Panel::footer(format!("-{penalty} due to {num_diags} nits")))
                       .with(Modify::new(Rows::new(1..)).with(Width::wrap(24).keep_words()))
                       .with(Modify::new(Rows::first()).with(Alignment::center())
                                                       .with(Alignment::center_vertical()),)
                       .with(Modify::new(Rows::last()).with(Alignment::center())
                                                      .with(Alignment::center_vertical()),)
                       .with(tabled::Style::modern()));

        let prompt = if num_diags > 0 {
            let context = get_source_context(all_diags, self.project, 1, 3, 6, false, None)?;

            let mut outputs = outputs.iter()
                                     .map(|output| format!("```\n{output}\n```"))
                                     .collect::<Vec<String>>()
                                     .join("\n\n---\n\n");

            if outputs.len() > PROMPT_TRUNCATE {
                outputs.truncate(PROMPT_TRUNCATE);
                outputs.push_str("...[TRUNCATED]");
            }

            Some(vec![
                ChatCompletionRequestSystemMessageArgs::default().content(
                    SYSTEM_MESSAGE.to_string(),
                )
                                                                 .name("Instructor".to_string())
                                                                 .build()?
                                                                 .into(),
                ChatCompletionRequestUserMessageArgs::default().content(outputs)
                                                               .name("Student".to_string())
                                                               .build()?
                                                               .into(),
                context,
                ChatCompletionRequestSystemMessageArgs::default().content(include_str!(
                    "prompts/javadoc.md"
                ).to_string())
                                                                 .name("Instructor".to_string())
                                                                 .build()?
                                                                 .into(),
            ])
        } else {
            None
        };
        Ok(GradeResult { requirement: self.req_name,
                         grade: Grade::new(grade, out_of),
                         reason: String::from("See above."),
                         prompt })
    }
}

#[derive(Clone, Default)]
/// Grades by running tests, and reports how many tests pass.
/// Final grade is the same percentage of maximum grade as the number of tests
/// passing.
pub struct ByUnitTestGrader {
    /// A list of test files to run.
    test_files:     Array,
    /// A list of test names that should be found. Grade returned is 0 if any
    /// are not found.
    expected_tests: Array,
    /// A reference to the project the test files belong to.
    project:        Project,
    /// Maximum possible grade.
    out_of:         f64,
    /// Display name for requirement to use while displaying grade result
    req_name:       String,
}

impl ByUnitTestGrader {
    /// Getter for test_files
    pub fn test_files(&mut self) -> Array {
        self.test_files.clone()
    }

    /// Setter for test_files
    pub fn set_test_files(mut self,
                          test_files: Array)
                          -> Self {
        self.test_files = test_files;
        self
    }

    /// Getter for expected_tests
    pub fn expected_tests(&mut self) -> Array {
        self.expected_tests.clone()
    }

    /// Setter for expected_tests
    pub fn set_expected_tests(mut self,
                              expected_tests: Array)
                              -> Self {
        self.expected_tests = expected_tests;
        self
    }

    /// Getter for project
    pub fn project(&mut self) -> Project {
        self.project.clone()
    }

    /// Setter for project
    pub fn set_project(mut self,
                       project: Project)
                       -> Self {
        self.project = project;
        self
    }

    /// Getter for out_of
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// Setter for out_of
    pub fn set_out_of(mut self,
                      out_of: f64)
                      -> Self {
        self.out_of = out_of;
        self
    }

    /// Getter for req_name
    pub fn req_name(&mut self) -> String {
        self.req_name.clone()
    }

    /// Setter for req_name
    pub fn set_req_name(mut self,
                        req_name: String)
                        -> Self {
        self.req_name = req_name;
        self
    }

    #[generate_rhai_variant(Fallible)]
    /// Grades by running tests, and reports how many tests pass.
    /// Final grade is the same percentage of maximum grade as the number of
    /// tests passing.
    pub fn grade_by_tests(self) -> Result<GradeResult> {
        let convert_to_string = |f: Vec<Dynamic>| -> Result<Vec<String>> {
            f.iter()
             .map(|f| match f.clone().into_string() {
                 Ok(n) => Ok(n),
                 Err(e) => Err(anyhow!("test_files array has something that's not a \
                                        string: {}",
                                       e)),
             })
             .try_collect()
        };

        let project = self.project.clone();
        let out_of = self.out_of;
        let req_name = self.req_name;
        let test_files: Vec<String> = convert_to_string(self.test_files)?;
        let expected_tests: Vec<String> = convert_to_string(self.expected_tests)?;

        let mut reasons = {
            let mut reasons = vec![];
            let mut actual_tests = vec![];
            let mut expected_tests = expected_tests;
            expected_tests.sort();

            for test_file in &test_files {
                let test_file = project.identify(test_file)?;

                actual_tests.append(&mut test_file.test_methods());
            }
            actual_tests.sort();

            if !expected_tests.is_empty() {
                for expected in &expected_tests {
                    let n = expected.split_once('#').unwrap().1;
                    if !actual_tests.contains(expected) {
                        reasons.push(format!("- {n} not found."));
                    }
                }

                for actual in &actual_tests {
                    let n = actual.split_once('#').unwrap().1;
                    if !expected_tests.contains(actual) {
                        reasons.push(format!("- Unexpected test called {n}"));
                    }
                }
            }

            reasons
        };

        let new_user_message = |content: String| {
            let mut content = content;
            if content.len() > PROMPT_TRUNCATE {
                content.truncate(PROMPT_TRUNCATE);
                content.push_str("...[TRUNCATED]");
            }

            ChatCompletionRequestUserMessageArgs::default().content(content)
                                                           .name("Student".to_string())
                                                           .build()
                                                           .unwrap()
                                                           .into()
        };
        let new_system_message = |content: String| {
            ChatCompletionRequestSystemMessageArgs::default().content(content)
                                                             .name("Instructor".to_string())
                                                             .build()
                                                             .unwrap()
                                                             .into()
        };

        let process_junit_stacktrace = |stacktrace: String| {
            let mut updated_stacktrace = Vec::new();
            let mut all_diags = Vec::new();

            for line in stacktrace.lines() {
                if line.contains("MethodSource") || line.contains("Native Method") {
                    continue;
                }

                if line.contains("Test run finished after") {
                    break;
                }

                if let Ok(diag) = parser::junit_stacktrace_line_ref(line) {
                    if project.identify(&diag.file_name).is_ok() {
                        updated_stacktrace.push(line.replace("\\\\", "\\")
                                                    .replace("\\\"", "\"")
                                                    .to_string());
                    }
                    all_diags.push(diag);
                } else {
                    updated_stacktrace.push(line.replace("\\\\", "\\")
                                                .replace("\\\"", "\"")
                                                .to_string());
                }
            }

            (updated_stacktrace, all_diags)
        };

        let initial_message = new_system_message(SYSTEM_MESSAGE.to_string());

        if !reasons.is_empty() {
            reasons.push("Tests will not be run until above is fixed.".into());
            let reasons = reasons.join("\n");
            let messages = vec![initial_message, new_user_message(reasons.clone())];
            Ok(GradeResult { requirement: req_name,
                             grade:       Grade::new(0.0, out_of),
                             reason:      reasons,
                             prompt:      Some(messages), })
        } else {
            let mut num_tests_passed = 0.0;
            let mut num_tests_total = 0.0;
            let mut messages = vec![initial_message.clone()];

            for test_file in test_files {
                let res = match project.identify(test_file.as_str())?
                                       .test(Vec::new(), Some(&project))
                {
                    Ok(res) => res,
                    Err(JavaFileError::FailedTests { test_results,
                                                     diags, }) => {
                        let (updated_stacktrace, _) =
                            process_junit_stacktrace(test_results.clone());

                        messages.extend(vec![
                            new_user_message(format!(
                                "Failed tests -\n```\n{}\n```",
                                updated_stacktrace.join("\n")
                            )),
                            get_source_context(
                                diags,
                                project.clone(),
                                3,
                                6,
                                6,
                                *USE_ACTIVE_RETRIEVAL.try_get().unwrap_or(&false),
                                Some(updated_stacktrace.join("\n")),
                            )?,
                        ]);

                        test_results
                    }
                    Err(JavaFileError::Unknown(e)) => {
                        let out = format!("Unknown error -\n```\n{:#?}\n```", e);
                        messages.push(new_user_message(out.clone()));
                        out
                    }
                    Err(JavaFileError::DuringCompilation { stacktrace, diags }) => {
                        let out = format!("Compiler error -\n```\n{}\n```", stacktrace);
                        messages.extend(vec![new_user_message(out.clone()),
                                             get_source_context(diags,
                                                                project.clone(),
                                                                3,
                                                                6,
                                                                6,
                                                                false,
                                                                None)?,]);
                        out
                    }
                    Err(JavaFileError::AtRuntime { output, diags }) => {
                        let out = format!("Error at runtime -\n```\n{}\n```", output);
                        messages.extend(vec![new_user_message(out.clone()),
                                             get_source_context(diags,
                                                                project.clone(),
                                                                3,
                                                                6,
                                                                6,
                                                                false,
                                                                None)?,]);
                        out
                    }
                };
                let mut current_tests_passed = 0.0;
                let mut current_tests_total = 0.0;

                for line in res.lines() {
                    let parse_result =
                        parser::num_tests_passed(line).context("While parsing Junit summary table");
                    if let Ok(n) = parse_result {
                        current_tests_passed = n as f64;
                    }
                    let parse_result =
                        parser::num_tests_found(line).context("While parsing Junit summary table");
                    if let Ok(n) = parse_result {
                        current_tests_total = n as f64;
                    }
                }

                num_tests_passed += current_tests_passed;
                num_tests_total += current_tests_total;
            }
            let grade = if num_tests_total != 0.0 {
                (num_tests_passed / num_tests_total) * out_of
            } else {
                0.0
            };

            Ok(GradeResult { requirement: req_name,
                             grade:       Grade::new(grade, out_of),
                             reason:      format!("- {num_tests_passed}/{num_tests_total} tests \
                                                   passing."),
                             prompt:      Some(messages), })
        }
    }
}

#[derive(Clone, Default)]
/// Runs mutation tests using ![Pitest](http://pitest.org/) to grade unit tests written by students.
pub struct UnitTestGrader {
    /// Name of the requirement.
    pub req_name:         String,
    /// Maximum possible grade.
    pub out_of:           f64,
    /// List of test classes to run.
    pub target_test:      Array,
    /// List of classes to mutate.
    pub target_class:     Array,
    /// List of methods to exclude from mutation.
    pub excluded_methods: Array,
    /// List of classes to avoid mutating.
    pub avoid_calls_to:   Array,
}

impl UnitTestGrader {
    /// A getter for the name of the requirement.
    pub fn get_req_name(&mut self) -> String {
        self.req_name.clone()
    }

    /// A getter for the maximum possible grade.
    pub fn get_out_of(&mut self) -> f64 {
        self.out_of
    }

    /// A getter for the list of test classes to run.
    pub fn get_target_test(&mut self) -> Array {
        self.target_test.clone()
    }

    /// A getter for the list of classes to mutate.
    pub fn get_target_class(&mut self) -> Array {
        self.target_class.clone()
    }

    /// A getter for the list of methods to exclude from mutation.
    pub fn get_excluded_methods(&mut self) -> Array {
        self.excluded_methods.clone()
    }

    /// A getter for the list of classes to avoid mutating.
    pub fn get_avoid_calls_to(&mut self) -> Array {
        self.avoid_calls_to.clone()
    }

    /// A setter for the name of the requirement.
    pub fn set_req_name(mut self,
                        req_name: String)
                        -> Self {
        self.req_name = req_name;
        self
    }

    /// A setter for the maximum possible grade.
    pub fn set_out_of(mut self,
                      out_of: f64)
                      -> Self {
        self.out_of = out_of;
        self
    }

    /// A setter for the list of test classes to run.
    pub fn set_target_test(mut self,
                           target_test: Array)
                           -> Self {
        self.target_test = target_test;
        self
    }

    /// A setter for the list of classes to mutate.
    pub fn set_target_class(mut self,
                            target_class: Array)
                            -> Self {
        self.target_class = target_class;
        self
    }

    /// A setter for the list of methods to exclude from mutation.
    pub fn set_excluded_methods(mut self,
                                excluded_methods: Array)
                                -> Self {
        self.excluded_methods = excluded_methods;
        self
    }

    /// A setter for the list of classes to avoid mutating.
    pub fn set_avoid_calls_to(mut self,
                              avoid_calls_to: Array)
                              -> Self {
        self.avoid_calls_to = avoid_calls_to;
        self
    }

    #[generate_rhai_variant(Fallible)]
    /// Runs mutation tests using ![Pitest](http://pitest.org/) to grade unit tests written by students.
    pub fn grade_unit_tests(&mut self) -> Result<GradeResult> {
        let req_name = self.get_req_name();
        let out_of = self.get_out_of();
        let target_test = self.get_target_test();
        let target_class = self.get_target_class();
        let excluded_methods = self.get_excluded_methods();
        let avoid_calls_to = self.get_avoid_calls_to();
        let project = Project::new()?;

        eprintln!("Running Mutation tests -");
        let target_test: Vec<String> =
            target_test.iter()
                       .map(|f| match f.clone().into_string() {
                           Ok(n) => Ok(n),
                           Err(e) => Err(anyhow!("target_test array has something that's not a \
                                                  string: {}",
                                                 e)),
                       })
                       .try_collect()?;
        let target_class: Vec<String> =
            target_class.iter()
                        .map(|f| match f.clone().into_string() {
                            Ok(n) => Ok(n),
                            Err(e) => Err(anyhow!("target_class array has something that's not \
                                                   a string: {}",
                                                  e)),
                        })
                        .try_collect()?;
        let excluded_methods: Vec<String> = excluded_methods.iter()
                                                            .map(|f| match f.clone().into_string() {
                                                                Ok(n) => Ok(n),
                                                                Err(e) => Err(anyhow!(
                    "excluded_methods array has something that's not a string: {}",
                    e
                )),
                                                            })
                                                            .try_collect()?;
        let avoid_calls_to: Vec<String> =
            avoid_calls_to.iter()
                          .map(|f| match f.clone().into_string() {
                              Ok(n) => Ok(n),
                              Err(e) => Err(anyhow!("avoid_calls_to array has something that's \
                                                     not a string: {}",
                                                    e)),
                          })
                          .try_collect()?;

        let child = Command::new(java_path()?).args(["--class-path",
                                                     classpath()?.as_str(),
                                                     "org.pitest.mutationtest.commandline.\
                                                      MutationCoverageReport",
                                                     "--reportDir",
                                                     "test_reports",
                                                     "--failWhenNoMutations",
                                                     "true",
                                                     "--threads",
                                                     "6",
                                                     "--targetClasses",
                                                     target_class.join(",").as_str(),
                                                     "--targetTests",
                                                     target_test.join(",").as_str(),
                                                     "--sourceDirs",
                                                     [SOURCE_DIR.to_str().unwrap_or("."),
                                                      ROOT_DIR.to_str().unwrap_or(".")].join(",")
                                                                                       .as_str(),
                                                     "--timestampedReports",
                                                     "false",
                                                     "--outputFormats",
                                                     "HTML,CSV",
                                                     "--mutators",
                                                     "STRONGER",
                                                     "--excludedMethods",
                                                     excluded_methods.join(",").as_str(),
                                                     "--avoidCallsTo",
                                                     avoid_calls_to.join(",").as_str()])
                                              .output()
                                              .context("Failed to spawn javac process.")?;

        if child.status.success() {
            fs::create_dir_all("test_reports")?;
            let file = fs::File::open(ROOT_DIR.join("test_reports").join("mutations.csv"))
                .context("Could not read ./test_reports/mutations.csv file".to_string())?;
            let reader = BufReader::new(file);
            let mut diags = vec![];

            for line in reader.lines() {
                let line = line?;
                let parse_result = parser::mutation_report_row(&line).context("While parsing \
                                                                               test_reports/\
                                                                               mutations.csv");

                match parse_result {
                    Ok(r) => {
                        if r.result == "SURVIVED" {
                            diags.push(r);
                        }
                    }
                    Err(e) => {
                        anyhow::bail!(e);
                    }
                };
            }
            let penalty = diags.len() as u32 * 4;
            eprintln!("Ran mutation tests for {} -", target_test.join(", "));
            let num_diags = diags.len();
            eprintln!("Problematic mutation test failures printed above.");

            let prompt = if num_diags > 0 {
                let context = get_source_context(diags.clone(), project, 3, 6, 6, false, None)?;

                let mut feedback = ExpandedDisplay::new(diags).to_string();
                eprintln!("{feedback}");

                if feedback.len() > PROMPT_TRUNCATE {
                    feedback.truncate(PROMPT_TRUNCATE);
                    feedback.push_str("...[TRUNCATED]");
                }

                Some(vec![
                    ChatCompletionRequestSystemMessageArgs::default().content(
                        SYSTEM_MESSAGE.to_string(),
                    )
                                                                     .name("Instructor".to_string())
                                                                     .build()
                                                                     .context(
                        "Failed to build system message",
                    )?
                                                                     .into(),
                    ChatCompletionRequestUserMessageArgs::default().content(feedback)
                                                                   .name("Student".to_string())
                                                                   .build()
                                                                   .context(
                        "Failed to build user message",
                    )?
                                                                   .into(),
                    context,
                    ChatCompletionRequestSystemMessageArgs::default().content(format!(
                        include_str!("prompts/mutation_testing.md"),
                        test = target_test.join(", "),
                        class = target_class.join(", ")
                    ))
                                                                     .name("Instructor".to_string())
                                                                     .build()
                                                                     .context(
                        "Failed to build system message",
                    )?
                                                                     .into(),
                ])
            } else {
                None
            };

            Ok(GradeResult { requirement: req_name,
                             grade: Grade::new((out_of as u32).saturating_sub(penalty).into(),
                                               out_of),
                             reason: format!("-{penalty} Penalty due to surviving mutations"),
                             prompt })
        } else {
            let mut output = [String::from_utf8(child.stderr)?,
                              String::from_utf8(child.stdout)?].concat();
            eprintln!("{output}");
            if output.len() > PROMPT_TRUNCATE {
                output.truncate(PROMPT_TRUNCATE);
                output.push_str("...[TRUNCATED]");
            }

            let prompt = if !output.is_empty() {
                Some(vec![
                    ChatCompletionRequestSystemMessageArgs::default().content(
                        SYSTEM_MESSAGE.to_string(),
                    )
                                                                     .name("Instructor".to_string())
                                                                     .build()
                                                                     .context(
                        "Failed to build system message",
                    )?
                                                                     .into(),
                    ChatCompletionRequestUserMessageArgs::default().content(output)
                                                                   .name("Student".to_string())
                                                                   .build()
                                                                   .context(
                        "Failed to build user message",
                    )?
                                                                   .into(),
                    ChatCompletionRequestSystemMessageArgs::default().content(format!(
                        include_str!("prompts/mutation_testing_2.md"),
                        test = target_test.join(", "),
                        class = target_class.join(", ")
                    ))
                                                                     .name("Instructor".to_string())
                                                                     .build()
                                                                     .context(
                        "Failed to build system message",
                    )?
                                                                     .into(),
                ])
            } else {
                None
            };
            Ok(GradeResult { requirement: req_name,
                             grade: Grade::new(0.0, out_of),
                             reason: String::from("Something went wrong while running \
                                                   mutation tests, skipping."),
                             prompt })
        }
    }
}

#[derive(Clone, Default)]
/// Grades using hidden tests. Test file is downloaded, ran, and then cleaned up
/// before returning.
pub struct ByHiddenTestGrader {
    /// URL to download test source from.
    pub url:             String,
    /// name of hidden test class.
    pub test_class_name: String,
    /// points to give if all tests pass.
    pub out_of:          f64,
    /// name of requirement.
    pub req_name:        String,
}

impl ByHiddenTestGrader {
    /// gets the `url` field.
    pub fn url(&mut self) -> String {
        self.url.clone()
    }

    /// sets the `url` field.
    pub fn set_url(mut self,
                   url: String)
                   -> Self {
        self.url = url;
        self
    }

    /// gets the `test_class_name` field
    pub fn test_class_name(&mut self) -> String {
        self.test_class_name.clone()
    }

    /// sets the `test_class_name` field
    pub fn set_test_class_name(mut self,
                               test_class_name: String)
                               -> Self {
        self.test_class_name = test_class_name;
        self
    }

    /// gets the `out_of` field
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// sets the `out_of` field
    pub fn set_out_of(mut self,
                      out_of: f64)
                      -> Self {
        self.out_of = out_of;
        self
    }

    /// gets the `req_name` field
    pub fn req_name(&mut self) -> String {
        self.req_name.clone()
    }

    /// sets the `req_name` field
    pub fn set_req_name(mut self,
                        req_name: String)
                        -> Self {
        self.req_name = req_name;
        self
    }

    #[generate_rhai_variant(Fallible)]
    /// Grades using hidden tests. Test file is downloaded, ran, and then
    /// cleaned up before returning.
    pub fn grade_by_hidden_tests(&mut self) -> Result<GradeResult> {
        let url = self.url();
        let test_class_name = self.test_class_name();
        let out_of = self.out_of();
        let req_name = self.req_name();

        let test_source =
            reqwest::blocking::get(&url).context(format!("Failed to download {url}"))?
                                        .bytes()
                                        .context(format!("Failed to get response as bytes: \
                                                          {url}"))?;

        let path = ROOT_DIR.join(format!("{test_class_name}.java"));
        let mut file = fs::File::create(&path)?;
        file.write_all(&test_source)?;

        let project = match Project::new() {
            Ok(a) => a,
            Err(e) => {
                fs::remove_file(&path)?;
                return Err(e);
            }
        };

        let grader = ByUnitTestGrader { test_files: vec![Dynamic::from(test_class_name)],
                                        expected_tests: Array::new(),
                                        project,
                                        out_of,
                                        req_name };

        let out = match grader.grade_by_tests() {
            Ok(o) => o,
            Err(e) => {
                fs::remove_file(&path)?;
                return Err(e);
            }
        };

        fs::remove_file(&path)?;
        Ok(out)
    }
}

/// Represents output format settings for Gradescope submissions.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum GradescopeOutputFormat {
    /// Plain text format.
    Text,
    /// HTML format.
    Html,
    /// This is very similar to the "html" format option but will also convert
    /// \n into <br /> and \n\n+ into a page break.
    SimpleFormat,
    /// Markdown format.
    Md,
    /// ANSI format for including ANSI escape codes (often used in terminal
    /// outputs).
    Ansi,
}

/// Represents visibility settings for Gradescope submissions and test cases.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum GradescopeVisibility {
    /// Hidden from students.
    Hidden,
    /// Visible after the due date of the assignment.
    AfterDueDate,
    /// Visible after the grades are published.
    AfterPublished,
    /// Always visible to students.
    Visible,
}

/// Represents the status of a test case in Gradescope submissions.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum GradescopeStatus {
    /// Indicates the test case passed successfully.
    Passed,
    /// Indicates the test case failed.
    Failed,
}

/// Represents the overall submission data.
#[derive(Serialize, Deserialize, Debug, TypedBuilder)]
#[builder(field_defaults(default, setter(into)))]
#[builder(doc)]
pub struct GradescopeSubmission {
    /// Optional overall score. Overrides total of test cases if specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,

    /// Optional execution time in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time: Option<u32>,

    /// Optional text relevant to the entire submission.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,

    /// Optional output format settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<GradescopeOutputFormat>,

    /// Optional default output format for test case outputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_output_format: Option<GradescopeOutputFormat>,

    /// Optional default output format for test case names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_name_format: Option<GradescopeOutputFormat>,

    /// Optional visibility setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<GradescopeVisibility>,

    /// Optional stdout visibility setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_visibility: Option<GradescopeVisibility>,

    /// Optional extra data to be stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_data: Option<serde_json::Value>,

    /// Optional test cases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests: Option<Vec<GradescopeTestCase>>,

    /// Optional leaderboard setup.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leaderboard: Option<Vec<GradescopeLeaderboardEntry>>,
}

/// Represents an individual test case.
#[derive(Serialize, Deserialize, Debug, TypedBuilder)]
#[builder(field_defaults(default, setter(into)))]
#[builder(doc)]
pub struct GradescopeTestCase {
    /// Optional score for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,

    /// Optional maximum score for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_score: Option<f64>,

    /// Optional status of the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<GradescopeStatus>,

    /// Optional name of the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional formatting for the test case name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_format: Option<GradescopeOutputFormat>,

    /// Optional number for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,

    /// Optional detailed output for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,

    /// Optional formatting for the test case output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<GradescopeOutputFormat>,

    /// Optional tags associated with the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    /// Optional visibility setting for the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<GradescopeVisibility>,

    /// Optional extra data to be stored with the test case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_data: Option<serde_json::Value>,
}

/// Represents an entry in the leaderboard.
#[derive(Serialize, Deserialize, Debug, TypedBuilder)]
#[builder(field_defaults(default, setter(into)))]
#[builder(doc)]
pub struct GradescopeLeaderboardEntry {
    /// Name of the leaderboard metric.
    pub name: String,

    /// Value of the leaderboard metric.
    pub value: String,

    /// Optional ordering for the leaderboard metric.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
}

/// What kind of file the SLO is for.
#[derive(Debug)]
enum SLOFileType {
    /// Only source files.
    Source,
    /// Only test files.
    Test,
    /// Both source and test files.
    SourceAndTest,
}

async fn generate_combined_slo_report(slo_responses: Vec<(&str,
                                           Result<CreateChatCompletionResponse,
                                                  OpenAIError>)>)
                                      -> Result<String> {
    let mut individual_feedbacks = Vec::new();

    for (name, resp) in slo_responses {
        match resp {
            Ok(response) => {
                let content = response.choices
                                      .first()
                                      .and_then(|choice| choice.message.content.clone())
                                      .unwrap_or_default();

                individual_feedbacks.push(format!("SLO: {}\n\n{}", name, content));
            }
            Err(e) => {
                // Log the error or handle it as appropriate for your use case
                eprintln!("Error processing SLO '{}': {:?}", name, e);
                individual_feedbacks.push(format!("SLO: {}\n\nError: Unable to process this SLO.",
                                                  name));
            }
        }
    }

    let combined_feedback = individual_feedbacks.join("\n\n---\n\n");

    let openai_client = OpenAIClient::with_config(
        OpenAIConfig::new()
            .with_api_base(
                std::env::var("OPENAI_ENDPOINT")
                    .context("OPENAI_ENDPOINT must be set for SLO feedback")?,
            )
            .with_api_key(
                std::env::var("OPENAI_API_KEY_SLO")
                    .context("OPENAI_API_KEY_SLO must be set for SLO feedback")?,
            ),
    );

    let messages =
        vec![
             ChatCompletionRequestSystemMessageArgs::default().content(
            "You are an AI assistant tasked with creating a concise, well-structured report that \
             combines feedback from multiple Student Learning Outcomes (SLOs). Your goal is to \
             provide a comprehensive overview of the student's performance across all SLOs, \
             highlighting strengths, areas for improvement, and specific recommendations.
                 
                 The report should also serve as an effective code review, offering actionable \
             insights that the student can use to enhance their code quality and programming \
             skills.
                 
                 The student is the intended audience for this report, so talk directly to them in \
             a friendly, constructive manner. Use clear, concise language and provide specific \
             code examples to support your feedback. Especially when making recommendations, \
             explain the reasoning behind your suggestions and offer guidance on how the student \
             can implement them effectively.",
        )
                                                              .name("Instructor")
                                                              .build()?
                                                              .into(),
             ChatCompletionRequestUserMessageArgs::default().content(format!(
            "Please create a combined report based on the following individual SLO \
             feedbacks:\n\n{}",
            combined_feedback
        ))
                                                            .name("Student")
                                                            .build()?
                                                            .into(),
        ];

    let response = openai_client
        .chat()
        .create(CreateChatCompletionRequest {
            model: std::env::var("OPENAI_MODEL")
                .context("OPENAI_MODEL must be set for SLO feedback")?,
            messages,
            temperature: Some(0.0),
            top_p: Some(1.0),
            n: Some(1),
            stream: Some(false),
            ..Default::default()
        })
        .await?;

    response.choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No content in OpenAI response"))
}

/// Generates SLO responses for a given project.
///
/// # Arguments
///
/// * `project` - The project for which to generate SLO responses.
/// * `source_files` - A list of source files in the project.
/// * `test_files` - A list of test files in the project.
/// * `project_title` - The title of the project.
/// * `project_description` - A description of the project.
///
/// # Returns
///
/// A vector of tuples containing the SLO name and the result of the SLO
/// response.
async fn generate_slo_responses(
    project: &Project,
    source_files: &[String],
    test_files: &[String],
    project_title: &str,
    project_description: &str,
    enabled_slos: &HashSet<String> /* New parameter */)
    -> Result<Vec<(&'static str, Result<CreateChatCompletionResponse, OpenAIError>)>> {
    let slos = vec![("slo_algorithmic_solutions",
                     "Algorithmic Solutions",
                     ALGORITHMIC_SOLUTIONS_SLO.as_str(),
                     SLOFileType::Source),
                    ("slo_code_readability",
                     "Code Readability and Formatting",
                     CODE_READABILITY_SLO.as_str(),
                     SLOFileType::SourceAndTest),
                    ("slo_comments",
                     "Comments",
                     COMMENTS_WRITTEN_SLO.as_str(),
                     SLOFileType::SourceAndTest),
                    ("slo_error_handling",
                     "Error Handling",
                     ERROR_HANDLING_SLO.as_str(),
                     SLOFileType::SourceAndTest),
                    ("slo_logic", "Logic", LOGIC_SLO.as_str(), SLOFileType::SourceAndTest),
                    ("slo_naming_conventions",
                     "Naming Conventions",
                     NAMING_CONVENTIONS_SLO.as_str(),
                     SLOFileType::SourceAndTest),
                    ("slo_oop_programming",
                     "Object Oriented Programming",
                     OBJECT_ORIENTED_PROGRAMMING_SLO.as_str(),
                     SLOFileType::SourceAndTest),
                    ("slo_syntax", "Syntax", SYNTAX_SLO.as_str(), SLOFileType::SourceAndTest),
                    ("slo_testing", "Testing", TESTING_SLO.as_str(), SLOFileType::Test),];

    let mut slo_requests = Vec::new();

    for (slo_key, slo_name, slo_system_message, slo_file_type) in slos {
        if !enabled_slos.contains(slo_key) {
            continue;
        }

        let relevant_files: Vec<File> = match slo_file_type {
            SLOFileType::Source => source_files.iter()
                                               .filter_map(|x| project.identify(x).ok())
                                               .collect(),
            SLOFileType::Test => test_files.iter()
                                           .filter_map(|x| project.identify(x).ok())
                                           .collect(),
            SLOFileType::SourceAndTest => source_files.iter()
                                                      .chain(test_files.iter())
                                                      .filter_map(|x| project.identify(x).ok())
                                                      .collect(),
        };

        let relevant_file_codes: Vec<String> =
            relevant_files.iter().map(|x| x.parser().code()).collect();

        ensure!(!relevant_file_codes.is_empty(),
                "No relevant files ({:?}) with source code found for SLO {}",
                slo_file_type,
                slo_name);

        let mut student_message =
            vec![format!("# Submission for {project_title}\n\nDescription: {project_description}")];

        for (file, code) in relevant_files.iter().zip(relevant_file_codes.iter()) {
            student_message.push(format!("\n\n## Contents of {file_name}\n\n```java\n{code}\n```",
                                         file_name = file.proper_name(),
                                         code = code));
        }

        let student_message = student_message.join("\n\n");
        let messages = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(slo_system_message.to_string())
                .name("Instructor".to_string())
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(student_message)
                .name("Student".to_string())
                .build()?
                .into(),
        ];

        slo_requests.push(async move {
                        let openai_client = OpenAIClient::with_config(
                OpenAIConfig::new()
                    .with_api_base(
                        std::env::var("OPENAI_ENDPOINT")
                            .expect("OPENAI_ENDPOINT must be set for SLO feedback"),
                    )
                    .with_api_key(
                        std::env::var("OPENAI_API_KEY_SLO")
                            .expect("OPENAI_API_KEY_SLO must be set for SLO feedback"),
                    ),
            );

                        let response = openai_client
                .chat()
                .create(CreateChatCompletionRequest {
                    model: std::env::var("OPENAI_MODEL")
                        .expect("OPENAI_MODEL must be set for SLO feedback"),
                    messages: messages.clone(),
                    temperature: Some(0.0),
                    top_p: Some(1.0),
                    n: Some(1),
                    stream: Some(false),
                    ..Default::default()
                })
                .await;

                        (slo_name, response)
                    });
    }

    let slo_responses = futures::future::join_all(slo_requests).await;
    Ok(slo_responses)
}

#[generate_rhai_variant(Fallible)]
/// Print grade result
///
/// * `results`: array of GradeResults to print in a table.
/// * `gradescope_config`: map of gradescope configuration options, which can
///   contain:
///     - `source_files`: array of source files to provide feedback on in the
///       submission. Defaults to empty array.
///     - `test_files`: array of test files to provide feedback on in the
///       submission. Defaults to empty array.
///     - `project_title`: title of the project. Defaults to empty string.
///     - `project_description`: description of the project. Defaults to empty
///       string.
///     - `pass_threshold`: threshold for passing the project. Defaults to 0.7.
///     - `show_table`: whether to show the grading table. Defaults to true.
///     - `results_json`: whether to write the gradescope results in JSON
///       format. Defaults to false.
///     - `feedback`: whether to provide feedback on penalties to students.
///       Defaults to false.
///     - `leaderboard`: whether to produce leaderboard entries. Also produces
///       relevant SLO feedback. Defaults to false.
///     - `debug`: whether to write gradescope JSON within the current
///       directory. Defaults to false.
///     - `slo_algorithmic_solutions`: whether to provide feedback on
///       Algorithmic Solutions SLO. Defaults to false.
///     - `slo_code_readability`: whether to provide feedback on Code
///       Readability and Formatting SLO. Defaults to false.
///     - `slo_comments`: whether to provide feedback on Comments SLO. Defaults
///       to false.
///     - `slo_error_handling`: whether to provide feedback on Error Handling
///       SLO. Defaults to false.
///     - `slo_logic`: whether to provide feedback on Logic SLO. Defaults to
///       false.
///     - `slo_naming_conventions`: whether to provide feedback on Naming
///       Conventions SLO. Defaults to false.
///     - `slo_oop_programming`: whether to provide feedback on Object Oriented
///       Programming SLO. Defaults to false.
///     - `slo_syntax`: whether to provide feedback on Syntax SLO. Defaults to
///       false.
///     - `slo_testing`: whether to provide feedback on Testing SLO. Defaults to
///       false.
pub fn show_result(results: Array,
                   gradescope_config: rhai::Map)
                   -> Result<()> {
    let results: Vec<GradeResult> = results.iter()
                                           .map(|f| f.clone().cast::<GradeResult>())
                                           .collect();
    let source_files = gradescope_config.get("source_files")
                                        .unwrap_or(&Dynamic::from(Array::new()))
                                        .clone()
                                        .cast::<Array>()
                                        .iter()
                                        .map(|f| f.clone().cast::<String>())
                                        .collect::<Vec<String>>();

    let test_files = gradescope_config.get("test_files")
                                      .unwrap_or(&Dynamic::from(Array::new()))
                                      .clone()
                                      .cast::<Array>()
                                      .iter()
                                      .map(|f| f.clone().cast::<String>())
                                      .collect::<Vec<String>>();

    let project_title = gradescope_config.get("project_title")
                                         .unwrap_or(&Dynamic::from(String::new()))
                                         .clone()
                                         .cast::<String>();
    let project_description = gradescope_config.get("project_description")
                                               .unwrap_or(&Dynamic::from(String::new()))
                                               .clone()
                                               .cast::<String>();
    let pass_threshold = gradescope_config.get("pass_threshold")
                                          .unwrap_or(&Dynamic::from(0.7))
                                          .clone()
                                          .cast::<f64>();

    let get_or_default = |f: &str, d: bool| -> bool {
        gradescope_config.get(f)
                         .unwrap_or(&Dynamic::from(d))
                         .clone()
                         .cast::<bool>()
    };
    let show_table = get_or_default("show_table", true);
    let gradescope_json = get_or_default("results_json", false);
    let gradescope_feedback = get_or_default("feedback", false);
    let gradescope_debug = get_or_default("debug", false);

    let enabled_slos: HashSet<String> =
        vec!["slo_algorithmic_solutions",
             "slo_code_readability",
             "slo_comments",
             "slo_error_handling",
             "slo_logic",
             "slo_naming_conventions",
             "slo_oop_programming",
             "slo_syntax",
             "slo_testing",].into_iter()
                            .filter(|&slo| get_or_default(slo, false))
                            .map(String::from)
                            .collect();

    let (grade, out_of) = results.iter().fold((0f64, 0f64), |acc, r| {
                                            (acc.0 + r.grade.grade, acc.1 + r.grade.out_of)
                                        });

    if show_table {
        eprintln!("{}",
                  results.clone()
                         .table()
                         .with(Panel::header("Grading Overview"))
                         .with(Panel::footer(format!("Total: {grade:.2}/{out_of:.2}")))
                         .with(Modify::new(Rows::new(1..)).with(Width::wrap(24).keep_words()))
                         .with(Modify::new(Rows::first()).with(Alignment::center())
                                                         .with(Alignment::center_vertical()),)
                         .with(Modify::new(Rows::last()).with(Alignment::center())
                                                        .with(Alignment::center_vertical()),)
                         .with(tabled::Style::modern()));
    }

    if gradescope_json {
        let project = Project::new()?;
        let mut test_cases = vec![];
        for result in results {
            let mut result = result.clone();

            let feedback = if gradescope_feedback {
                generate_single_feedback(&result)?
            } else {
                String::new()
            };

            let test_case = GradescopeTestCase::builder().name(result.requirement())
                                                         .name_format(GradescopeOutputFormat::Text)
                                                         .max_score(result.out_of())
                                                         .score(result.grade())
                                                         .status(if result.grade()
                                                                    > pass_threshold
                                                                      * result.out_of()
                                                                 {
                                                                     GradescopeStatus::Passed
                                                                 } else {
                                                                     GradescopeStatus::Failed
                                                                 })
                                                         .output(feedback)
                                                         .output_format(GradescopeOutputFormat::Md)
                                                         .build();

            test_cases.push(test_case);
        }

        if grade > pass_threshold * out_of && !enabled_slos.is_empty() {
            let runtime = RUNTIME.handle().clone();

            ensure!(!project_title.is_empty(),
                    "Project title must be specified to generate SLO feedback");
            ensure!(!project_description.is_empty(),
                    "Project description must be specified to generate SLO feedback");

            let slo_responses = runtime.block_on(async {
                                           generate_slo_responses(&project,
                                                                  &source_files,
                                                                  &test_files,
                                                                  &project_title,
                                                                  &project_description,
                                                                  &enabled_slos).await
                                       })?;

            let combined_report =
                runtime.block_on(async { generate_combined_slo_report(slo_responses).await })?;

            test_cases.push(GradescopeTestCase::builder().name("Student Learning Outcomes \
                                                                (SLOs) Feedback"
                                                                                .to_string())
                                                         .name_format(GradescopeOutputFormat::Text)
                                                         .output(combined_report)
                                                         .output_format(GradescopeOutputFormat::Md)
                                                         .max_score(0f64)
                                                         .score(0f64)
                                                         .build());
        }
        let submission =
            GradescopeSubmission::builder().tests(Some(test_cases))
                                           .test_output_format(GradescopeOutputFormat::Md)
                                           .test_name_format(GradescopeOutputFormat::Text)
                                           .stdout_visibility(GradescopeVisibility::Visible)
                                           .visibility(GradescopeVisibility::Visible)
                                           .build();

        let mut file = fs::File::create(if gradescope_debug {
                                            "./results.json"
                                        } else {
                                            "/autograder/results/results.json"
                                        })?;
        file.write_all(serde_json::to_string_pretty(&submission)?.as_bytes())?;
    }

    Ok(())
}

#[derive(Clone, Default)]
/// string. Any difference results in a `0` grade.
/// A grader that grades by diffing an `expected` string with an `actual`
pub struct DiffGrader {
    /// name of requirement
    pub req_name:    String,
    /// points to give if all tests pass
    pub out_of:      f64,
    /// the project to grade
    pub project:     Project,
    /// Java file to run
    pub file:        String,
    /// the expected output
    pub expected:    Array,
    /// the actual output
    pub input:       Array,
    /// ignore case when comparing
    pub ignore_case: bool,
}

impl DiffGrader {
    /// creates a new DiffGrader
    pub fn new() -> Self {
        Self::default()
    }

    /// gets the `req_name` field
    pub fn req_name(&mut self) -> String {
        self.req_name.clone()
    }

    /// sets the `req_name` field
    pub fn set_req_name(mut self,
                        req_name: String)
                        -> Self {
        self.req_name = req_name;
        self
    }

    /// gets the `out_of` field
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// sets the `out_of` field
    pub fn set_out_of(mut self,
                      out_of: f64)
                      -> Self {
        self.out_of = out_of;
        self
    }

    /// gets the `expected` field
    pub fn expected(&mut self) -> Array {
        self.expected.clone()
    }

    /// sets the `expected` field
    pub fn set_expected(mut self,
                        expected: Array)
                        -> Self {
        self.expected = expected;
        self
    }

    /// gets the `actual` field
    pub fn input(&mut self) -> Array {
        self.input.clone()
    }

    /// sets the `actual` field
    pub fn set_input(mut self,
                     input: Array)
                     -> Self {
        self.input = input;
        self
    }

    /// gets the `project` field
    pub fn project(&mut self) -> Project {
        self.project.clone()
    }

    /// sets the `project` field
    pub fn set_project(mut self,
                       project: Project)
                       -> Self {
        self.project = project;
        self
    }

    /// gets the `file` field
    pub fn file(&mut self) -> String {
        self.file.clone()
    }

    /// sets the `file` field
    pub fn set_file(mut self,
                    file: String)
                    -> Self {
        self.file = file;
        self
    }

    /// gets the `ignore_case` field
    pub fn ignore_case(&mut self) -> bool {
        self.ignore_case
    }

    /// sets the `ignore_case` field
    pub fn set_ignore_case(mut self,
                           ignore_case: bool)
                           -> Self {
        self.ignore_case = ignore_case;
        self
    }

    #[generate_rhai_variant(Fallible)]
    /// Grades by diffing the `expected` and `actual` strings.
    pub fn grade_by_diff(&mut self) -> Result<GradeResult> {
        ensure!(!self.expected.is_empty() & !self.input.is_empty(),
                "At least one test case (input-expected pair) must be provided");
        ensure!(self.expected.len() == self.input.len(),
                "expected and input case arrays must be of the same length");

        let file = self.project.identify(&self.file)?;
        let mut prompts = vec![];

        for (expected, input) in self.expected.iter().zip(self.input.iter()) {
            let expected = {
                let expected = expected.clone().cast::<String>();
                if self.ignore_case {
                    expected.to_lowercase().trim().to_string()
                } else {
                    expected.trim().to_string()
                }
            };
            let input = input.clone().cast::<String>();

            let actual_out = {
                let out = match file.run(Some(input.clone())) {
                    Ok(out) => out,
                    Err(JavaFileError::AtRuntime { output, diags }) => {
                        let messages = vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(SYSTEM_MESSAGE.to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(format!("Error while running -\n```\n{}\n```", output))
                                .name("Student".to_string())
                                .build()
                                .context("Failed to build user message")?
                                .into(),
                            get_source_context(diags, self.project.clone(), 3, 6, 6, false, None)?,
                        ];
                        return Ok(GradeResult { requirement: self.req_name.clone(),
                                                grade:       Grade::new(0.0, self.out_of),
                                                reason:
                                                    "Error running file for some cases.".to_string(),
                                                prompt:      Some(messages), });
                    }
                    Err(JavaFileError::DuringCompilation { stacktrace, diags }) => {
                        let messages = vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(SYSTEM_MESSAGE.to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(format!(
                                    "Error while compiling -\n```\n{}\n```",
                                    stacktrace
                                ))
                                .name("Student".to_string())
                                .build()
                                .context("Failed to build user message")?
                                .into(),
                            get_source_context(diags, self.project.clone(), 3, 6, 6, false, None)?,
                        ];
                        return Ok(GradeResult {
                            requirement: self.req_name.clone(),
                            grade:       Grade::new(0.0, self.out_of),
                            reason:      "Error compiling file for some cases.".to_string(),
                            prompt:      Some(messages),
                        });
                    }
                    Err(e) => {
                        let messages = vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(SYSTEM_MESSAGE.to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(format!("Unknown error -\n```\n{:?}\n```", e))
                                .name("Student".to_string())
                                .build()
                                .context("Failed to build user message")?
                                .into(),
                        ];
                        return Ok(GradeResult { requirement: self.req_name.clone(),
                                                grade:       Grade::new(0.0, self.out_of),
                                                reason:      "Unknown error while running file \
                                                              for some cases."
                                                                              .to_string(),
                                                prompt:      Some(messages), });
                    }
                };

                if self.ignore_case {
                    out.to_lowercase().trim().to_string()
                } else {
                    out.trim().to_string()
                }
            };

            let diff = diff_unicode_words(Algorithm::Patience, &expected, &actual_out);

            let mut is_equal = true;
            let mut expected = String::new();
            let mut actual = String::new();

            for (change, value) in diff {
                match change {
                    ChangeTag::Equal => {
                        expected.push_str(value);
                        actual.push_str(value);
                    }
                    ChangeTag::Insert => {
                        actual.push_str(format!("{}", value.green()).as_str());
                        if !value.trim().is_empty() {
                            is_equal = false;
                        }
                    }
                    ChangeTag::Delete => {
                        expected.push_str(format!("{}", value.red()).as_str());
                        if !value.trim().is_empty() {
                            is_equal = false;
                        }
                    }
                }
            }

            if !is_equal {
                let prompt = format!("Comparing expected and actual output for \
                                      {}:\n```{inp}Expected:\n{}\nActual:\n{}\n```\n",
                                     file.file_name(),
                                     expected,
                                     actual,
                                     inp = if self.input.is_empty() {
                                         String::new()
                                     } else {
                                         format!("\nInput:\n`{}`\n", input)
                                     },);

                eprintln!("{prompt}");
                prompts.push(prompt);
            }
        }

        if prompts.is_empty() {
            Ok(GradeResult { requirement: self.req_name.clone(),
                             grade:       Grade { grade:  self.out_of,
                                                  out_of: self.out_of, },
                             reason:      "Got expected output".to_string(),
                             prompt:      None, })
        } else {
            let context = format!("{prompt}\n\nSource code:\n```java\n{code}\n```\nMy tests are \
                                   failing due to the above.",
                                  prompt = prompts.join("\n\n"),
                                  code = file.parser().code());

            Ok(GradeResult { requirement: self.req_name.clone(),
                             grade:       Grade { grade:  0.0,
                                                  out_of: self.out_of, },
                             reason:      "See above.".to_string(),
                             prompt:      Some(vec![
                ChatCompletionRequestSystemMessageArgs::default().content(
                    SYSTEM_MESSAGE.to_string(),
                )
                                                                 .name("Instructor".to_string())
                                                                 .build()
                                                                 .context(
                    "Failed to build system message",
                )?
                                                                 .into(),
                ChatCompletionRequestSystemMessageArgs::default().content(context)
                                                                 .name("Student".to_string())
                                                                 .build()
                                                                 .context(
                    "Failed to build system message",
                )?
                                                                 .into(),
            ]), })
        }
    }
}

/// Schema for `prompts` table
#[derive(Serialize, Debug)]
pub struct PromptRow {
    /// UUID of data entry
    id:               String,
    /// ChatGPT message prompt
    messages:         Option<Vec<ChatCompletionRequestMessage>>,
    /// Name of the autograder requirement
    requirement_name: String,
    /// Reasons for penalty
    reason:           String,
    /// Grade/out_of as a string
    grade:            String,
    /// Status of prompt response generation - not_started, started, completed
    status:           String,
}

#[generate_rhai_variant(Fallible)]
/// Generates feedback for a single `GradeResult` and posts it to the database.
fn generate_single_feedback(result: &GradeResult) -> Result<String> {
    let rt = RUNTIME.handle().clone();

    if result.grade.grade < result.grade.out_of {
        let id = uuid::Uuid::new_v4().to_string();
        let mut result = result.clone();
        let body = PromptRow { id:               id.clone(),
                               messages:         result.prompt(),
                               requirement_name: result.requirement(),
                               reason:           result.reason(),
                               grade:            result.grade.to_string(),
                               status:           "not_started".into(), };

        let messages = serde_json::to_string(&body)?;

        // Post to the database
        rt.block_on(async {
              POSTGREST_CLIENT.from("prompts")
                              .insert(messages)
                              .execute()
                              .await
          })?;

        // Return feedback URL
        Ok(format!(
            "- For explanation and feedback on `{}` (refer rubric), please \
             see this link - https://feedback.dhruvdh.com/{}",
            result.requirement(),
            id
        ))
    } else {
        Ok(String::from("This type of feedback cannot be generated \
                         for submissions without penalty."))
    }
}

#[generate_rhai_variant(Fallible)]
/// Generates a FEEDBACK file after prompting ChatGPT for feedback on an array
/// of results.
pub fn generate_feedback(results: Array) -> Result<()> {
    let mut feedback = vec!["## Understanding Your Autograder Results\n".to_string()];

    for result in results.iter().map(|f| f.clone().cast::<GradeResult>()) {
        match generate_single_feedback(&result) {
            Ok(fb) => feedback.push(fb),
            Err(e) => eprintln!("Error generating feedback: {}", e),
        }
    }

    if !feedback.is_empty() {
        let feedback = feedback.join("\n");
        fs::write("FEEDBACK", &feedback).context("Something went wrong writing FEEDBACK file.")?;
        eprintln!("{}", &feedback);
    } else {
        fs::write(
            "FEEDBACK",
            "This type of feedback cannot be generated for submissions without penalty.",
        )
        .context("Something went wrong writing FEEDBACK file.")?;
    }

    Ok(())
}

#[derive(Default, Debug, Clone)]
/// A struct to represent a treesitter query.
pub struct Query {
    /// The query to run.
    query:   String,
    /// The capture to extract from the query.
    capture: String,
    /// A function pointer to filter the matches using. Must return a boolean.
    filter:  Option<FnPtr>,
}

impl Query {
    /// Creates a new query with default values (empty strings).
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the query to run.
    pub fn query(&self) -> String {
        unescape(&format!("{:#?}", self.query)).unwrap()
    }

    /// Sets the query to run.
    pub fn set_query(mut self,
                     query: String)
                     -> Self {
        self.query = query;
        self
    }

    /// Gets the captures to extract from the query.
    pub fn capture(&self) -> String {
        self.capture.clone()
    }

    /// Sets the captures to extract from the query.
    pub fn set_capture(mut self,
                       capture: String)
                       -> Self {
        self.capture = capture;
        self
    }

    /// Gets the function to filter the results of the query.
    pub fn filter(&self) -> Option<FnPtr> {
        self.filter.clone()
    }

    /// Set the function to filter the results of the query.
    pub fn set_filter(mut self,
                      filter: FnPtr)
                      -> Self {
        self.filter = Some(filter);
        self
    }
}

/// An enum to represent possible errors when running a query.
#[derive(thiserror::Error, Debug)]
pub enum QueryError {
    /// No file was selected to run the query on.
    #[error("No file was selected to run the query on.")]
    NoFileSelected,
    /// No capture was selected to extract from the query.
    #[error("No capture was selected to extract from the query: {0}")]
    NoCaptureSelected(String),
    /// No previous query to add capture or filter to.
    #[error("No previous query to add capture or filter to.")]
    NoPreviousQuery,
    /// The file selected to run the query on does not exist.
    #[error("The file selected (`{0}`) to run the query on could not be found.")]
    FileNotFound(String),
    /// The query could not be run.
    #[error("This query could not be run, likely due to a syntax \
             error.\nQuery:\n```\n{q}\n```\nError:\n```\n{e}\n```")]
    DuringQueryExecution {
        /// The query that could not be run.
        q: String,
        /// The error that occurred.
        e: String,
    },
    /// No matches found for a previously selected capture, all subsequent
    /// queries will return nothing.
    #[error("No matches found for a previously selected capture: `{0}`, all subsequent queries \
             will return nothing.")]
    NoMatchesFound(String),
    /// Unknown error.
    #[error("Unknown error: {0}")]
    Unknown(#[from] anyhow::Error),
}

#[derive(Default, Clone)]
/// An enum to represent the constraint of a query.
pub enum QueryConstraint {
    #[default]
    /// The query must match at least once.
    MustMatchAtLeastOnce,
    /// The query must match exactly once.
    MustMatchExactlyNTimes(usize),
    /// Must not match.
    MustNotMatch,
}

#[derive(Default, Clone)]
/// A struct to represent a query grader.
pub struct QueryGrader {
    /// The name of the requirement.
    req_name:   String,
    /// The grade for the requirement.
    out_of:     f64,
    /// The queries to run.
    queries:    Vec<Query>,
    /// The input to run the queries on.
    project:    Project,
    /// The file to run the query on.
    file:       String,
    /// The constraint of the query.
    constraint: QueryConstraint,
    /// The reason to share with the student.
    reason:     String,
}

impl QueryGrader {
    /// Creates a new query grader with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the name of the requirement.
    pub fn req_name(&self) -> &str {
        &self.req_name
    }

    /// Sets the name of the requirement.
    pub fn set_req_name(mut self,
                        req_name: String)
                        -> Self {
        self.req_name = req_name;
        self
    }

    /// Gets the "out of" grade for the requirement.
    pub fn out_of(&self) -> f64 {
        self.out_of
    }

    /// Sets the "out of" grade for the requirement.
    pub fn set_out_of(mut self,
                      out_of: f64)
                      -> Self {
        self.out_of = out_of;
        self
    }

    /// Gets the file to run the query on.
    pub fn file(&self) -> &str {
        &self.file
    }

    /// Sets the file to run the query on.
    pub fn set_file(mut self,
                    file: String)
                    -> Self {
        self.file = file;
        self
    }

    /// Gets the project to run the query on.
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Sets the project to run the query on.
    pub fn set_project(mut self,
                       project: Project)
                       -> Self {
        self.project = project;
        self
    }

    /// Gets the queries to run.
    pub fn queries(&self) -> Vec<Query> {
        self.queries.clone()
    }

    /// Gets the constraint of the query.
    pub fn constraint(&self) -> QueryConstraint {
        self.constraint.clone()
    }

    /// Sets the constraint of the query to "must match at least once".
    pub fn must_match_at_least_once(mut self) -> Self {
        self.constraint = QueryConstraint::MustMatchAtLeastOnce;
        self
    }

    /// Sets the constraint of the query to "must match exactly n times".
    pub fn must_match_exactly_n_times(mut self,
                                      n: usize)
                                      -> Self {
        self.constraint = QueryConstraint::MustMatchExactlyNTimes(n);
        self
    }

    /// Sets the constraint of the query to "must not match".
    pub fn must_not_match(mut self) -> Self {
        self.constraint = QueryConstraint::MustNotMatch;
        self
    }

    /// Gets the reason to share with the student.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Sets the reason to share with the student.
    pub fn set_reason(mut self,
                      reason: String)
                      -> Self {
        self.reason = reason;
        self
    }

    #[generate_rhai_variant(Fallible)]
    /// Adds a query to run.
    /// If no file has been selected, this will throw an error.
    pub fn query(#[allow(unused_mut)] mut self,
                 q: String)
                 -> Result<Self, QueryError> {
        if self.file.is_empty() {
            return Err(QueryError::NoFileSelected);
        }

        self.queries.push(Query { query:   q,
                                  capture: String::new(),
                                  filter:  None, });

        Ok(self)
    }

    #[generate_rhai_variant(Fallible)]
    /// Adds a capture to the last query.
    /// If no queries have been added, this will throw an error.
    pub fn capture(#[allow(unused_mut)] mut self,
                   c: String)
                   -> Result<Self, QueryError> {
        if let Some(last) = self.queries.last_mut() {
            *last = last.clone().set_capture(c);
            Ok(self)
        } else {
            Err(QueryError::NoPreviousQuery)
        }
    }

    #[generate_rhai_variant(Fallible)]
    /// Adds a capture to the last query.
    /// If no queries have been added, this will throw an error.
    pub fn filter(#[allow(unused_mut)] mut self,
                  f: FnPtr)
                  -> Result<Self, QueryError> {
        if let Some(last) = self.queries.last_mut() {
            *last = last.clone().set_filter(f);
            Ok(self)
        } else {
            Err(QueryError::NoPreviousQuery)
        }
    }

    /// Selects entire method body and returns
    pub fn method_body_with_name(mut self,
                                 method_name: String)
                                 -> Self {
        self.queries
            .push(Query { query:   format!(include_str!("queries/method_body_with_name.scm"),
                                           method_name),
                          capture: "body".to_string(),
                          filter:  None, });
        self
    }

    /// Selects entire method body and returns
    pub fn method_body_with_return_type(mut self,
                                        return_type: String)
                                        -> Self {
        self.queries.push(Query { query:   format!(
            include_str!("queries/method_body_with_return_type.scm"),
            return_type
        ),
                                  capture: "body".to_string(),
                                  filter:  None, });
        self
    }

    /// Selects and returns the entire main method
    pub fn main_method(mut self) -> Self {
        self.queries
            .push(Query { query:   include_str!("queries/main_method.scm").to_string(),
                          capture: "body".to_string(),
                          filter:  None, });
        self
    }

    /// Selects entire class body with name
    pub fn class_body_with_name(mut self,
                                class_name: String)
                                -> Self {
        self.queries
            .push(Query { query:   format!(include_str!("queries/class_with_name.scm"), class_name),
                          capture: "body".to_string(),
                          filter:  None, });
        self
    }

    /// Selects local variable declaration statements
    pub fn local_variables(mut self) -> Self {
        self.queries
            .push(Query { query:   String::from("((local_variable_declaration) @var)"),
                          capture: "var".to_string(),
                          filter:  None, });
        self
    }

    /// Selects local variable declaration statements with supplied name
    pub fn local_variables_with_name(mut self,
                                     name: String)
                                     -> Self {
        self.queries.push(Query { query:
                                      format!(include_str!("queries/local_variable_with_name.scm"),
                                              name),
                                  capture: "body".to_string(),
                                  filter:  None, });
        self
    }

    /// Selects local variable declaration statements with supplied type
    pub fn local_variables_with_type(mut self,
                                     type_name: String)
                                     -> Self {
        self.queries.push(Query { query:   format!(
            include_str!("queries/local_variable_with_type.scm"),
            type_name
        ),
                                  capture: "body".to_string(),
                                  filter:  None, });
        self
    }

    /// Selects if statements (entire, including else if and else)
    pub fn if_statements(mut self) -> Self {
        self.queries
            .push(Query { query:   String::from("((if_statement) @if)"),
                          capture: "if".to_string(),
                          filter:  None, });
        self
    }

    /// Selects for loops
    pub fn for_loops(mut self) -> Self {
        self.queries
            .push(Query { query:   String::from("((for_statement) @for)"),
                          capture: "for".to_string(),
                          filter:  None, });
        self
    }

    /// Selects while loops
    pub fn while_loops(mut self) -> Self {
        self.queries
            .push(Query { query:   String::from("((while_statement) @while)"),
                          capture: "while".to_string(),
                          filter:  None, });
        self
    }

    /// Selects method invocations
    pub fn method_invocations(mut self) -> Self {
        self.queries
            .push(Query { query:   include_str!("queries/method_invocation.scm").to_string(),
                          capture: "body".to_string(),
                          filter:  None, });
        self
    }

    /// Selects method invocations with supplied name
    pub fn method_invocations_with_name(mut self,
                                        name: String)
                                        -> Self {
        self.queries.push(Query { query:   format!(
            include_str!("queries/method_invocations_with_name.scm"),
            name
        ),
                                  capture: "body".to_string(),
                                  filter:  None, });
        self
    }

    /// Selects method invocations with supplied arguments
    pub fn method_invocations_with_arguments(mut self,
                                             name: String)
                                             -> Self {
        self.queries.push(Query { query:   format!(
            include_str!("queries/method_invocations_with_arguments.scm"),
            name
        ),
                                  capture: "body".to_string(),
                                  filter:  None, });
        self
    }

    /// Selects method invocations with supplied object
    pub fn method_invocations_with_object(mut self,
                                          name: String)
                                          -> Self {
        self.queries.push(Query { query:   format!(
            include_str!("queries/method_invocations_with_object.scm"),
            name
        ),
                                  capture: "body".to_string(),
                                  filter:  None, });
        self
    }

    #[generate_rhai_variant(Fallible)]
    /// Runs the queries, and returns the result.
    /// TODO: Make it so that it doesn't parse a new piece of code, just filters
    /// out the irrelevant line ranges. This performs better but more
    /// importantly is more accurate.
    pub fn run_query(&self) -> Result<Dynamic, QueryError> {
        let engine = create_engine();
        let ast = std::sync::Arc::clone(&SCRIPT_AST);
        let ast = ast.lock().unwrap();

        let first =
            self.queries
                .first()
                .ok_or_else(|| QueryError::NoMatchesFound("No queries to run".to_string()))?;

        let file = self.project
                       .identify(self.file())
                       .map_err(|_| QueryError::FileNotFound(self.file().to_string()))?;

        let mut matches: Vec<String> = match file.query(&first.query()) {
            Ok(m) => {
                if first.capture().is_empty() {
                    return Err(QueryError::NoCaptureSelected(format!("{:#?}", first)));
                }
                let result = m.iter()
                              .filter_map(|map| map.get(&first.capture()))
                              .cloned();

                let result: Vec<String> = if let Some(f) = first.filter() {
                    result.filter(|x| f.call(&engine, &ast, (x.clone(),)).unwrap_or(false))
                          .collect()
                } else {
                    result.collect()
                };

                if m.is_empty() {
                    return Err(QueryError::NoMatchesFound(
                        unescape(&format!("{:#?}", first)).context("Unescape error")?,
                    ));
                }
                result
            }
            Err(e) => {
                return Err(QueryError::DuringQueryExecution { q: first.query(),
                                                              e: format!("{:#?}", e), })
            }
        };

        if self.queries.len() == 1 {
            return Ok(matches.into());
        }

        for (prev_q, q) in self.queries().into_iter().tuple_windows() {
            if matches.is_empty() {
                return Err(QueryError::NoMatchesFound(
                    unescape(&format!("{:#?}", prev_q)).context("Unescape error")?,
                ));
            }

            if q.capture().is_empty() {
                return Err(QueryError::NoCaptureSelected(format!("{:#?}", q)));
            }

            let mut new_matches = vec![];

            for code in matches {
                let parser = Parser::new(code).context(format!("Failed to create parser for \
                                                                query: `{}`",
                                                               q.query()))?;

                match parser.query(&q.query()) {
                    Ok(m) => {
                        let result = m.iter().filter_map(|map| map.get(&q.capture())).cloned();

                        let mut result: Vec<String> = if let Some(f) = q.filter() {
                            result.filter(|x| f.call(&engine, &ast, (x.clone(),)).unwrap_or(false))
                                  .collect()
                        } else {
                            result.collect()
                        };

                        new_matches.append(&mut result)
                    }
                    Err(e) => {
                        return Err(QueryError::DuringQueryExecution { q: q.query(),
                                                                      e: format!("{:#?}", e), })
                    }
                };
            }

            matches = new_matches;
        }

        Ok(matches.into())
    }

    #[generate_rhai_variant(Fallible)]
    /// Grades the file according to the supplied queries, captures, and
    /// constraints.
    pub fn grade_by_query(self) -> Result<GradeResult> {
        let reason = if self.reason.trim().is_empty() {
            eprintln!("Warning: No reason provided for query grading. Feedback to student will \
                       not be very helpful.");
            match self.constraint {
                QueryConstraint::MustMatchAtLeastOnce => {
                    "Query Constraint: Must match at least once.".to_string()
                }
                QueryConstraint::MustMatchExactlyNTimes(n) => {
                    format!("Query Constraint: Must match exactly {n} times.")
                }
                QueryConstraint::MustNotMatch => "Query Constraint: Must not match.".to_string(),
            }
        } else {
            self.reason.to_string()
        };

        let result: Vec<String> = match self.run_query() {
            Ok(r) => {
                let r: Array = r.cast();
                r.into_iter().map(|s| s.cast()).collect()
            }
            Err(e) => {
                return Ok(GradeResult { requirement: self.req_name.clone(),
                                        grade: Grade { grade:  0.0,
                                                       out_of: self.out_of, },
                                        reason,
                                        prompt: Some(vec![
                    ChatCompletionRequestSystemMessageArgs::default().content(
                        SYSTEM_MESSAGE.to_string(),
                    )
                                                                     .name("Instructor".to_string())
                                                                     .build()
                                                                     .context(
                        "Failed to build system message",
                    )?
                                                                     .into(),
                    ChatCompletionRequestSystemMessageArgs::default().content(format!(
                        "Something went wrong when using treesitter queries to grade `{}`. Error \
                         message:\n\n```\n{}\n```\n",
                        self.file, e
                    ))
                                                                     .name("Instructor".to_string())
                                                                     .build()
                                                                     .context(
                        "Failed to build system message",
                    )?
                                                                     .into(),
                ]) })
            }
        };

        match self.constraint {
            QueryConstraint::MustMatchAtLeastOnce => {
                if result.is_empty() {
                    Ok(GradeResult { requirement: self.req_name.clone(),
                                     grade: Grade { grade:  0.0,
                                                    out_of: self.out_of, },
                                     reason,
                                     prompt: Some(vec![
                        ChatCompletionRequestSystemMessageArgs::default().content(
                            SYSTEM_MESSAGE.to_string(),
                        )
                                                                         .name(
                            "Instructor".to_string(),
                        )
                                                                         .build()
                                                                         .context(
                            "Failed to build system message",
                        )?
                                                                         .into(),
                        ChatCompletionRequestSystemMessageArgs::default().content(format!(
                            "For file `{}`: {}.",
                            self.file, self.reason
                        ))
                                                                         .name(
                            "Instructor".to_string(),
                        )
                                                                         .build()
                                                                         .context(
                            "Failed to build system message",
                        )?
                                                                         .into(),
                    ]) })
                } else {
                    Ok(GradeResult { requirement: self.req_name.clone(),
                                     grade: Grade { grade:  self.out_of,
                                                    out_of: self.out_of, },
                                     reason,
                                     prompt: None })
                }
            }
            QueryConstraint::MustMatchExactlyNTimes(n) => {
                if result.len() == n {
                    Ok(GradeResult { requirement: self.req_name.clone(),
                                     grade: Grade { grade:  self.out_of,
                                                    out_of: self.out_of, },
                                     reason,
                                     prompt: None })
                } else {
                    Ok(GradeResult { requirement: self.req_name.clone(),
                                     grade: Grade { grade:  0.0,
                                                    out_of: self.out_of, },
                                     reason,
                                     prompt: Some(vec![
                        ChatCompletionRequestSystemMessageArgs::default().content(
                            SYSTEM_MESSAGE.to_string(),
                        )
                                                                         .name(
                            "Instructor".to_string(),
                        )
                                                                         .build()
                                                                         .context(
                            "Failed to build system message",
                        )?
                                                                         .into(),
                        ChatCompletionRequestSystemMessageArgs::default().content(format!(
                            "For file `{}`: {}",
                            self.file, self.reason
                        ))
                                                                         .name(
                            "Instructor".to_string(),
                        )
                                                                         .build()
                                                                         .context(
                            "Failed to build system message",
                        )?
                                                                         .into(),
                    ]) })
                }
            }
            QueryConstraint::MustNotMatch => {
                if result.is_empty() {
                    Ok(GradeResult { requirement: self.req_name.clone(),
                                     grade: Grade { grade:  self.out_of,
                                                    out_of: self.out_of, },
                                     reason,
                                     prompt: None })
                } else {
                    Ok(GradeResult { requirement: self.req_name.clone(),
                                     grade: Grade { grade:  0.0,
                                                    out_of: self.out_of, },
                                     reason,
                                     prompt: Some(vec![
                        ChatCompletionRequestSystemMessageArgs::default().content(
                            SYSTEM_MESSAGE.to_string(),
                        )
                                                                         .name(
                            "Instructor".to_string(),
                        )
                                                                         .build()
                                                                         .context(
                            "Failed to build system message",
                        )?
                                                                         .into(),
                        ChatCompletionRequestSystemMessageArgs::default().content(format!(
                            "For file `{}`: {}",
                            self.file, self.reason
                        ))
                                                                         .name(
                            "Instructor".to_string(),
                        )
                                                                         .build()
                                                                         .context(
                            "Failed to build system message",
                        )?
                                                                         .into(),
                    ]) })
                }
            }
        }
    }
}

// Allowed because CustomType is volatile, not deprecated
#[allow(deprecated)]
/// Allows registering custom types with Rhai.
impl CustomType for Grade {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("Grade")
               .with_fn("grade", Self::grade)
               .with_fn("grade", Self::set_grade)
               .with_fn("out_of", Self::out_of)
               .with_fn("out_of", Self::set_out_of)
               .with_fn("new_grade", Self::new)
               .with_fn("from_string", Self::grade_from_string_script)
               .with_fn("to_string", Self::to_string);
    }
}

// Allowed because CustomType is volatile, not deprecated
#[allow(deprecated)]
/// Allows registering custom types with Rhai.
impl CustomType for GradeResult {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("GradeResult")
               .with_fn("requirement", Self::requirement)
               .with_fn("requirement", Self::set_requirement)
               .with_fn("grade", Self::grade)
               .with_fn("grade", Self::set_grade)
               .with_fn("out_of", Self::out_of)
               .with_fn("out_of", Self::set_out_of)
               .with_fn("reason", Self::reason)
               .with_fn("reason", Self::set_reason)
               .with_fn("new_grade_result", Self::default);
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
/// Allows registering custom types with Rhai
impl CustomType for DocsGrader {
    /// Builds a custom type to be registered with Rhai
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("DocsGrader")
               .with_fn("req_name", Self::req_name)
               .with_fn("req_name", Self::set_req_name)
               .with_fn("project", Self::project)
               .with_fn("project", Self::set_project)
               .with_fn("files", Self::files)
               .with_fn("files", Self::set_files)
               .with_fn("out_of", Self::out_of)
               .with_fn("out_of", Self::set_out_of)
               .with_fn("penalty", Self::penalty)
               .with_fn("penalty", Self::set_penalty)
               .with_fn("new_docs_grader", Self::default)
               .with_fn("run", Self::grade_docs_script);
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
/// Allows registering custom types with Rhai
impl CustomType for ByUnitTestGrader {
    /// Builds a custom type to be registered with Rhai
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("ByUnitTestGrader")
               .with_fn("test_files", Self::test_files)
               .with_fn("test_files", Self::set_test_files)
               .with_fn("project", Self::project)
               .with_fn("project", Self::set_project)
               .with_fn("expected_tests", Self::expected_tests)
               .with_fn("expected_tests", Self::set_expected_tests)
               .with_fn("out_of", Self::out_of)
               .with_fn("out_of", Self::set_out_of)
               .with_fn("req_name", Self::req_name)
               .with_fn("req_name", Self::set_req_name)
               .with_fn("new_by_unit_test_grader", Self::default)
               .with_fn("run", Self::grade_by_tests_script);
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
/// Allows registering custom types with Rhai
impl CustomType for UnitTestGrader {
    /// Builds a custom type to be registered with Rhai
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("UnitTestGrader")
               .with_fn("req_name", Self::get_req_name)
               .with_fn("req_name", Self::set_req_name)
               .with_fn("out_of", Self::get_out_of)
               .with_fn("out_of", Self::set_out_of)
               .with_fn("target_test", Self::get_target_test)
               .with_fn("target_test", Self::set_target_test)
               .with_fn("target_class", Self::get_target_class)
               .with_fn("target_class", Self::set_target_class)
               .with_fn("excluded_methods", Self::get_excluded_methods)
               .with_fn("excluded_methods", Self::set_excluded_methods)
               .with_fn("avoid_calls_to", Self::get_avoid_calls_to)
               .with_fn("avoid_calls_to", Self::set_avoid_calls_to)
               .with_fn("new_unit_test_grader", Self::default)
               .with_fn("run", Self::grade_unit_tests_script);
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
/// Allows registering custom types with Rhai.
impl CustomType for ByHiddenTestGrader {
    /// Builds a custom type to be registered with Rhai.
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("ByHiddenTestGrader")
               .with_fn("url", Self::url)
               .with_fn("url", Self::set_url)
               .with_fn("test_class_name", Self::test_class_name)
               .with_fn("test_class_name", Self::set_test_class_name)
               .with_fn("out_of", Self::out_of)
               .with_fn("out_of", Self::set_out_of)
               .with_fn("req_name", Self::req_name)
               .with_fn("req_name", Self::set_req_name)
               .with_fn("new_by_hidden_test_grader", Self::default)
               .with_fn("run", Self::grade_by_hidden_tests_script);
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
/// Allows registering custom types with Rhai.
impl CustomType for DiffGrader {
    /// Builds a custom type to be registered with Rhai.
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("DiffGrader")
               .with_fn("req_name", Self::req_name)
               .with_fn("req_name", Self::set_req_name)
               .with_fn("out_of", Self::out_of)
               .with_fn("out_of", Self::set_out_of)
               .with_fn("expected", Self::expected)
               .with_fn("expected", Self::set_expected)
               .with_fn("input", Self::input)
               .with_fn("input", Self::set_input)
               .with_fn("project", Self::project)
               .with_fn("project", Self::set_project)
               .with_fn("file", Self::file)
               .with_fn("file", Self::set_file)
               .with_fn("ignore_case", Self::ignore_case)
               .with_fn("ignore_case", Self::set_ignore_case)
               .with_fn("new_diff_grader", Self::default)
               .with_fn("run", Self::grade_by_diff_script);
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
/// Allows registering custom types with Rhai.
impl CustomType for Query {
    /// Builds a custom type to be registered with Rhai.
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("Query")
               .with_fn("new_query", Self::new)
               .with_fn("query", Self::query)
               .with_fn("query", Self::set_query)
               .with_fn("capture", Self::capture)
               .with_fn("capture", Self::set_capture);
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
/// Allows registering custom types with Rhai.
impl CustomType for QueryGrader {
    /// Builds a custom type to be registered with Rhai.
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("QueryGrader")
               .with_fn("req_name", Self::req_name)
               .with_fn("req_name", Self::set_req_name)
               .with_fn("out_of", Self::out_of)
               .with_fn("out_of", Self::set_out_of)
               .with_fn("file", Self::file)
               .with_fn("file", Self::set_file)
               .with_fn("project", Self::project)
               .with_fn("project", Self::set_project)
               .with_fn("queries", Self::queries)
               .with_fn("query", Self::query_script)
               .with_fn("capture", Self::capture_script)
               .with_fn("reason", Self::reason)
               .with_fn("reason", Self::set_reason)
               .with_fn("must_match_at_least_once", Self::must_match_at_least_once)
               .with_fn("must_match_exactly_n_times",
                        Self::must_match_exactly_n_times)
               .with_fn("must_not_match", Self::must_not_match)
               .with_fn("method_body_with_name", Self::method_body_with_name)
               .with_fn("method_body_with_return_type",
                        Self::method_body_with_return_type)
               .with_fn("main_method", Self::main_method)
               .with_fn("class_body_with_name", Self::class_body_with_name)
               .with_fn("local_variables", Self::local_variables)
               .with_fn("local_variables_with_name", Self::local_variables_with_name)
               .with_fn("local_variables_with_type", Self::local_variables_with_type)
               .with_fn("if_statements", Self::if_statements)
               .with_fn("for_loops", Self::for_loops)
               .with_fn("while_loops", Self::while_loops)
               .with_fn("method_invocations", Self::method_invocations)
               .with_fn("method_invocations_with_name",
                        Self::method_invocations_with_name)
               .with_fn("method_invocations_with_arguments",
                        Self::method_invocations_with_arguments)
               .with_fn("method_invocations_with_object",
                        Self::method_invocations_with_object)
               .with_fn("filter", Self::filter_script)
               .with_fn("run_query", Self::run_query_script)
               .with_fn("run", Self::grade_by_query_script)
               .with_fn("new_query_grader", Self::default);
    }
}

    </file-contents>
    <file-contents path="./src/health.rs" name="health.rs">
// TODO: make recommendations for the above

use anyhow::{Context, Result};
use futures::{future::try_join_all, stream::FuturesUnordered};
use tokio::{fs::OpenOptions, task::JoinError};
use walkdir::WalkDir;

use crate::{
    constants::{BUILD_DIR, LIB_DIR, ROOT_DIR, RUNTIME, SOURCE_DIR, TEST_DIR},
    java::{FileType, Project},
};

impl Project {
    /// Checks the project for common CodingRooms errors
    pub fn check_health(&self) -> Result<()> {
        tracing::info!("Checking Project Health...");
        let project = Project::new()?;

        let rt = RUNTIME.handle().clone();
        let _guard = rt.enter();

        let handle1 = rt.spawn(async {
                            let files = WalkDir::new(ROOT_DIR.as_path())
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_path_buf())
                .map(|path| {
                    tokio::spawn(async move {
                        match tokio::fs::metadata(path.clone()).await {
                            Ok(m) => {
                                if m.len() == 0 {
                                    tracing::warn!("File {}\n\tis empty", &path.display())
                                }
                                if let Err(e) =
                                    OpenOptions::new().read(true).write(true).open(&path).await
                                {
                                    tracing::warn!(
                                        "File {}\n\tcould not be opened (read + write): {}",
                                        &path.display(),
                                        e
                                    )
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Could not read file {}: {}", path.display(), e)
                            }
                        };

                        if path.extension().unwrap_or_default() == "jar" {
                            let output = tokio::process::Command::new("zip")
                                .arg("-T")
                                .arg(&path)
                                .output()
                                .await
                                .unwrap_or_else(|_| {
                                    panic!("Could not run zip -T on {}", &path.display())
                                });

                            if !output.status.success() {
                                tracing::warn!(
                                    "File {}\n\tis not a valid zip file: {}",
                                    &path.display(),
                                    String::from_utf8_lossy(&output.stderr)
                                )
                            }
                        }
                    })
                })
                .collect::<FuturesUnordered<_>>();

                            try_join_all(files).await
                        });

        let handle2 =
            rt.spawn(async move {
                  let files =
                      project.files()
                             .iter()
                             .map(|file| {
                                 let file = file.clone();
                                 tokio::spawn(async move {
                                     if file.package_name().is_none() {
                                         tracing::warn!("File {}\n\tdoesn't belong to any package",
                                                        file.path().display());
                                     } else {
                                         let expected_path = if let FileType::Test = file.kind() {
                                             TEST_DIR.join(file.package_name().unwrap())
                                         } else {
                                             SOURCE_DIR.join(file.package_name().unwrap())
                                         };
                                         if file.path().parent().unwrap_or(&ROOT_DIR)
                                            != expected_path.as_path()
                                         {
                                             tracing::warn!("File {}\n\tis in the wrong \
                                                             directory.\n\t\tExpected: \
                                                             {}\n\t\tFound: {}",
                                                            file.path().display(),
                                                            expected_path.display(),
                                                            file.path()
                                                                .parent()
                                                                .unwrap_or(&ROOT_DIR)
                                                                .to_string_lossy());
                                         }
                                     }
                                 })
                             })
                             .collect::<FuturesUnordered<_>>();
                  try_join_all(files).await
              });

        rt.block_on(async {
              if BUILD_DIR.join(".vscode").exists() {
                  tokio::fs::remove_dir_all(BUILD_DIR.join(".vscode").as_path())
                    .await
                    .with_context(|| {
                        format!("Could not delete {}", BUILD_DIR.join(".vscode").display())
                    })
                    .unwrap();
              }

              if BUILD_DIR.join(LIB_DIR.display().to_string()).exists() {
                  tokio::fs::remove_dir_all(BUILD_DIR.join(LIB_DIR.display().to_string())
                                                     .as_path()).await
                                                                .with_context(|| {
                                                                    format!(
                            "Could not delete {}",
                            BUILD_DIR.join(LIB_DIR.display().to_string()).display()
                        )
                                                                })
                                                                .unwrap();
              }
              let handles = FuturesUnordered::from_iter(vec![handle1, handle2]);
              try_join_all(handles).await
          })?
          .into_iter()
          .collect::<Result<Vec<Vec<()>>, JoinError>>()?;

        tracing::info!("This is information an instructor can use to help you, please don't try \
                        to interpret it yourself or make any changes to your submission based on \
                        it.");
        Ok(())
    }
}

    </file-contents>
    <file-contents path="./src/java.rs" name="java.rs">
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    fmt::Formatter,
    hash::{Hash, Hasher},
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

use anyhow::{anyhow, bail, Context, Result};
use futures::{
    future::{join_all, try_join_all},
    stream::FuturesUnordered,
};
use rhai::Array;
// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
use rhai::{CustomType, EvalAltResult};
use serde::{Deserialize, Serialize};
use snailquote::unescape;
use tokio::io::AsyncWriteExt;
use tree_sitter::{Query, QueryCursor, Tree};
use umm_derive::generate_rhai_variant;

use crate::{
    constants::*,
    grade::{JavacDiagnostic, LineRef},
    parsers::parser,
    util::*,
    vscode::{self},
    Dict,
};

/// Types of Java files -
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileType {
    /// - Interface
    Interface,
    /// - Class
    Class,
    /// - Class with a main method
    ClassWithMain,
    /// - JUnit test class
    Test,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Struct representing a java file
pub struct File {
    /// path to java file.
    path:         PathBuf,
    /// name of file.
    file_name:    String,
    /// package the java file belongs to.
    package_name: Option<String>,
    /// imports made by the java file.
    imports:      Option<Vec<Dict>>,
    /// name of the file TODO: How does this differ from `file_name`?
    name:         String,
    /// colored terminal string representing java file name.
    proper_name:  String,
    /// Name of tests methods in this file, as understood by JUnit.
    test_methods: Vec<String>,
    /// Name of tests methods in this file, colored using terminal color codes.
    kind:         FileType,
    #[serde(skip)]
    /// The parser for this file
    parser:       Parser,
    /// Concise description of the file
    description:  String,
}

/// Two `File`s are equal if their paths are equal
impl PartialEq for File {
    fn eq(&self,
          other: &Self)
          -> bool {
        self.path == other.path
    }
}

/// Based on PartialEq
impl Eq for File {}

/// Hash based on path
impl Hash for File {
    fn hash<H: Hasher>(&self,
                       state: &mut H) {
        self.path.hash(state);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
/// Struct representing a Java project.
/// Any index `i` in any collection in this struct always refers to the same
/// JavaFile.
pub struct Project {
    /// Collection of java files in this project
    files:      Vec<File>,
    /// Names of java files in this project.
    names:      Vec<String>,
    /// Classpath
    classpath:  Vec<String>,
    /// Source path
    sourcepath: Vec<String>,
    /// Root directory
    root_dir:   String,
}

#[derive(Clone)]
/// A struct that wraps a tree-sitter parser object and source code
pub struct Parser {
    /// the source code being parsed
    code:  String,
    /// the parse tree
    _tree: Option<Tree>,
    /// the tree-sitter java grammar language
    lang:  tree_sitter::Language,
}

impl Default for Parser {
    fn default() -> Self {
        let mut parser = tree_sitter::Parser::new();
        let code = String::new();
        parser.set_language(&tree_sitter_java::language())
              .expect("Error loading Java grammar");
        let tree = parser.parse(code, None);

        Self { code:  String::new(),
               _tree: tree,
               lang:  tree_sitter_java::language(), }
    }
}

impl std::fmt::Debug for Parser {
    fn fmt(&self,
           _: &mut Formatter<'_>)
           -> std::fmt::Result {
        Ok(())
    }
}

impl Parser {
    #[generate_rhai_variant(Impl, Fallible)]
    /// Returns a new parser object
    ///
    /// * `source_code`: the source code to be parsed
    /// * `lang`: the tree-sitter grammar to use
    pub fn new(source_code: String) -> Result<Self> {
        let mut parser = tree_sitter::Parser::new();

        parser.set_language(&tree_sitter_java::language())
              .expect("Error loading Java grammar");
        let tree = parser.parse(source_code.clone(), None)
                         .context("Error parsing Java code")?;

        Ok(Self { code:  source_code,
                  _tree: Some(tree),
                  lang:  tree_sitter_java::language(), })
    }

    /// A getter for parser's source code
    pub fn code(&mut self) -> String {
        self.code.clone()
    }

    /// A setter for parser's source code
    pub fn set_code(&mut self,
                    code: String) {
        self.code = code;
    }

    #[generate_rhai_variant(Fallible, Mut)]
    /// Applies a tree sitter query and returns the result as a collection of
    /// HashMaps
    ///
    /// * `q`: the tree-sitter query to be applied
    pub fn query(&self,
                 q: &str)
                 -> Result<Vec<Dict>> {
        let mut results = vec![];
        let tree = self._tree
                       .as_ref()
                       .context("Treesitter could not parse code")?;

        let query = Query::new(&self.lang, q).unwrap();
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), self.code.as_bytes());
        let capture_names = query.capture_names();

        for m in matches {
            let mut result = Dict::new();

            for name in capture_names {
                let index = query.capture_index_for_name(name);
                let index = match index {
                    Some(i) => i,
                    None => bail!("Error while querying source code. Capture name: {} has no \
                                   index associated.",
                                  name),
                };

                let value = m.captures.iter().find(|c| c.index == index);
                let value = match value {
                    Some(v) => v,
                    None => continue,
                };

                let value = value.node
                                 .utf8_text(self.code.as_bytes())
                                 .with_context(|| {
                                     format!("Cannot match query result indices with source code \
                                              for capture name: {name}.")
                                 })?;

                result.insert(name.to_string(), value.to_string());
            }
            results.push(result);
        }

        Ok(results)
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
impl CustomType for Parser {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("JavaParser")
               .with_fn("new_java_parser", Parser::new_script)
               .with_fn("code", Parser::code)
               .with_fn("set_code", Parser::set_code)
               .with_fn("query", Parser::query_mut_script);
    }
}

/// An enum to represent possible errors with a Java file
#[derive(thiserror::Error, Debug)]
pub enum JavaFileError {
    /// An error while compiling a Java file (running
    /// [fn@crate::java::File::check])
    #[error("Something went wrong while compiling the Java file")]
    DuringCompilation {
        /// javac stacktrace
        stacktrace: String,
        /// javac stacktrace, parsed with
        /// [fn@crate::parsers::parser::parse_diag]
        diags:      Vec<JavacDiagnostic>,
    },
    /// An error while running a Java file (running
    /// [fn@crate::java::File::run])
    #[error("Something went wrong while running the Java file")]
    AtRuntime {
        /// java output
        output: String,
        /// java stacktrace, parsed with [parser::junit_stacktrace_line_ref]
        diags:  Vec<LineRef>,
    },
    /// An error while testing a Java file (running
    /// [fn@crate::java::File::test])
    #[error("Something went wrong while testing the Java file")]
    FailedTests {
        /// junit test results
        test_results: String,
        /// junit stacktrace, parsed with [parser::junit_stacktrace_line_ref]
        diags:        Vec<LineRef>,
    },
    /// Unknown error
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

impl File {
    #[generate_rhai_variant(Impl, Fallible)]
    /// Creates a new `File` from `path`
    ///
    /// * `path`: the path to read and try to create a File instance for.
    fn new(path: PathBuf) -> Result<Self> {
        let parser = {
            let source_code = std::fs::read_to_string(&path).with_context(|| {
                                                                format!("Could not read file: {:?}",
                                                                        &path)
                                                            })?;
            Parser::new(source_code)?
        };

        let imports = {
            let imports = parser.query(IMPORT_QUERY)?;
            if imports.is_empty() {
                None
            } else {
                Some(imports)
            }
        };

        let package_name = {
            let package_name = parser.query(PACKAGE_QUERY)?;

            if package_name.is_empty() {
                None
            } else {
                package_name[0].get("name").map(String::to_owned)
            }
        };

        let (kind, name) = 'outer: {
            let work = vec![(FileType::Interface, INTERFACENAME_QUERY),
                            (FileType::ClassWithMain, CLASSNAME_QUERY),
                            (FileType::Class, CLASSNAME_QUERY),];
            for (kind, query) in work {
                let result = parser.query(query)?;

                if !result.is_empty() {
                    break 'outer (kind,
                                  #[allow(clippy::or_fun_call)]
                                  result.first()
                                        .ok_or(anyhow!("Could not find a valid class/interface \
                                                        declaration for {} (vec size is 0)",
                                                       path.display()))?
                                        .get("name")
                                        .ok_or(anyhow!("Could not find a valid class/interface \
                                                        declaration for {} (hashmap has no name \
                                                        key) ",
                                                       path.display()))?
                                        .to_string());
                }
            }

            (FileType::Class, String::new())
        };

        let proper_name = if package_name.is_some() {
            format!("{}.{}", package_name.as_ref().unwrap(), name)
        } else {
            name.clone()
        };

        let test_methods = {
            let test_methods = parser.query(TEST_ANNOTATION_QUERY)?;
            let mut tests = vec![];
            for t in test_methods {
                if let Some(t) = t.get("name") {
                    tests.push(format!("{}#{}", &proper_name, t));
                }
            }

            tests
        };

        let kind = if !test_methods.is_empty() {
            FileType::Test
        } else {
            kind
        };

        let description = match kind {
            FileType::Interface => {
                let empty_dict = Dict::new();
                let empty = String::new();
                let not_found = String::from("[NOT FOUND]");

                let query_result = parser.query(INTERFACE_DECLARATION_QUERY)
                                         .unwrap_or_default();
                let declaration = query_result.first().unwrap_or(&empty_dict);

                let parameters = declaration.get("parameters").unwrap_or(&empty).trim();
                let extends = declaration.get("extends").unwrap_or(&empty).trim();

                let consts = parser.query(INTERFACE_CONSTANTS_QUERY)
                                   .unwrap_or_default()
                                   .iter()
                                   .map(|c| c.get("constant").unwrap_or(&not_found).to_string())
                                   .collect::<Vec<String>>()
                                   .join("\n");

                let methods = parser.query(INTERFACE_METHODS_QUERY)
                                    .unwrap_or_default()
                                    .iter()
                                    .map(|m| m.get("signature").unwrap_or(&not_found).to_string())
                                    .collect::<Vec<String>>()
                                    .join("\n");

                let methods = if methods.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    methods.trim().to_string()
                };

                let consts = if consts.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    consts.trim().to_string()
                };

                let mut result = vec![];
                result.push(format!("Interface: {proper_name} {parameters} {extends}:\n"));

                if !consts.contains("NOT FOUND") {
                    result.push(String::from("Constants:"));
                    result.push(consts);
                }
                if !methods.contains("NOT FOUND") {
                    result.push(String::from("Methods:"));
                    result.push(methods);
                }

                format!("```\n{r}\n```", r = result.join("\n"))
            }
            _ => {
                let empty_dict = Dict::new();
                let empty = String::new();
                let not_found = String::from("[NOT FOUND]");

                let query_result = parser.query(CLASS_DECLARATION_QUERY).unwrap_or_default();
                let declaration = query_result.first().unwrap_or(&empty_dict);

                let parameters = declaration.get("typeParameters").unwrap_or(&empty).trim();
                let implements = declaration.get("interfaces").unwrap_or(&empty).trim();

                let fields = parser.query(CLASS_FIELDS_QUERY)
                                   .unwrap_or_default()
                                   .iter()
                                   .map(|f| f.get("field").unwrap_or(&not_found).trim().to_string())
                                   .collect::<Vec<String>>()
                                   .join(", ");

                let methods = parser.query(CLASS_METHOD_QUERY)
                                    .unwrap_or_default()
                                    .iter()
                                    .map(|m| {
                                        let identifier =
                                            m.get("identifier").unwrap_or(&not_found).trim();
                                        let parameters = m.get("parameters").unwrap_or(&empty);

                                        if identifier == not_found.as_str() {
                                            "[NOT FOUND]".to_string()
                                        } else {
                                            format!("{}{}", identifier.trim(), parameters.trim())
                                        }
                                    })
                                    .collect::<Vec<String>>()
                                    .join(", ");

                let constructors =
                    parser.query(CLASS_CONSTRUCTOR_QUERY)
                          .unwrap_or_default()
                          .iter()
                          .map(|m| {
                              let identifier = m.get("identifier").unwrap_or(&not_found).trim();
                              let parameters = m.get("parameters").unwrap_or(&empty);

                              if identifier == not_found.as_str() {
                                  "[NOT FOUND]".to_string()
                              } else {
                                  format!("{}{}", identifier.trim(), parameters.trim())
                              }
                          })
                          .collect::<Vec<String>>()
                          .join(", ");

                let fields = if fields.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    format!("\tFields: {}", fields)
                };

                let methods = if methods.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    format!("\tMethods: {}", methods)
                };

                let constructors = if constructors.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    format!("\tConstructors: {}", constructors)
                };

                let mut result = vec![];
                result.push(format!("Class: {proper_name} {parameters} {implements}:\n"));

                if !fields.contains("NOT FOUND") {
                    result.push(fields);
                }
                if !constructors.contains("NOT FOUND") {
                    result.push(constructors);
                }
                if !methods.contains("NOT FOUND") {
                    result.push(methods);
                }

                result.join("\n")
            }
        };

        Ok(Self { path: path.to_owned(),
                  file_name: path.file_name().unwrap().to_str().unwrap().to_string(),
                  package_name,
                  imports,
                  name,
                  test_methods,
                  kind,
                  proper_name,
                  parser,
                  description })
    }

    /// Returns the inner doc check of this [`File`].
    fn inner_doc_check(&self,
                       err: Stdio,
                       out: Stdio,
                       in_: Stdio)
                       -> Result<Output> {
        Command::new(javac_path()?).stderr(err)
                                   .stdout(out)
                                   .stdin(in_)
                                   .args(["--source-path",
                                          sourcepath()?.as_str(),
                                          "-g",
                                          "--class-path",
                                          classpath()?.as_str(),
                                          "-d",
                                          BUILD_DIR.to_str().unwrap(),
                                          self.path.as_path().to_str().unwrap(),
                                          "-Xdiags:verbose",
                                          "-Xdoclint"
                                          /* "-Xlint", */])
                                   .output()
                                   .context("Failed to spawn javac process.")
    }

    /// Utility method to ask javac for documentation lints using the -Xdoclint
    /// flag.
    ///
    /// The method simply returns the output produced by javac as a String.
    /// There is a ['parse_diag method'][fn@crate::parsers::parser::parse_diag]
    /// that can parse these to yield useful information.
    pub fn doc_check(&self) -> Result<String, JavaFileError> {
        let child = self.inner_doc_check(Stdio::piped(), Stdio::piped(), Stdio::piped())?;

        let output = unescape(
            &[
                String::from_utf8(child.stderr).context("Error when parsing stderr as utf8")?,
                String::from_utf8(child.stdout).context("Error when parsing stdout as utf8")?,
            ]
            .concat(),
        )
        .context("Error when un-escaping javac output.")?;

        Ok(output)
    }

    /// Utility method to ask javac for documentation lints using the -Xdoclint
    /// flag.
    ///
    /// The method simply returns the output produced by javac as a String.
    /// There is a ['parse_diag method'][fn@crate::parsers::parser::parse_diag]
    /// that can parse these to yield useful information.
    pub fn doc_check_mut_script(&self) -> Result<String, Box<EvalAltResult>> {
        match self.inner_doc_check(Stdio::inherit(), Stdio::inherit(), Stdio::inherit()) {
            Ok(child) => match unescape(&[match String::from_utf8(child.stderr) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          },
                                          match String::from_utf8(child.stdout) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          }].concat())
            {
                Ok(s) => Ok(s),
                Err(e) => Err(format!("{}", e).into()),
            },
            Err(e) => {
                Err(Box::new(unescape(e.to_string().as_str()).unwrap_or_else(|_| {
                                                                 format!("Could not unescape: {:?}",
                                                                         e)
                                                             })
                                                             .into()))
            }
        }
    }

    /// Returns the inner check of this [`File`].
    fn inner_check(&self,
                   err: Stdio,
                   out: Stdio,
                   in_: Stdio)
                   -> Result<Output> {
        let path = self.path.display().to_string();

        Command::new(javac_path()?).stderr(err)
                                   .stdout(out)
                                   .stdin(in_)
                                   .args(["--source-path",
                                          sourcepath()?.as_str(),
                                          "-g",
                                          "--class-path",
                                          classpath()?.as_str(),
                                          "-d",
                                          BUILD_DIR.to_str().unwrap(),
                                          path.as_str(),
                                          "-Xdiags:verbose",
                                          // "-Xlint",
                                          "-Xprefer:source"])
                                   .output()
                                   .context("Failed to spawn javac process.")
    }

    /// Utility method to check for syntax errors using javac flag.
    pub fn check(&self) -> Result<String, JavaFileError> {
        match self.inner_check(Stdio::piped(), Stdio::piped(), Stdio::piped()) {
            Ok(out) => {
                let output = unescape(
                    &[
                        String::from_utf8(out.stderr).context("Error parsing stderr as utf8")?,
                        String::from_utf8(out.stdout).context("Error parsing stdout as utf8")?,
                    ]
                    .concat(),
                )
                .context("Error when un-escaping javac output.")?;

                if out.status.success() {
                    Ok(output)
                } else {
                    let mut diags = Vec::new();
                    for line in output.lines() {
                        if let Ok(diag) = parser::parse_diag(line) {
                            diags.push(diag);
                        }
                    }

                    Err(JavaFileError::DuringCompilation { stacktrace: output,
                                                           diags })
                }
            }
            Err(e) => Err(JavaFileError::Unknown(e)),
        }
    }

    /// Utility method to check for syntax errors using javac flag.
    pub fn check_mut_script(&self) -> Result<String, Box<EvalAltResult>> {
        match self.inner_check(Stdio::inherit(), Stdio::inherit(), Stdio::inherit()) {
            Ok(child) => match unescape(&[match String::from_utf8(child.stderr) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          },
                                          match String::from_utf8(child.stdout) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          }].concat())
            {
                Ok(s) => Ok(s),
                Err(e) => Err(format!("{}", e).into()),
            },
            Err(e) => {
                Err(Box::new(unescape(e.to_string().as_str()).unwrap_or_else(|_| {
                                                                 format!("Could not unescape: {:?}",
                                                                         e)
                                                             })
                                                             .into()))
            }
        }
    }

    /// Returns the inner run of this [`File`].
    fn inner_run(&self,
                 input: Option<String>,
                 err: Stdio,
                 out: Stdio)
                 -> Result<Output> {
        if self.kind != FileType::ClassWithMain {
            Err(JavaFileError::DuringCompilation { stacktrace: "The file you wish to run does not \
                                                                have a main method."
                                                                                    .into(),
                                                   diags:      vec![], })?;
        }

        if let Some(input_str) = input {
            let mut child = Command::new(java_path()?).args(["--class-path",
                                                             classpath()?.as_str(),
                                                             self.proper_name.clone().as_str()])
                                                      .stdin(Stdio::piped())
                                                      .stdout(out)
                                                      .stderr(err)
                                                      .spawn()
                                                      .context("Failed to spawn javac process.")?;

            let input = format!("{}\r\n", input_str);

            let mut stdin = child.stdin.take().unwrap();

            stdin.write_all(input.as_bytes())
                 .context("Error when trying to write input to stdin")?;
            stdin.flush().context("Error when trying to flush stdin")?;

            child.wait_with_output()
                 .context("Error when waiting for child process to finish")
        } else {
            Command::new(java_path()?).args(["--class-path",
                                             classpath()?.as_str(),
                                             self.proper_name.clone().as_str()])
                                      .stdin(Stdio::inherit())
                                      .stdout(out)
                                      .stderr(err)
                                      .spawn()?
                                      .wait_with_output()
                                      .context("Failed to spawn javac process.")
        }
    }

    /// Utility method to run a java file that has a main method.
    pub fn run(&self,
               input: Option<String>)
               -> Result<String, JavaFileError> {
        self.check()?;

        match self.inner_run(input, Stdio::piped(), Stdio::piped()) {
            Ok(out) => {
                let output = unescape(
                    &[
                        String::from_utf8(out.stderr)
                            .context("Error when parsing stderr as utf8")?,
                        String::from_utf8(out.stdout)
                            .context("Error when parsing stdout as utf8")?,
                    ]
                    .concat(),
                )
                .context("Error when escaping java output.")?;

                if out.status.success() {
                    Ok(output)
                } else {
                    let mut diags = Vec::new();

                    for line in output.lines() {
                        if let Ok(diag) = parser::junit_stacktrace_line_ref(line) {
                            diags.push(diag);
                        }
                    }

                    Err(JavaFileError::AtRuntime { output, diags })
                }
            }
            Err(e) => Err(anyhow!(e).into()),
        }
    }

    /// Utility method to run a java file that has a main method.
    pub fn run_mut_script(&self,
                          input: Option<String>)
                          -> Result<String, Box<EvalAltResult>> {
        match self.inner_run(input, Stdio::inherit(), Stdio::inherit()) {
            Ok(child) => match unescape(&[match String::from_utf8(child.stderr) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          },
                                          match String::from_utf8(child.stdout) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          }].concat())
            {
                Ok(s) => Ok(s),
                Err(e) => Err(format!("{}", e).into()),
            },
            Err(e) => {
                Err(Box::new(unescape(e.to_string().as_str()).unwrap_or_else(|_| {
                                                                 format!("Could not unescape: {:?}",
                                                                         e)
                                                             })
                                                             .into()))
            }
        }
    }

    /// Inner method to run tests.
    fn inner_test(&self,
                  tests: Vec<&str>,
                  err: Stdio,
                  out: Stdio,
                  in_: Stdio)
                  -> Result<Output> {
        let tests = {
            let mut new_tests = Vec::<String>::new();
            for t in tests {
                new_tests.push(format!("{}#{}", self.proper_name.clone(), t));
            }

            if new_tests.is_empty() {
                self.test_methods.clone()
            } else {
                new_tests
            }
        };

        let tests = tests.iter()
                         .map(|s| format!("-m{s}"))
                         .collect::<Vec<String>>();
        let methods: Vec<&str> = tests.iter().map(String::as_str).collect();

        Command::new(java_path().context("Could not find `java` command on path.")?)
            .stderr(err)
            .stdout(out)
            .stdin(in_)
            .args(
                [
                    [
                        "-jar",
                        LIB_DIR.join(JUNIT_PLATFORM).as_path().to_str().unwrap(),
                        "--disable-banner",
                        "--disable-ansi-colors",
                        "--details-theme=unicode",
                        "--single-color",
                        "-cp",
                        &classpath()?,
                    ]
                    .as_slice(),
                    methods.as_slice(),
                ]
                .concat(),
            )
            .output()
            .context("Failed to spawn javac process.")
    }

    /// A utility method that takes a list of strings (or types that implement
    /// `Into<String>`) meant to represent test method names, and runs those
    /// tests.
    ///
    /// Returns the output from JUnit as a string. There are parsers in
    /// ['parsers module'][crate::parsers::parser] that helps parse this output.
    ///
    /// * `tests`: list of strings (or types that implement `Into<String>`)
    ///   meant to represent test method names,
    pub fn test(&self,
                tests: Vec<&str>,
                project: Option<&Project>)
                -> Result<String, JavaFileError> {
        self.check()?;

        match self.inner_test(tests, Stdio::piped(), Stdio::piped(), Stdio::inherit()) {
            Ok(out) => {
                let output = unescape(
                    &[
                        String::from_utf8(out.stderr)
                            .context("Error when parsing stderr as utf8")?,
                        String::from_utf8(out.stdout)
                            .context("Error when parsing stdout as utf8")?,
                    ]
                    .concat(),
                )
                .context("Error when un-escaping JUnit output.")?;

                if out.status.success() {
                    Ok(output)
                } else {
                    let mut diags = Vec::new();
                    let mut new_output = Vec::new();

                    for line in output.lines() {
                        if line.contains("MethodSource") || line.contains("Native Method") {
                            continue;
                        }

                        // if line.contains("Test run finished after") {
                        //     break;
                        // }

                        if let Ok(diag) = parser::junit_stacktrace_line_ref(line) {
                            if let Some(proj) = project
                               && proj.identify(diag.file_name()).is_ok()
                            {
                                new_output.push(line.replace("\\\\", "\\")
                                                    .replace("\\\"", "\"")
                                                    .to_string());
                            }
                            diags.push(diag);
                        } else if let Ok(diag) = parser::parse_diag(line) {
                            if let Some(proj) = project
                               && proj.identify(diag.file_name()).is_ok()
                            {
                                new_output.push(line.replace("\\\\", "\\")
                                                    .replace("\\\"", "\"")
                                                    .to_string());
                            }
                            diags.push(diag.into());
                        } else {
                            new_output.push(line.replace("\\\\", "\\")
                                                .replace("\\\"", "\"")
                                                .to_string());
                        }
                    }

                    Err(JavaFileError::FailedTests { test_results: new_output.join("\n"),
                                                     diags })
                }
            }
            Err(e) => Err(anyhow!(e).into()),
        }
    }

    /// A utility method that takes a list of strings (or types that implement
    /// `Into<String>`) meant to represent test method names, and runs those
    /// tests.
    pub fn test_mut_script(&mut self,
                           tests: Vec<&str>)
                           -> Result<String, Box<EvalAltResult>> {
        match self.inner_test(tests, Stdio::inherit(), Stdio::inherit(), Stdio::inherit()) {
            Ok(child) => match unescape(&[match String::from_utf8(child.stderr) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          },
                                          match String::from_utf8(child.stdout) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          }].concat())
            {
                Ok(s) => Ok(s),
                Err(e) => Err(format!("{}", e).into()),
            },
            Err(e) => {
                Err(Box::new(unescape(e.to_string().as_str()).unwrap_or_else(|_| {
                                                                 format!("Could not unescape: {:?}",
                                                                         e)
                                                             })
                                                             .into()))
            }
        }
    }

    /// A utility method that takes a list of strings (or types that implement
    /// `Into<String>`) meant to represent test method names, and runs those
    /// tests.
    ///
    /// Returns the output from JUnit as a string. There are parsers in
    /// ['parsers module'][crate::parsers::parser] that helps parse this output.

    /// Get a reference to the file's kind.
    pub fn kind(&self) -> &FileType {
        &self.kind
    }

    /// Get a reference to the file's file name.
    pub fn file_name(&self) -> &str {
        self.file_name.as_ref()
    }

    /// Get a reference to the file's test methods.
    pub fn test_methods(&self) -> Vec<String> {
        self.test_methods.clone()
    }

    /// Get a reference to the file's test methods.
    pub fn test_methods_mut_script(&mut self) -> Array {
        self.test_methods().iter().map(|s| s.into()).collect()
    }

    /// treesitter query for this file
    pub fn query(&self,
                 q: &str)
                 -> Result<Vec<Dict>> {
        self.parser.query(q)
    }

    /// treesitter query for this file
    pub fn query_mut_script(&mut self,
                            q: &str)
                            -> Result<Array, Box<EvalAltResult>> {
        match self.parser.query(q) {
            Ok(v) => {
                let mut arr = Array::new();
                for d in v {
                    arr.push(d.into());
                }
                Ok(arr)
            }
            Err(e) => Err(format!("Failed to query file: {e}").into()),
        }
    }

    /// Get a reference to the file's path.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get a reference to the file's path.
    pub fn path_mut_script(&mut self) -> String {
        self.path.display().to_string()
    }

    /// Get a reference to the file's proper name.
    pub fn package_name(&self) -> Option<&String> {
        self.package_name.as_ref()
    }

    /// Get a reference to the file's parser.
    pub fn parser(&self) -> Parser {
        self.parser.clone()
    }

    /// Get a reference to the file's description.
    pub fn description(&self) -> String {
        self.description.clone()
    }

    /// Get the file's proper name.
    pub fn proper_name(&self) -> String {
        self.proper_name.clone()
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
impl CustomType for File {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("JavaFile")
               .with_fn("new_java_file", File::new_script)
               .with_fn("check", File::check_mut_script)
               .with_fn("doc_check", File::doc_check_mut_script)
               .with_fn("run", File::run_mut_script)
               .with_fn("test", File::test_mut_script)
               .with_fn("kind", File::kind)
               .with_fn("file_name", File::file_name)
               .with_fn("test_methods", File::test_methods_mut_script)
               .with_fn("query", File::query_mut_script)
               .with_fn("package_name", File::package_name)
               .with_fn("path", File::path_mut_script)
               .with_fn("parser", File::parser);
    }
}

impl Project {
    #[generate_rhai_variant(Impl, Fallible)]
    /// Initializes a Project, by discovering java files in the
    /// [struct@UMM_DIR] directory. Also downloads some `jar`
    /// files required for unit testing and mutation testing.
    pub fn new() -> Result<Self> {
        let mut files = vec![];
        let mut names = vec![];

        let rt = RUNTIME.handle().clone();
        let handles = FuturesUnordered::new();

        let results = rt.block_on(async {
                            let found_files = match find_files("java", 15, &ROOT_DIR) {
                                Ok(f) => f,
                                Err(e) => panic!("Could not find java files: {e}"),
                            };

                            for path in found_files {
                                handles.push(rt.spawn_blocking(|| File::new(path)))
                            }

                            join_all(handles).await
                        });

        for result in results {
            let file = result??;
            names.push(file.proper_name.clone());
            files.push(file);
        }

        let classpath = vec![LIB_DIR.join("*.jar").display().to_string()];

        let mut sourcepath = vec![SOURCE_DIR.join("").display().to_string(),
                                  TEST_DIR.join("").display().to_string(),];

        if !find_files("java", 0, &ROOT_DIR)?.is_empty() {
            sourcepath.push(ROOT_DIR.join("").display().to_string());
        }

        let proj = Self { files,
                          names,
                          classpath,
                          sourcepath,
                          root_dir: ROOT_DIR.display().to_string() };

        let _guard = rt.enter();
        rt.block_on(async {
              let handles = FuturesUnordered::new();
              let (proj1, proj2, proj3) = (proj.clone(), proj.clone(), proj.clone());

              handles.push(tokio::spawn(async move { proj1.download_libraries_if_needed().await }));
              handles.push(tokio::spawn(async move { proj2.update_vscode_settings().await }));
              handles.push(tokio::spawn(async move { proj3.update_vscode_tasks().await }));

              try_join_all(handles).await
          })?
          .into_iter()
          .collect::<Result<Vec<()>>>()?;

        Ok(proj)
    }

    #[generate_rhai_variant(Impl, Mut, Fallible)]
    /// Attempts to identify the correct file from the project from a partial or
    /// fully formed name as expected by a java compiler.
    ///
    /// Returns a reference to the identified file, if any.
    ///
    /// * `name`: partial/fully formed name of the Java file to look for.
    pub fn identify(&self,
                    name: &str)
                    -> Result<File> {
        let name: String = name.into();

        if let Some(i) = self.names.iter().position(|n| *n == name) {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files.iter().position(|n| n.file_name == name) {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files
                                    .iter()
                                    .position(|n| n.file_name.replace(".java", "") == name)
        {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files.iter().position(|n| n.name.clone() == name) {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files
                                    .iter()
                                    .position(|n| n.path.display().to_string() == name)
        {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files.iter().position(|n| n.proper_name == name) {
            Ok(self.files[i].clone())
        } else {
            bail!("Could not find {} in the project", name)
        }
    }

    /// Returns true if project contains a file with the given name.
    pub fn contains(&self,
                    name: &str)
                    -> bool {
        self.identify(name).is_ok()
    }

    /// Downloads certain libraries like JUnit if found in imports.
    /// times out after 20 seconds.
    pub async fn download_libraries_if_needed(&self) -> Result<()> {
        let need_junit = 'outer: {
            for file in self.files.iter() {
                if let Some(imports) = &file.imports {
                    for import in imports {
                        if let Some(path) = import.get(&String::from("path")) {
                            if path.starts_with("org.junit") {
                                break 'outer true;
                            }
                        }
                    }
                }
            }
            false
        };

        if need_junit {
            if !LIB_DIR.as_path().is_dir() {
                std::fs::create_dir(LIB_DIR.as_path()).unwrap();
            }

            let handle1 = tokio::spawn(async {
                download(
                    "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/junit-platform-console-standalone-1.10.2.jar",
                    &LIB_DIR.join(JUNIT_PLATFORM),
                false
                        )
                        .await
            });

            let handle2 = tokio::spawn(async {
                download("https://ummfiles.fra1.digitaloceanspaces.com/jar_files/junit-4.13.2.jar",
                         &LIB_DIR.join("junit-4.13.2.jar"),
                         false).await
            });

            let handle3 = tokio::spawn(async {
                download("https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-1.16.1.jar",
                         &LIB_DIR.join("pitest.jar"),
                         false).await
            });

            let handle4 = tokio::spawn(async {
                download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-command-line-1.16.1.jar",
                        &LIB_DIR.join("pitest-command-line.jar"),
                        false,
                    )
                    .await
            });

            let handle5 = tokio::spawn(async {
                download(
                    "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-entry-1.16.1.jar",
                    &LIB_DIR.join("pitest-entry.jar"),
                    false,
                )
                .await
            });

            let handle6 = tokio::spawn(async {
                download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-junit5-plugin-1.2.1.jar",
                        &LIB_DIR.join("pitest-junit5-plugin.jar"),
                        false,
                    )
                    .await
            });

            let handle7 = tokio::spawn(async {
                download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/commons-text-1.12.0.jar",
                        &LIB_DIR.join("commons-text-1.12.0.jar"),
                        false,
                    )
                    .await
            });

            let handle8 = tokio::spawn(async {
                download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/commons-lang3-3.14.0.jar",
                        &LIB_DIR.join("commons-lang3-3.14.0.jar"),
                        false,
                    )
                    .await
            });

            let handles = FuturesUnordered::from_iter([handle1, handle2, handle3, handle4,
                                                       handle5, handle6, handle7, handle8]);

            futures::future::try_join_all(handles).await?;
        }
        Ok(())
    }

    /// Creates a vscode settings.json file for the project.
    pub async fn update_vscode_settings(&self) -> Result<()> {
        // TODO: Move this to an init function that takes CONSTANTS into account
        if !ROOT_DIR.join(".vscode").as_path().is_dir() {
            tokio::fs::create_dir(ROOT_DIR.join(".vscode").as_path()).await
                                                                     .unwrap();
        }

        if !ROOT_DIR.join(".vscode/settings.json").as_path().exists() {
            let mut file = tokio::fs::OpenOptions::new().write(true)
                                                        .truncate(true)
                                                        .create(true)
                                                        .open(ROOT_DIR.join(".vscode")
                                                                      .join("settings.json")
                                                                      .as_path())
                                                        .await?;

            let settings =
                vscode::SettingsFile::builder().java_source_path(self.sourcepath.clone())
                                               .java_output_path(BUILD_DIR.join("")
                                                                          .display()
                                                                          .to_string())
                                               .java_referenced_libs(self.classpath.clone())
                                               .umm_binary_path(umm_path())
                                               .build();

            file.write_all(serde_json::to_string_pretty(&settings)?.as_bytes())
                .await?;
        }

        // Do the same for extensions.json
        if !ROOT_DIR.join(".vscode/extensions.json").as_path().exists() {
            let mut file = tokio::fs::OpenOptions::new().write(true)
                                                        .truncate(true)
                                                        .create(true)
                                                        .open(ROOT_DIR.join(".vscode")
                                                                      .join("extensions.json")
                                                                      .as_path())
                                                        .await?;

            let extensions = r#"
{
	// See https://go.microsoft.com/fwlink/?LinkId=827846 to learn about workspace recommendations.
	// Extension identifier format: ${publisher}.${name}. Example: vscode.csharp

	// List of extensions which should be recommended for users of this workspace.
	"recommendations": [
        "vscjava.vscode-java-pack",
        "ms-vsliveshare.vsliveshare"
	],
	// List of extensions recommended by VS Code that should not be recommended for users of this workspace.
	"unwantedRecommendations": [
		
	]
}
            "#;

            file.write_all(extensions.as_bytes()).await?;
        }

        Ok(())
    }

    /// Get a reference to the project's files.
    pub fn files(&self) -> &[File] {
        self.files.as_ref()
    }

    #[generate_rhai_variant(Fallible)]
    /// Prints project struct as a json
    pub fn info(&self) -> Result<()> {
        println!("{}", serde_json::to_string(&self)?);
        Ok(())
    }

    /// Returns a short summary of the project, it's files, their fields and
    /// methods.
    pub fn describe(&self) -> String {
        let mut result = String::new();
        result.push_str("> What follows is a summary of the student's submission's files, their \
                         fields and methods generated via treesitter queries.\n\n");

        for f in self.files.iter() {
            if f.proper_name.contains("Hidden") {
                continue;
            }
            result.push_str(f.description().as_str());
            result.push_str("\n\n");
        }

        result
    }

    /// Writes a .vscode/tasks.json file for the project.
    pub async fn update_vscode_tasks(&self) -> Result<()> {
        let mut tasks = Vec::new();
        let mut inputs = Vec::new();

        let (default_depends_on, default_depends_order) = if umm_path() == "./umm" {
            (Some(vec!["Set umm to be executable".to_string()]),
             Some(vscode::DependsOrder::Sequence))
        } else {
            (None, None)
        };

        tasks.push(
                   vscode::Task::builder().label("Set umm to be executable".to_string())
                                          .r#type(vscode::Type::Shell)
                                          .command("chmod")
                                          .args(vec![
            vscode::Args::builder().value("+x")
                                   .quoting(vscode::ArgQuoting::Escape)
                                   .build(),
            vscode::Args::builder().value("${config:ummBinaryPath}")
                                   .quoting(vscode::ArgQuoting::Weak)
                                   .build(),
        ])
                                          .depends_on(None)
                                          .depends_order(None)
                                          .build(),
        );
        tasks.push(
                   vscode::Task::builder().label("Clean library and target folders".to_string())
                                          .r#type(vscode::Type::Shell)
                                          .command("${config:ummBinaryPath}")
                                          .args(vec![vscode::Args::builder().value("clean")
                                                                            .quoting(
            vscode::ArgQuoting::Escape,
        )
                                                                            .build()])
                                          .depends_on(default_depends_on.clone())
                                          .depends_order(default_depends_order)
                                          .build(),
        );

        tasks.push(
                   vscode::Task::builder().label("Reset project metadata".into())
                                          .r#type(vscode::Type::Shell)
                                          .command("${config:ummBinaryPath}")
                                          .args(vec![vscode::Args::builder().value("reset")
                                                                            .quoting(
            vscode::ArgQuoting::Escape,
        )
                                                                            .build()])
                                          .depends_on(default_depends_on.clone())
                                          .depends_order(default_depends_order)
                                          .build(),
        );

        tasks.push(
                   vscode::Task::builder().label("Check health of the project".into())
                                          .r#type(vscode::Type::Shell)
                                          .command("${config:ummBinaryPath}")
                                          .args(vec![vscode::Args::builder().value("check-health")
                                                                            .quoting(
            vscode::ArgQuoting::Escape,
        )
                                                                            .build()])
                                          .depends_on(default_depends_on.clone())
                                          .depends_order(default_depends_order)
                                          .build(),
        );

        tasks.push(
                   vscode::Task::builder().label("Update umm executable".into())
                                          .r#type(vscode::Type::Shell)
                                          .command("${config:ummBinaryPath}")
                                          .args(vec![vscode::Args::builder().value("update")
                                                                            .quoting(
            vscode::ArgQuoting::Escape,
        )
                                                                            .build()])
                                          .depends_on(default_depends_on.clone())
                                          .depends_order(default_depends_order)
                                          .build(),
        );

        for file in self.files().iter() {
            match file.kind() {
                FileType::ClassWithMain => {
                    tasks.push(
                               vscode::Task::builder().label(format!("Run {}", file.name))
                                                      .r#type(vscode::Type::Shell)
                                                      .command("${config:ummBinaryPath}")
                                                      .args(vec![
                        vscode::Args::builder().value("run")
                                               .quoting(vscode::ArgQuoting::Escape)
                                               .build(),
                        vscode::Args::builder().value(&file.proper_name)
                                               .quoting(vscode::ArgQuoting::Escape)
                                               .build(),
                    ])
                                                      .depends_on(default_depends_on.clone())
                                                      .depends_order(default_depends_order)
                                                      .build(),
                    );
                }
                FileType::Test => {
                    tasks.push(
                               vscode::Task::builder().label(format!(
                        "Run tests for {}",
                        file.name
                    ))
                                                      .r#type(vscode::Type::Shell)
                                                      .command("${config:ummBinaryPath}")
                                                      .args(vec![
                        vscode::Args::builder().value("test")
                                               .quoting(vscode::ArgQuoting::Escape)
                                               .build(),
                        vscode::Args::builder().value(&file.proper_name)
                                               .quoting(vscode::ArgQuoting::Escape)
                                               .build(),
                    ])
                                                      .group("test".to_string())
                                                      .depends_on(default_depends_on.clone())
                                                      .depends_order(default_depends_order)
                                                      .build(),
                    );

                    let mut test_methods = Vec::new();

                    for method in file.test_methods() {
                        let method = method.clone();
                        #[allow(clippy::or_fun_call)]
                        let method =
                            method.split_once('#')
                                  .ok_or(anyhow!("Could not parse test method - {}", method))?
                                  .1;
                        // commands.push(method.into());
                        test_methods.push(String::from(method));
                    }

                    if !test_methods.is_empty() {
                        let input = vscode::Input::PickString { id:          file.proper_name
                                                                                 .to_string(),
                                                                description:
                                                                    "Which test to run?".to_string(),
                                                                options:     test_methods.clone(),
                                                                default:     test_methods.first()
                                                                                         .unwrap()
                                                                                         .clone(), };
                        inputs.push(input);
                    }

                    tasks.push(
                               vscode::Task::builder().label(format!(
                        "Run specific test from {}",
                        file.name
                    ))
                                                      .r#type(vscode::Type::Shell)
                                                      .command("${config:ummBinaryPath}")
                                                      .args(vec![
                        vscode::Args::builder().value("test")
                                               .quoting(vscode::ArgQuoting::Escape)
                                               .build(),
                        vscode::Args::builder().value(&file.proper_name)
                                               .quoting(vscode::ArgQuoting::Escape)
                                               .build(),
                        vscode::Args::builder().value(format!("${{input:{}}}", file.proper_name))
                                               .quoting(vscode::ArgQuoting::Escape)
                                               .build(),
                    ])
                                                      .group("test".to_string())
                                                      .depends_on(default_depends_on.clone())
                                                      .depends_order(default_depends_order)
                                                      .build(),
                    );
                }
                _ => {}
            };

            tasks.push(
                       vscode::Task::builder().label(format!("Check {}", file.name))
                                              .r#type(vscode::Type::Shell)
                                              .command("${config:ummBinaryPath}")
                                              .args(vec![
                vscode::Args::builder().value("check")
                                       .quoting(vscode::ArgQuoting::Escape)
                                       .build(),
                vscode::Args::builder().value(&file.proper_name)
                                       .quoting(vscode::ArgQuoting::Escape)
                                       .build(),
            ])
                                              .depends_on(default_depends_on.clone())
                                              .depends_order(default_depends_order)
                                              .build(),
            );
            tasks.push(
                       vscode::Task::builder().label(format!("Check JavaDoc for {}", file.name))
                                              .r#type(vscode::Type::Shell)
                                              .command("${config:ummBinaryPath}")
                                              .args(vec![
                vscode::Args::builder().value("doc-check")
                                       .quoting(vscode::ArgQuoting::Escape)
                                       .build(),
                vscode::Args::builder().value(&file.proper_name)
                                       .quoting(vscode::ArgQuoting::Escape)
                                       .build(),
            ])
                                              .depends_on(default_depends_on.clone())
                                              .depends_order(default_depends_order)
                                              .build(),
            );
        }

        let rhai_scripts = {
            let scripts = find_files(".rhai", 3, &ROOT_DIR)?.iter()
                                                            .map(|f| f.display().to_string())
                                                            .collect::<Vec<String>>();

            if scripts.is_empty() {
                vec!["script.rhai".to_string()]
            } else {
                scripts
            }
        };

        inputs.push(vscode::Input::PickString { id:          "gradable_assignments".to_string(),
                                          description: "What script to use?".to_string(),
                                          options:     rhai_scripts.clone(),
                                          default:     rhai_scripts.first().unwrap().clone(), });

        tasks.push(
            vscode::Task::builder()
                .label("Grade Assignment".to_string())
                .r#type(vscode::Type::Shell)
                .command("${config:ummBinaryPath}")
                .args(vec![
                    vscode::Args::builder()
                        .value("grade")
                        .quoting(vscode::ArgQuoting::Escape)
                        .build(),
                    vscode::Args::builder()
                        .value("${input:gradable_assignments}".to_string())
                        .quoting(vscode::ArgQuoting::Escape)
                        .build(),
                ])
                .problem_matcher(Some(vec![vscode::ProblemMatcher::builder()
                    .apply_to("allDocuments".to_string())
                    .file_location(vec![
                        "relative".to_string(),
                        "${workspaceFolder}".to_string(),
                    ])
                    .owner("umm".to_string())
                    .pattern(
                        vscode::Pattern::builder()
                            .regexp(r#"\s*[│]\s*([\w./]+)\s*[│]\s*([0-9]+)\s*[│]\s*([\w ]+)"#)
                            .file(1)
                            .line(2)
                            .end_line(2)
                            .message(3)
                            .build(),
                    )
                    .build()]))
                .depends_on(default_depends_on)
                .depends_order(default_depends_order)
                .build(),
        );

        if !ROOT_DIR.join(".vscode").as_path().exists() {
            tokio::fs::create_dir(ROOT_DIR.join(".vscode").as_path()).await
                                                                     .unwrap();
        }

        let mut file =
            tokio::fs::OpenOptions::new().write(true)
                                         .truncate(true)
                                         .create(true)
                                         .open(ROOT_DIR.join(".vscode")
                                                       .join("tasks.json")
                                                       .as_path())
                                         .await?;

        let task_file = vscode::TasksFile::builder().tasks(tasks)
                                                    .inputs(inputs)
                                                    .build();

        file.write_all(serde_json::to_string_pretty(&task_file)?.as_bytes())
            .await?;

        Ok(())
    }

    /// Serves the project code as a static website.
    pub fn serve_project_code(&self) -> anyhow::Result<()> {
        let mut markdown = format!("# Student Submission Source Code\n\n## Overview\n\n{}\n\n## \
                                    Source Code\n\n",
                                   self.describe());

        for file in &self.files {
            markdown.push_str(&format!("### {}\n\n```java\n{}\n```\n\n",
                                       file.proper_name(),
                                       file.parser().code()));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let submission = serde_json::to_string(&SubmissionRow { id:      id.clone(),
                                                                course:  COURSE.to_string(),
                                                                term:    TERM.to_string(),
                                                                content: markdown, })?;

        let rt = RUNTIME.handle().clone();
        rt.block_on(async {
              POSTGREST_CLIENT.from("submissions")
                              .insert(submission)
                              .execute()
                              .await
          })?;

        println!("Please visit https://feedback.dhruvdh.com/submissions/{} to see your \
                  submission code.",
                 id);

        Ok(())
    }
}

/// Schema for `submissions` table
#[derive(Serialize, Debug)]
pub struct SubmissionRow {
    /// UUID of data entry
    id:      String,
    /// Course the submission belongs to
    course:  String,
    /// Term of the course
    term:    String,
    /// Content of the submission
    content: String,
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
impl CustomType for Project {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("JavaProject")
               .with_fn("new_java_project", Project::new_script)
               .with_fn("identify", Project::identify_mut_script)
               .with_fn("files", Project::files)
               .with_fn("info", Project::info_script);
    }
}

    </file-contents>
    <file-contents path="./src/lib.rs" name="lib.rs">
//! # umm
//!
//! A scriptable build tool/grader/test runner for Java projects that don't use
//! package managers.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]
#![feature(iterator_try_collect)]
#![feature(stmt_expr_attributes)]
#![feature(let_chains)]
#![feature(iter_collect_into)]

/// A module defining a bunch of constant values to be used throughout
pub mod constants;
/// For all things related to grading
pub mod grade;
/// For all things related to project health
pub mod health;
/// For discovering Java projects, analyzing them, and generating/executing
/// build tasks
pub mod java;
/// For all parsers used
pub mod parsers;
/// Utility functions for convenience
pub mod util;
/// For structs and enums related to VSCode Tasks
pub mod vscode;

use anyhow::{Context, Result};
use constants::{
    BUILD_DIR, COURSE, LIB_DIR, POSTGREST_CLIENT, ROOT_DIR, RUNTIME, SCRIPT_AST, TERM,
};
use grade::*;
use java::{File, FileType, Parser, Project};
use rhai::{Engine, EvalAltResult};
use umm_derive::generate_rhai_variant;
use util::{use_active_retrieval, use_heuristic_retrieval};

/// Defined for convenience
type Dict = std::collections::HashMap<String, String>;

/// Creates and returns a new `Engine` with all the types and functions
/// registered
pub fn create_engine() -> Engine {
    let mut engine = Engine::new();
    engine.register_type_with_name::<FileType>("JavaFileType")
          .build_type::<DocsGrader>()
          .build_type::<ByUnitTestGrader>()
          .build_type::<UnitTestGrader>()
          .build_type::<ByHiddenTestGrader>()
          .build_type::<DiffGrader>()
          .build_type::<Grade>()
          .build_type::<GradeResult>()
          .build_type::<Parser>()
          .build_type::<File>()
          .build_type::<Query>()
          .build_type::<QueryGrader>()
          .build_type::<Project>()
          .register_fn("clean", clean_script)
          .register_fn("show_results", show_result_script)
          .register_fn("generate_single_feedback", generate_single_feedback_script)
          .register_fn("generate_feedback", generate_feedback_script)
          .register_fn("use_active_retrieval", use_active_retrieval)
          .register_fn("use_heuristic_retrieval", use_heuristic_retrieval);
    engine
}

/// Prints the result of grading
pub fn grade(name_or_path: &str) -> Result<()> {
    let engine = create_engine();

    // println!("{}", engine.gen_fn_signatures(false).join("\n"));
    let script = match std::fs::read_to_string(name_or_path) {
        Ok(s) => s,
        Err(_) => {
            let assignment_name = name_or_path.to_string().replace(['\"', '\\'], "");
            let rt = RUNTIME.handle().clone();

            let resp = rt.block_on(async {
                             POSTGREST_CLIENT.from("grading_scripts")
                                             .eq("course", COURSE)
                                             .eq("term", TERM)
                                             .eq("assignment", &assignment_name)
                                             .select("url")
                                             .single()
                                             .execute()
                                             .await?
                                             .text()
                                             .await
                                             .context(format!("Could not get grading script for \
                                                               {assignment_name}"))
                         });

            let resp: serde_json::Value = serde_json::from_str(resp?.as_str())?;
            let resp = resp.as_object().unwrap();

            if let Some(message) = resp.get("message") {
                anyhow::bail!("Error for {assignment_name}: {message}");
            }

            let script_url = resp.get("url").unwrap().as_str().unwrap();

            reqwest::blocking::get(script_url).context(format!("Cannot get url: {script_url}"))?
                                              .text()
                                              .context(format!("Could not parse the response \
                                                                from {script_url} to text."))?
        }
    };
    let ast = engine.compile(script)?;
    {
        let ast = std::sync::Arc::clone(&SCRIPT_AST);
        let mut ast = ast.lock().unwrap();
        *ast = ast.clone();
    }

    // Run the script
    engine.run_ast(&ast)?;

    Ok(())
}

#[generate_rhai_variant(Fallible)]
/// Deletes all java compiler artefacts
pub fn clean() -> Result<()> {
    if BUILD_DIR.as_path().exists() {
        std::fs::remove_dir_all(BUILD_DIR.as_path()).with_context(|| {
                                                        format!("Could not delete {}",
                                                                BUILD_DIR.display())
                                                    })?;
    }
    if LIB_DIR.as_path().exists() {
        std::fs::remove_dir_all(LIB_DIR.as_path()).with_context(|| {
                                                      format!("Could not delete {}",
                                                              LIB_DIR.display())
                                                  })?;
    }
    if ROOT_DIR.join(".vscode/settings.json").as_path().exists() {
        std::fs::remove_file(ROOT_DIR.join(".vscode/settings.json").as_path()).with_context(
            || {
                format!(
                    "Could not delete {}",
                    ROOT_DIR.join(".vscode/settings.json").display()
                )
            },
        )?;
    }
    if ROOT_DIR.join(".vscode/tasks.json").as_path().exists() {
        std::fs::remove_file(ROOT_DIR.join(".vscode/tasks.json").as_path()).with_context(|| {
                                                                               format!(
                "Could not delete {}",
                ROOT_DIR.join(".vscode/tasks.json").display()
            )
                                                                           })?;
    }

    Ok(())
}

// TODO: replace std::Command with cmd_lib
// TODO: Lazily load all constants from rhai scripts instead
// TODO: Fix java mod impls
// TODO: update classpath when discovering project
// TODO: fix grading api
// TODO: add rhai scripting for grading
// TODO: find a way to generate a rhai wrapper for all methods
// TODO: add rhai scripting for project init
// TODO: update tabled to 0.6
// TODO: make reedline shell optional behind a feature
// TODO: Download jars only if required OR remove jar requirement altogether.

    </file-contents>
    <file-contents path="./src/main.rs" name="main.rs">
//! # umm
//! ## Introduction

//! A java build tool for novices.

//! ## Installation

//! You would need rust installed, ideally the nightly toolchain. You can visit https://rustup.rs/ to find out how to install this on your computer, just make sure you install the "nightly" toolchain instead of stable.

//! On Linux, Windows Subsystem for Linux (WSL), and Mac you should be able to run `curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly` on a terminal to install the nightly toolchain for rust.

//! Once you are done, just type `cargo install --git=https://github.com/DhruvDh/umm.git` and it should compile and install it on your system.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    collections::HashSet,
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::Result;
use bpaf::*;
use dotenvy::dotenv;
use self_update::cargo_crate_version;
use tracing::{metadata::LevelFilter, Level};
use tracing_subscriber::{fmt, prelude::*, util::SubscriberInitExt};
use umm::{
    clean,
    constants::{LIB_DIR, ROOT_DIR, SOURCE_DIR, TEST_DIR},
    grade,
    java::Project,
};
use walkdir::WalkDir;

/// Updates binary based on github releases
fn update() -> Result<()> {
    self_update::backends::github::Update::configure().repo_owner("dhruvdh")
                                                      .repo_name("umm")
                                                      .bin_name("umm")
                                                      .no_confirm(true)
                                                      .target_version_tag("spring_24")
                                                      .show_download_progress(true)
                                                      .show_output(false)
                                                      .current_version(cargo_crate_version!())
                                                      .build()?
                                                      .update()?;

    eprintln!("Update done!");
    Ok(())
}

/// Enum to represent different commands
#[derive(Debug, Clone)]
enum Cmd {
    /// Run a file
    Run(String),
    /// Check a file
    Check(String),
    /// Test a file
    Test(String, Vec<String>),
    /// Check a files documentation
    DocCheck(String),
    /// Grade a file
    Grade(String),
    /// Create a submission zip
    CreateSubmission(String),
    /// Clean the project artifacts
    Clean,
    /// Print information about the project
    Info,
    /// Update the command
    Update,
    /// Checks project health
    CheckHealth,
    /// Starts and serves a web server that serves the project code
    ServeProjectCode,
    /// Resets the project metadata, and re-downloads libraries
    Reset,
    /// Exit the program
    Exit,
}

/// Parse the command line arguments and return a `Cmd` enum
fn options() -> Cmd {
    /// parses test names
    fn t() -> impl Parser<Vec<String>> {
        positional("TESTNAME").help("Name of JUnit test to run")
                              .many()
    }

    /// parsers file name
    fn f() -> impl Parser<String> {
        positional("FILENAME").help("Name of java file")
    }

    /// parses Assignment name or path to grading script file
    fn g() -> impl Parser<String> {
        positional("NAME/PATH").help("Name of assignment in database or path to grading script")
    }

    /// parses path to project root folder
    fn h() -> impl Parser<String> {
        positional("PATH").help("Path to project root folder. Defaults to current directory")
                          .fallback(String::from("."))
    }

    let run = construct!(Cmd::Run(f())).to_options()
                                       .command("run")
                                       .help("Run a java file with a main method");

    let check = construct!(Cmd::Check(f())).to_options()
                                           .command("check")
                                           .help("Check for syntax errors");

    let test = construct!(Cmd::Test(f(), t())).to_options()
                                              .command("test")
                                              .help("Run JUnit tests");

    let doc_check = construct!(Cmd::DocCheck(f())).to_options()
                                                  .command("doc-check")
                                                  .help("Check a file for missing javadoc");

    let grade = construct!(Cmd::Grade(g())).to_options()
                                           .command("grade")
                                           .help("Grade your work");

    let create_submission = construct!(Cmd::CreateSubmission(h())).to_options()
                                                                  .command("create-submission")
                                                                  .help("Create a submission zip");

    let clean =
        pure(Cmd::Clean).to_options()
                        .command("clean")
                        .help("Cleans the build folder, library folder, and vscode settings");

    let info = pure(Cmd::Info).to_options()
                              .command("info")
                              .help("Prints a JSON description of the project as parsed");

    let update = pure(Cmd::Update).to_options()
                                  .command("update")
                                  .help("Update the umm command");

    let check_health = pure(Cmd::CheckHealth).to_options()
                                             .command("check-health")
                                             .help("Checks the health of the project");

    let serve =
        pure(Cmd::ServeProjectCode).to_options()
                                   .command("serve-project-code")
                                   .help("Starts and serves a web server that serves the project \
                                          code");

    let reset = pure(Cmd::Reset).to_options()
                                .command("reset")
                                .help("Reset the project metadata, and re-download libraries");

    let exit = pure(Cmd::Exit).to_options()
                              .command("exit")
                              .help("Exit the program");

    let cmd = construct!([run,
                          check,
                          test,
                          doc_check,
                          grade,
                          create_submission,
                          clean,
                          info,
                          update,
                          check_health,
                          serve,
                          reset,
                          exit]).fallback(Cmd::Exit);

    cmd.to_options().descr("Build tool for novices").run()
}

fn main() -> Result<()> {
    dotenv().ok();

    let fmt = fmt::layer().without_time()
                          .with_file(false)
                          .with_line_number(false);
    let filter_layer = LevelFilter::from_level(Level::INFO);
    tracing_subscriber::registry().with(fmt)
                                  .with(filter_layer)
                                  .init();

    let cmd = options();

    // TODO: move this to a separate method and call that method in shell()
    match cmd {
        Cmd::Run(f) => {
            match Project::new()?.identify(f.as_str())?.run_mut_script(None) {
                Ok(out) => {
                    println!("{out}");
                }
                Err(e) => {
                    eprintln!("{:#?}", e);
                }
            };
        }
        Cmd::Check(f) => match Project::new()?.identify(f.as_str())?.check_mut_script() {
            Ok(out) => {
                println!("{out}");
            }
            Err(e) => {
                eprintln!("{:#?}", e);
            }
        },
        Cmd::Test(f, t) => {
            let out = if t.is_empty() {
                Project::new()?.identify(f.as_str())?
                               .test_mut_script(vec![])?
            } else {
                Project::new()?.identify(f.as_str())?
                               .test_mut_script(t.iter().map(|i| i.as_str()).collect())?
            };

            println!("{out}");
        }
        Cmd::DocCheck(f) => {
            let out = Project::new()?.identify(f.as_str())?
                                     .doc_check_mut_script()?;
            println!("{out}");
        }
        Cmd::Grade(g) => grade(&g)?,
        Cmd::CreateSubmission(p) => {
            let zip_file_name = format!("submission-{}.zip",
                                        chrono::offset::Local::now().format("%Y-%m-%d-%H-%M-%S"));
            let zip_file = std::fs::File::create(PathBuf::from(zip_file_name.clone()))?;

            let all_files = {
                let source_walkdir: Vec<_> =
                    WalkDir::new(SOURCE_DIR.as_path()).into_iter()
                                                      .filter_map(|e| e.ok())
                                                      .collect();
                let lib_walkdir: Vec<_> = WalkDir::new(LIB_DIR.as_path()).into_iter()
                                                                         .filter_map(|e| e.ok())
                                                                         .collect();
                let test_walkdir: Vec<_> = WalkDir::new(TEST_DIR.as_path()).into_iter()
                                                                           .filter_map(|e| e.ok())
                                                                           .collect();
                let all_java_files: Vec<_> =
                    WalkDir::new(PathBuf::from(p).as_path()).into_iter()
                                                            .filter_map(|e| {
                                                                e.ok().filter(|x| {
                                                                          x.path()
                                                                           .extension()
                                                                           .unwrap_or_default()
                                                                          == "java"
                                                                      })
                                                            })
                                                            .collect();

                source_walkdir.into_iter()
                              .chain(lib_walkdir)
                              .chain(test_walkdir)
                              .chain(all_java_files)
            };

            let mut zip = zip::ZipWriter::new(zip_file);
            let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o755);
            let mut buffer = Vec::new();
            let mut already_added = HashSet::<PathBuf>::new();

            for entry in all_files {
                let path = match entry.path().strip_prefix(ROOT_DIR.as_path()) {
                    Ok(path) => path,
                    Err(_) => entry.path(),
                };

                if already_added.contains(path) {
                    continue;
                } else {
                    already_added.insert(path.to_path_buf());
                }

                let mut name = PathBuf::from(ROOT_DIR.as_path());
                name.push(path);

                if path.is_file() {
                    #[allow(deprecated)]
                    zip.start_file_from_path(name.as_path(), options)?;
                    let mut f = std::fs::File::open(path)?;

                    f.read_to_end(&mut buffer)?;
                    zip.write_all(&buffer)?;
                    buffer.clear();
                } else if !name.as_os_str().is_empty() {
                    // Only if not root! Avoids path spec / warning
                    // and mapname conversion failed error on unzip
                    #[allow(deprecated)]
                    zip.add_directory_from_path(name.as_path(), options)?;
                }
            }

            zip.finish()?;
            println!("Submission zip created - {}", zip_file_name);
        }
        Cmd::Clean => clean()?,
        Cmd::Info => Project::new()?.info()?,
        Cmd::Update => {
            match update() {
                Ok(_) => {}
                Err(e) => eprintln!("{e}"),
            };
        }
        Cmd::CheckHealth => Project::new()?.check_health()?,
        Cmd::ServeProjectCode => Project::new()?.serve_project_code()?,
        Cmd::Reset => {
            clean()?;
            Project::new()?;
        }
        Cmd::Exit => {}
    };

    Ok(())
}

    </file-contents>
    <file-contents path="./src/parsers.rs" name="parsers.rs">
use crate::grade::{JavacDiagnostic, LineRef, MutationDiagnostic};

peg::parser! {
    /// includes some useful grammars for parsing JUNit/javac/pitest outputs.
    pub grammar parser() for str {
        /// matches any sequence of 1 or more numbers
        rule number() -> u32
            = n:$(['0'..='9']+) {? n.parse().or(Err("u32")) }

        /// matches any number of whitespace characters
        rule whitespace() = quiet!{[' ' | '\n' | '\t' | '\r']+}

        /// matches the keyword "tests successful"
        rule successful_tests()
            = " tests successful"

        /// matches the keyword "tests found"
        rule found_tests()
            = " tests found"

        /// parses and returns the number of tests passed
        pub rule num_tests_passed() -> u32
            = "[" whitespace()? l:number() successful_tests() whitespace()? "]" { l }

        /// parses and returns the number of tests found
        pub rule num_tests_found() -> u32
            = "[" whitespace()? l:number() found_tests() whitespace()? "]" { l }

        /// matches any path separator, hopefully cross-platform
        rule path_separator() =
            whitespace()?
            "."?
            "/" / "\\" / "\\\\"
            whitespace()?

        /// matches any sequence of upper and lowercase alphabets
        rule word() -> String
            = whitespace()?
                w:[
                    'a'..='z' |
                    'A'..='Z' |
                    '0'..='9' |
                    '-' | '.' | ' ' |
                    '[' | ']' | '_'
                ]+
                whitespace()?
            { w.iter().collect::<String>() }

        /// matches any sequence of upper and lowercase alphabets
        rule mutations_csv_word() -> String
            = whitespace()?
                w:[
                    'a'..='z' |
                    'A'..='Z' |
                    '0'..='9' |
                    '-' | '.' | ' ' |
                    '[' | ']' | ':' |
                    '<' | '>' | '_' |
                    '(' | ')'
                ]+
                whitespace()?
            { w.iter().collect::<String>() }

        /// matches any valid path, hopefully
        rule path() -> String
            = whitespace()?
              path_separator()?
              p:(word() ++ path_separator())
              whitespace()?
            { p.iter().fold(String::new(), |acc, w| format!("{acc}/{w}")) }

        /// matches line numbers (colon followed by numbers, eg. :23)
        rule line_number() -> u32
            = ":" n:number() ":" whitespace()? { n }

        /// matches "error" or "warning", returns true if error
        rule diag_type() -> bool
            = whitespace()?
              a:"error"? b:"warning"?
              ":"
              whitespace()?
            { a.is_some() }

        /// matches anything, placed where diagnostic should be
        rule diagnostic() -> String
            = a:([_]+)
            { a.iter().collect::<String>() }

        /// parses the first line of a javac diagnostic message and returns a `JavacDiagnostic`
        pub rule parse_diag() -> JavacDiagnostic
            = p:path() l:line_number() d:diag_type() m:diagnostic()
            {
                let p = std::path::PathBuf::from(p);
            let name = p.file_name().expect("Could not parse path to file in javac error/warning");

            JavacDiagnostic::builder()
                .path(format!(".{}", p.display()))
                .file_name(name.to_string_lossy().to_string())
                .line_number(l)
                .is_error(d)
                .message(if d { format!("Error: {m}") } else { m })
                .build()
            }

        rule mutation_test_examined_path() -> Vec<String>
            = a:mutations_csv_word()? "/"? b:mutations_csv_word()? "/"?  c:mutations_csv_word()?
            {
                let mut res = vec![];
                if let Some(a) = a { res.push(a); }
                if let Some(b) = b { res.push(b); }
                if let Some(c) = c { res.push(c); }
                res
            }

        rule mutation_test_examined_none() -> &'input str
            = $("none")

        /// parses one row of mutation report
        pub rule mutation_report_row() -> MutationDiagnostic
            = file_name:word()
              ","
              source_file_name:word()
              ","
              mutation:word()
              ","
              source_method:mutations_csv_word()
              ","
              line_no:number()
              ","
              result:word()
              ","
              test_method:mutation_test_examined_path()?
              whitespace()?
                {
                let test = test_method.unwrap_or_else(|| panic!("Had trouble parsing last column for mutation at {source_file_name}#{source_method}:{line_no}"));
                let mut test_file_name;
                let mut test_method;

    if test.len() == 3 {
                    let splitter = if test.get(1).unwrap().contains("[runner:") { "[runner:" } else { "[class:" };
                    test_file_name = test.get(1)
                                .unwrap()
                                .to_string()
                                .split_once(splitter)
                                .unwrap_or_else(|| panic!("had trouble parsing test_file_class for mutation at {source_file_name}#{source_method}:{line_no}"))
                                .1
                                .replace(']', "");

                    let splitter = if test.get(2).unwrap().contains("[test:") { "[test:" } else { "[method:" };
                    test_method = test.get(2)
                                    .unwrap()
                                    .to_string()
                                    .split_once(splitter)
                                    .unwrap_or_else(|| panic!("Had trouble parsing test_file_method for mutation at {source_file_name}#{source_method}:{line_no}"))
                                    .1
                                    .replace("()]", "");
                } else {
                    test_file_name = "NA".to_string();
                    test_method = "None".to_string()
                }
                let mutator = mutation
                                .to_string()
                                .split_once(".mutators.")
                                .expect("Could not split mutators while parsing mutations.csv.")
                                .1.to_string();

                MutationDiagnostic::builder()
                    .line_number(line_no)
                    .mutator(mutator)
                    .source_file_name(source_file_name)
                    .source_method(source_method)
                    .test_file_name(test_file_name)
                    .test_method(test_method)
                    .result(result)
                    .build()
            }

            /// Parses a word in a JUnit stacktrace
            rule junit_stacktrace_word() -> String
                = whitespace()?
                w:[
                    'a'..='z' |
                    'A'..='Z' |
                    '0'..='9' |
                    '-' | '.' | ' ' |
                    '[' | ']' | '/' |
                    '>' | '=' | '$'
                ]+
                whitespace()?
            { w.iter().collect::<String>() }

            /// Parses a filename from a JUnit stacktrace
            rule junit_stacktrace_filename() -> String
                = whitespace()?
                w:[
                    'a'..='z' |
                    'A'..='Z' |
                    '0'..='9' |
                    '-' | '_' | '$'
                ]+
                ".java:"
                whitespace()?
            { w.iter().collect::<String>() }


            /// Parses a LineRef from a JUnit stacktrace
            pub rule junit_stacktrace_line_ref() -> LineRef
                = whitespace()?
                junit_stacktrace_word()*
                whitespace()?
                "("
                c:junit_stacktrace_filename()
                d:number()
                whitespace()?
                ")"
                whitespace()?
                {
                    LineRef { line_number: d as usize, file_name: c }
                }
    }
}

    </file-contents>
    <file-contents path="./src/util.rs" name="util.rs">
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use glob::glob;
use tokio::io::AsyncWriteExt;
use which::which;

use crate::constants::*;

/// Finds and returns the path to javac binary
pub fn javac_path() -> Result<OsString> {
    which("javac").map(PathBuf::into_os_string)
                  .context("Cannot find a Java Compiler on path (javac)")
}

/// Finds and returns the path to java binary
pub fn java_path() -> Result<OsString> {
    which("java").map(PathBuf::into_os_string)
                 .context("Cannot find a Java runtime on path (java)")
}

/// Finds and returns the path to umm
/// If not found, returns "./umm"
pub fn umm_path() -> String {
    match which("umm") {
        Ok(path) => path.display().to_string(),
        Err(_) => "./umm".into(),
    }
}

/// A glob utility function to find paths to files with certain extension
///
/// * `extension`: the file extension to find paths for
/// * `search_depth`: how many folders deep to search for
/// * `root_dir`: the root directory where search starts
pub fn find_files(extension: &str,
                  search_depth: i8,
                  root_dir: &Path)
                  -> Result<Vec<PathBuf>> {
    let mut root_dir = PathBuf::from(root_dir);

    for _ in 0..search_depth {
        root_dir.push("**");
    }

    root_dir.push(format!("*.{extension}"));
    let root_dir = root_dir.to_str()
                           .context("Could not convert root_dir to string")?;

    Ok(glob(root_dir).context("Could not create glob")?
                     .filter_map(Result::ok)
                     .map(|path| ROOT_DIR.join(path))
                     .collect())
}

/// Find class, jar files in library path and build directory to populate
/// classpath and return it
pub fn classpath() -> Result<String> {
    let mut path: Vec<String> = vec![LIB_DIR.display().to_string(),
                                     BUILD_DIR.display().to_string(),];

    path.append(&mut find_files("jar", 4, &ROOT_DIR)?.iter()
                                                     .map(|p| p.as_path().display().to_string())
                                                     .collect());

    Ok(path.join(&SEPARATOR))
}

/// Find java files in source path and root directory to populate
/// sourcepath and return it
pub fn sourcepath() -> Result<String> {
    let mut path: Vec<String> = vec![SOURCE_DIR.join("").display().to_string(),
                                     TEST_DIR.join("").display().to_string(),
                                     ROOT_DIR.join("").display().to_string(),];

    path.append(&mut find_files("java", 4, &ROOT_DIR)?.iter()
                                                      .map(|p| p.as_path().display().to_string())
                                                      .collect());

    Ok(path.join(&SEPARATOR))
}

/// TODO: Add docs
pub async fn download(url: &str,
                      path: &PathBuf,
                      replace: bool)
                      -> Result<()> {
    if !replace && path.exists() {
        Ok(())
    } else {
        let bytes = reqwest::get(url).await
                                     .context(format!("Failed to download url: {url}"))?
                                     .bytes()
                                     .await
                                     .context(format!("Failed to read response as bytes: {url}"))?;

        let name = path.file_name().unwrap().to_str().unwrap();

        let mut file =
            tokio::fs::File::create(path).await
                                         .context(format!("Failed to create file at {name}"))?;

        file.write_all(&bytes)
            .await
            .context(format!("Failed to write to file at {name}"))
    }
}

/// Download a URL and return response as string
pub async fn download_to_string(url: &str) -> Result<String> {
    reqwest::get(url).await
                     .context(format!("Failed to download url: {url}"))?
                     .text()
                     .await
                     .context(format!("Failed to read response as text: {url}"))
}

/// Download a URL and return response as JSON
pub async fn download_to_json(url: &str) -> Result<HashMap<String, String>> {
    reqwest::get(url).await
                     .context(format!("Failed to download url: {url}"))?
                     .json()
                     .await
                     .context(format!("Failed to read response as json: {url}"))
}

/// Use active retrieval when retrieving context from student submission.
pub fn use_active_retrieval() {
    USE_ACTIVE_RETRIEVAL.set(true);
    dbg!(USE_ACTIVE_RETRIEVAL.get());
}

/// Use heuristic based retrieval when retrieving context from student
/// submission.
pub fn use_heuristic_retrieval() {
    USE_ACTIVE_RETRIEVAL.set(false);
}

    </file-contents>
    <file-contents path="./src/vscode.rs" name="vscode.rs">
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

/// Enum for VSCode task's type.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Type {
    /// If shell is specified, the command is interpreted as a shell command
    /// (for example: bash, cmd, or PowerShell).
    Shell,
    ///  If process is specified, the command is interpreted as a process to
    /// execute.
    Process,
}

/// enum for VSCode task's arg quoting.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArgQuoting {
    /// escape strings
    Escape,
    /// ses the shell's strong quoting mechanism, which suppresses all
    /// evaluations inside the string. Under PowerShell and for shells under
    /// Linux and macOS, single quotes are used (`'`). For cmd.exe, `"` is used.
    Strong,
    /// Uses the shell's weak quoting mechanism, which still evaluates
    /// expression inside the string (for example, environment variables). Under
    /// PowerShell and for shells under Linux and macOS, double quotes are used
    /// (`"`). cmd.exe doesn't support weak quoting so VS Code uses `"` as well.
    Weak,
}

/// Struct for VSCode task's args.
#[derive(Serialize, Deserialize, TypedBuilder)]
pub struct Args {
    /// value of arg
    #[builder(default, setter(into))]
    value:   String,
    /// specifies how to escape the arg value.
    #[builder(default=ArgQuoting::Escape)]
    quoting: ArgQuoting,
}

/// Enum for VSCode task's dependsOrder.
#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum DependsOrder {
    /// In parallel with other tasks.
    Parallel,
    /// In sequence with other tasks.
    Sequence,
}

/// Struct for VSCode task's presentation.
#[derive(Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(doc)]
#[builder(field_defaults(default, setter(into)))]
pub struct Presentation {
    /// Controls whether the Integrated Terminal panel is brought to front.
    /// Valid values are:
    /// * `always` - The panel is always brought to front. This is the default.
    /// * `never` - The user must explicitly bring the terminal panel to the
    ///   front using the  **View** > **Terminal** command
    ///   (`kb(workbench.action.terminal.toggleTerminal)`).
    /// * `silent` - The terminal panel is brought to front only if the output
    ///   is not scanned for errors and warnings.
    #[serde(skip_serializing_if = "Option::is_none")]
    reveal:             Option<String>,
    /// Controls whether the Problems panel is revealed when running this task
    /// or not. Takes precedence over option `reveal`. Default is `never`.
    ///   * `always` - Always reveals the Problems panel when this task is
    ///     executed.
    ///   * `onProblem` - Only reveals the Problems panel if a problem is found.
    ///   * `never` - Never reveals the Problems panel when this task is
    ///     executed.
    #[serde(skip_serializing_if = "Option::is_none")]
    reveal_problems:    Option<String>,
    /// Controls whether the terminal is taking input focus or not. Default is
    /// `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    focus:              Option<bool>,
    /// Controls whether the executed command is echoed in the terminal. Default
    /// is `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    echo:               Option<bool>,
    /// Controls whether to show the "Terminal will be reused by tasks, press
    /// any key to close it" message.
    #[serde(skip_serializing_if = "Option::is_none")]
    show_reuse_message: Option<bool>,
    /// Controls whether the terminal instance is shared between task runs.
    /// Possible values are:
    ///   * `shared` - The terminal is shared and the output of other task runs
    ///     are added to the same terminal.
    ///   * `dedicated` - The terminal is dedicated to a specific task. If that
    ///     task is executed again, the terminal is reused. However, the output
    ///     of a different task is presented in a different terminal.
    ///   * `new` - Every execution of that task is using a new clean terminal.
    #[serde(skip_serializing_if = "Option::is_none")]
    panel:              Option<String>,
    /// Controls whether the terminal is cleared before this task is run.
    /// Default is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    clear:              Option<bool>,
    /// Controls whether the terminal the task runs in is closed when the task
    /// exits.

    #[serde(skip_serializing_if = "Option::is_none")]
    close:              Option<bool>,
    /// Controls whether the task is executed in a specific terminal group using
    /// split panes. Tasks in the same group (specified by a string value) will
    /// use split terminals to present instead of a new terminal panel.
    #[serde(skip_serializing_if = "Option::is_none")]
    group:              Option<bool>,
}

/// Struct for VSCode task's problem matcher.
#[derive(Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct ProblemMatcher {
    /// Controls if a problem reported on a text document is applied only to
    /// open, closed or all documents.
    /// Valid values are:
    ///  * `openDocuments` - Only applied to open documents.
    /// * `closedDocuments` - Only applied to closed documents.
    /// * `allDocuments` - Applied to all documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(default, setter(into))]
    apply_to:      Option<String>,
    /// Patterns to track the begin and end of a matcher active on a background
    /// task.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    background:    Option<String>,
    /// The name of a base problem matcher to use.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    base:          Option<String>,
    /// Defines how file names reported in a problem pattern should be
    /// interpreted. A relative fileLocation may be an array, where the second
    /// element of the array is the path the relative file location.
    /// Valid values are:
    ///  * `absolute` - File names are interpreted as absolute paths.
    /// * `relative` - File names are interpreted as relative paths.
    /// * `autoDetect` - automatically detects
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    file_location: Option<Vec<String>>,
    /// The owner of the problem inside Code. Can be omitted if base is
    /// specified. Defaults to 'external' if omitted and base is not specified.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    owner:         Option<String>,
    /// A problem pattern or the name of a contributed or predefined problem
    /// pattern. Can be omitted if base is specified.
    pattern:       Pattern,
    /// The default severity for captures problems. Is used if the pattern
    /// doesn't define a match group for severity.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    severity:      Option<String>,
    /// A human-readable string describing the source of this diagnostic, e.g.
    /// 'typescript' or 'super lint'.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    source:        Option<String>,
}

/// Struct for VSCode task's problem matcher's pattern.
#[derive(Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct Pattern {
    /// The match group index of the problem's code. Defaults to undefined
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    code:       Option<usize>,
    /// The match group index of the problem's line character. Defaults to 3
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    column:     Option<usize>,
    /// The match group index of the problem's end line character. Defaults to
    /// undefined
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    end_column: Option<usize>,
    /// The match group index of the problem's end line. Defaults to undefined
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    end_line:   Option<usize>,
    /// The match group index of the filename. If omitted 1 is used.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    file:       Option<usize>,
    /// whether the pattern matches a location (file and line) or only a file.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    kind:       Option<String>,
    /// The match group index of the problem's line. Defaults to 2
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    line:       Option<usize>,
    /// The match group index of the problem's location. Valid location patterns
    /// are: (line), (line,column) and
    /// (startLine,startColumn,endLine,endColumn). If omitted (line,column) is
    /// assumed.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    location:   Option<String>,
    /// In a multi line matcher loop indicated whether this pattern is executed
    /// in a loop as long as it matches. Can only specified on a last pattern in
    /// a multi line pattern.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    r#loop:     Option<bool>,
    /// The match group index of the message. If omitted it defaults to 4 if
    /// location is specified. Otherwise it defaults to 5.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    message:    Option<usize>,
    /// The regular expression to find an error, warning or info in the output.
    #[builder(setter(into))]
    regexp:     String,
    /// The match group index of the problem's severity. Defaults to undefined
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    severity:   Option<usize>,
}

/// Struct to represent a VSCode task as JSON.
#[derive(Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    /// The task's label used in the user interface.
    label:           String,
    /// The task's type.
    #[builder(default=Some(Type::Shell), setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type:          Option<Type>,
    /// The actual command to execute.
    #[builder(default, setter(into))]
    command:         String,
    /// Any Windows specific properties. Will be used instead of the default
    /// properties when the command is executed on the Windows operating system.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    windows:         Option<String>,
    /// Defines to which group the task belongs. In the example, it belongs to
    /// the test group. Tasks that belong to the test group can be executed by
    /// running Run Test Task from the Command Palette.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    group:           Option<String>,
    /// Defines how the task output is handled in the user interface.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    presentation:    Option<Presentation>,
    /// Override the defaults for cwd (current working directory), env
    /// (environment variables), or shell (default shell). Options can be set
    /// per task but also globally or per platform. Environment variables
    /// configured here can only be referenced from within your task script or
    /// process and will not be resolved if they are part of your args, command,
    /// or other task attributes.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    options:         Option<String>,
    /// Arguments passed to the command when this task is invoked.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    args:            Option<Vec<Args>>,
    /// Either a string representing another task or an array of other tasks
    /// that this task depends on.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    depends_on:      Option<Vec<String>>,
    /// Run all dependsOn tasks in parallel.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    depends_order:   Option<DependsOrder>,
    /// An optional description of a task that shows in the Run Task quick pick
    /// as a detail.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    detail:          Option<String>,
    /// An optional icon path.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    icon:            Option<String>,
    /// Whether the executed task is kept alive and is running in the
    /// background.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    is_background:   Option<bool>,
    /// Any linux specific properties. Will be used instead of the default
    /// properties when the command is executed on the Linux operating system.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    linux:           Option<String>,
    /// Any macOS specific properties. Will be used instead of the default
    /// properties when the command is executed on the macOS operating system.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    osx:             Option<String>,
    /// The problem matcher(s) to use. Can either be a string or a problem
    /// matcher definition or an array of strings and problem matchers.
    #[builder(default=Some(Vec::new()))]
    problem_matcher: Option<Vec<ProblemMatcher>>,
    /// Whether the user is prompted when VS Code closes with a running task.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_on_close: Option<bool>,
    /// The task's run related options.
    /// Valid values are:
    /// * **reevaluateOnRerun**: Controls how variables are evaluated when a
    ///   task is executed through the **Rerun Last Task** command. The default
    ///   is `true`, meaning that variables will be reevaluated when a task is
    ///   rerun. When set to `false` the resolved variable values from the
    ///   previous run of the task will be used.
    /// * **runOn**: Specifies when a task is run.
    /// * `default` - The task will only be run when executed through the **Run
    ///   Task** command.
    /// * `folderOpen` - The task will be run when the containing folder is
    ///   opened. The first time you open a folder that contains a task with
    ///   `folderOpen`, you will be asked if you want to allow tasks to run
    ///   automatically in that folder. You can change your decision later using
    ///   the **Manage Automatic Tasks in Folder** command and selecting between
    ///   **Allow Automatic Tasks in Folder** and **Disallow Automatic Tasks in
    ///   Folder**.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    run_options:     Option<String>,
}

/// default run task action
fn run_task_action() -> Option<String> {
    Some("workbench.action.tasks.runTask".to_string())
}

/// default run task action
fn when_keybindings() -> Option<String> {
    Some("config:workspaceKeybindings.ummTasksKeys.enabled".to_string())
}

/// A struct to represent a keybinding for tasks in VSCode.
#[derive(Serialize, Deserialize, TypedBuilder)]
pub struct KeyBindings {
    /// The keybinding
    key:     String,
    /// The command to execute, defaults to `workbench.action.tasks.runTask`
    #[serde(default = "run_task_action")]
    #[builder(default, setter(into))]
    command: Option<String>,
    /// The command's arguments - name of task, etc.
    args:    String,
    /// when to activate keybinding
    #[serde(default = "when_keybindings")]
    #[builder(default, setter(into))]
    when:    Option<String>,
}

/// Enum to represent the type of a task input.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Input {
    /// Shows an input box to get a string from the user.
    PromptString {
        /// ID for input
        id:          String,
        /// Shown in the quick input, provides context for the input.
        description: String,
        /// Default value that will be used if the user doesn't enter something
        /// else.
        default:     String,
        ///  Set to true to input with a password prompt that will not show the
        /// typed value.
        password:    Option<bool>,
    },
    /// Shows a Quick Pick dropdown to let the user select from several options.
    PickString {
        /// ID for input
        id:          String,
        /// Shown in the quick input, provides context for the input.
        description: String,
        /// A list of strings to pick from.
        options:     Vec<String>,
        /// Default value that will be used if the user doesn't enter something
        /// else. It must be one of the option values.
        default:     String,
    },
}

/// Struct representing a tasks.json file
#[derive(Serialize, Deserialize, TypedBuilder)]
pub struct TasksFile {
    /// The tasks.json version.
    #[builder(default = "2.0.0".to_string())]
    version: String,
    /// The tasks.json tasks.
    #[builder(default = vec![])]
    tasks:   Vec<Task>,
    /// The tasks.json keybindings.
    #[builder(default = vec![])]
    inputs:  Vec<Input>,
}

/// Struct representing vscode settings.json file
/// Only the properties that we need.
#[derive(Serialize, Deserialize, TypedBuilder)]
pub struct SettingsFile<'a> {
    /// javac source path
    #[serde(rename = "java.project.sourcePaths")]
    java_source_path:     Vec<String>,
    /// javac target path
    #[serde(rename = "java.project.outputPath")]
    java_output_path:     String,
    /// javac classpath
    #[serde(rename = "java.project.referencedLibraries")]
    java_referenced_libs: Vec<String>,
    /// whether to use keybindings or not
    #[serde(rename = "workspaceKeybindings.ummTasksKeys.enabled")]
    #[builder(default = true)]
    keybindings_enabled:  bool,
    /// path to umm binary
    #[serde(rename = "ummBinaryPath")]
    umm_binary_path:      String,
    /// word wrap setting
    #[serde(rename = "editor.wordWrap")]
    #[builder(default = "on")]
    word_wrap:            &'a str,
    /// minimap setting
    #[serde(rename = "editor.minimap.enabled")]
    #[builder(default = false)]
    minimap:              bool,
}

    </file-contents>
  </folder>

  <folder path="./src/prompts">
    <file-contents path="./src/prompts/javadoc.md" name="javadoc.md">
> - The student is sharing errors and warning generated when linting JavaDoc documentation using `javac -Xdoclint` flag.
> - Sometimes JavaDoc cannot be linted due to compiler errors and the compiler errors are shared instead.
> - At the end of your explanation, share a list of places the student needs to add JavaDoc documentation.

    </file-contents>
    <file-contents path="./src/prompts/mutation_testing.md" name="mutation_testing.md">
> Note:
>
> - The autograder is running PiTest mutation testing. Target test is {test}, and target class is {class}.
> - Assume the student is new to mutation testing and does not understand the autograder output for mutation testing.
> - Sharing examples of what each mutator does is preferred.
> - If you are unsure what a mutator does, ask students to read the [List of Mutators](https://charlotte-cci-icc.github.io/itsc-2214-readings/07_list_of_mutators.html) document.

    </file-contents>
    <file-contents path="./src/prompts/mutation_testing_2.md" name="mutation_testing_2.md">
> Note:
> 
> - The autograder is running PiTest mutation testing. Target test is {test}, and target class is {class}.
> - Assume the student is new to mutation testing and does not understand the autograder output for mutation testing.
> - Mutation testing can only run when all tests are passing. If no mutations are shown, please explain why to the student and ask them to first ensure that the tests pass.

    </file-contents>
    <file-contents path="./src/prompts/retrieval_system_message_intro.md" name="retrieval_system_message_intro.md">
As part of an AI collaborative system tutoring students working on their Java labs, your role is pivotal in triaging issues based on autograder feedback and selecting what parts of a student's submission should be retrieved for another AI that specializes in offering targeted guidance.

The user will share with you auto-grader feedback from their lab submission, which you must interpret to understand the challenges within the student's work. Based on this analysis, you will select code excerpts using the provided tools and functions to share with the assisting AI.

Rules for your operation:

1. Carefully study the auto-grader feedback shared with you by the student, which can include:
   - Summary of passed and failed tests
   - Compiler errors
   - Runtime exceptions
   - stdout output for any print statements the student wrote

2. Select which methods from the student's code to share with the tutoring AI to help the student. This tutoring AI will only have access to the autograder feedback you studied, and the contents of the methods you select. These should correlate directly with the issues highlighted by the auto-grader, such as:
   - Source methods where the failure occurred
   - Other source methods that are likely to be implicated in the failure
   - Test method that failed

3. Do not select less than three or more than nine methods to avoid overburdening the tutoring AI with data, there is a limit on how much text it can process at once.

4. JUnit tests are typically written by the instructor, and the student is expected to write the code to pass the tests. The student is not expected to modify the tests. Generally, the fault lies in the student's code, not the tests.

Your discernment in interpreting the auto-grader feedback and relevant methods for retrieval is critical in streamlining the tutoring process, thus facilitating an effective learning journey for the student.

You MUST select at least three methods to share for the tutoring AI to be able to offer guidance to the student. You can select up to nine methods, but no more.

    </file-contents>
    <file-contents path="./src/prompts/retrieval_system_message_outro.md" name="retrieval_system_message_outro.md">
Java Files present in student's submission: {JAVA_FILE_NAMES}

> Below is a synthesized outline of the student's submission, detailing the structure, fields, and methods of the Java files, as derived from treesitter queries:

{SYNTHESIZED_OUTLINE}

    </file-contents>
    <file-contents path="./src/prompts/system_message_intro.md" name="system_message_intro.md">
You are an AI teaching assistant at UNC, Charlotte for students in introductory Java programming courses.

Your responses show up as feedback when students use an autograding tool called `umm` to check their code for correctness.

The interface does not allow the student to respond to you.

    </file-contents>
    <file-contents path="./src/prompts/system_message_outro.md" name="system_message_outro.md">
**Primary Objectives:**

1. Facilitate student learning and progress.
2. Foster independent problem-solving skills by equipping students with the necessary knowledge and strategies to confidently tackle similar problems in the future.
3. Encourage active learning and critical thinking.

**Rules to Follow:**

1. Use Markdown for formatting, with code blocks for identifiers and code snippets.
2. Avoid repeating explanations; refer students to previous responses when applicable.
3. Assume students are new to Java and its tooling; tailor explanations to their level.
4. If unsure, direct students to human teaching assistants for further assistance.
5. Do not share solutions directly. Use code snippets to provide high-level explanations and hints only.
6. Ensure responses are educational, well-written, concise, and to the point.

**Guidelines for Responses:**

1. When addressing multiple test failures or compiler errors, focus on one or two high-priority issues to help students make progress.
2. Quote relevant parts of compiler errors, stack traces, or test failure messages verbatim in code blocks when discussing issues.
3. Provide relevant examples or analogies to clarify complex concepts.
4. Acknowledge students' efforts and progress, and motivate them to persevere.
5. Be patient and empathetic; understand that students may be frustrated or confused.
6. Keep responses concise and appropriate as most students have short attention spans. Avoid lengthy explanations that may overwhelm them.

    </file-contents>
  </folder>

  <folder path="./src/prompts/slos">
    <file-contents path="./src/prompts/slos/algorithmic_solutions_quant.md" name="algorithmic_solutions_quant.md">
## Quantitative Reasoning SLO - Algorithmic solutions

Objective: Construct appropriate algorithmic solutions to computational problems

Rubric:

- Exemplary (5): For a given computational problem, the student is consistently able to construct an optimal algorithm to correctly solve the problem, often demonstrating creativity in the solution.

- Accomplished (4): For a given computational problem, the student can construct an algorithm that is appropriate and correctly solves the problem. The algorithm(s) used may be novel, simple, or an application of known / previously developed algorithms.

- Acceptable (3): For a given computational problem, the student selects an appropriate algorithm, but does not incorporate it into a complete and correct solution to the problem.

- Needs Improvement (2): For a given computational problem, the student is often unable to select and use an appropriate algorithm to solve the problem.
-
- Beginner (1): The student does not demonstrate the ability to select or use an algorithm to solve the problem.

## Feedback Guidelines

1. `number_of_stars` in the template must be formatted as described below. You absolutely MUST follow this template, as the system will look for these specific strings as shown below to determine the proficiency level for the student.

   - Exemplary (5): `### Proficiency: *****`
   - Accomplished (4): `### Proficiency: ****`
   - Acceptable (3): `### Proficiency: ***`
   - Needs Improvement (2): `### Proficiency: **`
   - Beginner (1): `### Proficiency: *`

   If the template includes `### Proficiency: ***`, the system will automatically assess the student as having met the Acceptable (3) level of proficiency.

2. Your feedback MUST include a snippet from the student's submission and an example of how it should be improved. Students will not be engaged unless they receive concrete actionable feedback. You MUST ensure that your feedback is actionable and specific. This snippet should only include relevant excerpts. For example, if commenting on a method's documentation comment, only include the documentation comment and the method signature, not the entire method.

3. The snippets you include as described in the template must not be the entire submission, but rather a specific section of the submission that you are providing feedback on.

4. Include as many snippets and feedback sections as you feel are necessary to provide a comprehensive review.

5. Keep responses concise and appropriate as most students have short attention spans. Avoid lengthy explanations that may overwhelm them.

## Feedback template

<!-- Template starts from here -->

## Algorithmic Solutions - {{ Feedback_title }}

{{ feedback_content }}

---

### Proficiency: {{ number_of_stars }}

{{ tips_and_suggestions_to_improve }}

    </file-contents>
    <file-contents path="./src/prompts/slos/code_readability_written_com.md" name="code_readability_written_com.md">
## Written Communication SLO - Code Readability and Formatting

Objective: Code formatting, organization, and other stylistic choices support readers in understanding your code.

Rubric:

- Exemplary (5): Not only is each program file easy to understand by reading it, but the student can make larger projects or projects designed for advanced architectures easy to navigate and understand as a whole. (For example, through clear, succinct, comprehensive README files within a project.)

- Accomplished (4): Others can easily understand the code by reading it. Examples: There is appropriate and consistent use of white space, indentation, etc. The code format adheres to standard language conventions. The *written* structure of the code is well-organized (all imports at the beginning of a file, all declarations at the beginning of a file/function body, separate files for each class in Java, reasonable ordering of function/object/main code blocks in Python, etc.). Maximum line length is conducive to readability. Minimal use of unnecessary hard-coded or global values that make the program more challenging to read, understand, and maintain.

- Acceptable (3): Others can understand most of the code by reading it, but some portions have inconsistent use of white space, indentation, etc., or do not adhere to standard language conventions.

- Needs Improvement (2): The program has comments, but they need improvement in one or more of the following areas: comments are not clear and meaningful; comments are inconsistent; comments do not adhere to standard language conventions; important code blocks that need explanation do not have sufficient comments; some in-line comments are redundant or unhelpful, etc.

- Needs Improvement (2): It is often difficult for others to understand the code through reading it because of inconsistent use of white space, indentation, etc.

- Beginner (1): It is very difficult for others to understand the code through reading it because of the highly inconsistent use of white space, indentation, etc.

## Formatting in VS Code

> Here is additional documentation on Formatting in Visual Studio Code, the IDE that students use. Point students in the direction of this documentation if they are struggling with formatting their code.

VS Code has great support for source code formatting. The editor has two explicit format actions:

- **Format Document** (`kb(editor.action.formatDocument)`) - Format the entire active file.
- **Format Selection** (`kb(editor.action.formatSelection)`) - Format the selected text.

You can invoke these from the **Command Palette** (`kb(workbench.action.showCommands)`) or the editor context menu.

VS Code has default formatters for JavaScript, TypeScript, JSON, HTML, and CSS. Each language has specific formatting options (for example, `html.format.indentInnerHtml`) which you can tune to your preference in your user or workspace [settings](/docs/getstarted/settings.md). You can also disable the default language formatter if you have another extension installed that provides formatting for the same language.

```json
"html.format.enable": false
```

Along with manually invoking code formatting, you can also trigger formatting based on user gestures such as typing, saving or pasting. These are off by default but you can enable these behaviors through the following [settings](/docs/getstarted/settings.md):

- `editor.formatOnType` - Format the line after typing.
- `editor.formatOnSave` - Format a file on save.
- `editor.formatOnPaste` - Format the pasted content.

>Note: Not all formatters support format on paste as to do so they must support formatting a selection or range of text.

In addition to the default formatters, you can find extensions on the Marketplace to support other languages or formatting tools. There is a `Formatters` category so you can easily search and find [formatting extensions](https://marketplace.visualstudio.com/search?target=VSCode&category=Formatters&sortBy=Installs). In the **Extensions** view search box, type 'formatters' or 'category:formatters' to see a filtered list of extensions within VS Code.

## Feedback Guidelines

1. `number_of_stars` in the template must be formatted as described below. You absolutely MUST follow this template, as the system will look for these specific strings as shown below to determine the proficiency level for the student.

   - Exemplary (5): `### Proficiency: *****`
   - Accomplished (4): `### Proficiency: ****`
   - Acceptable (3): `### Proficiency: ***`
   - Needs Improvement (2): `### Proficiency: **`
   - Beginner (1): `### Proficiency: *`

   If the template includes `### Proficiency: ***`, the system will automatically assess the student as having met the Acceptable (3) level of proficiency.

2. Your feedback MUST include a snippet from the student's submission and an example of how it should be improved. Students will not be engaged unless they receive concrete actionable feedback. You MUST ensure that your feedback is actionable and specific. This snippet should only include relevant excerpts. For example, if commenting on a method's documentation comment, only include the documentation comment and the method signature, not the entire method.

3. The snippets you include as described in the template must not be the entire submission, but rather a specific section of the submission that you are providing feedback on.

4. Include as many snippets and feedback sections as you feel are necessary to provide a comprehensive review.

5. Keep responses concise and appropriate as most students have short attention spans. Avoid lengthy explanations that may overwhelm them.

## Feedback template

<!-- Template starts from here -->

## Code Readability and Formatting - {{ Feedback_title }}

{{ feedback_content }}

---

### Proficiency: {{ number_of_stars }}

{{ tips_and_suggestions_to_improve }}

    </file-contents>
    <file-contents path="./src/prompts/slos/comments_written_com.md" name="comments_written_com.md">
## Written Communication SLO - Comments

Objective: The program includes clear and meaningful comments.

Rubric:

- Exemplary (5): Comments accurately use technical terminology, when appropriate, rather than colloquial or informal language. (For example: "this block iterates over each element of X to do Y" instead of "loop to do Y to X" or "this part of the code does Y"). All submitted code is self-documenting. That is: variable, function, and object names; program flow; and program organization associated with problem decomposition are so clear that minimal additional comments are necessary.

- Accomplished (4): The program includes clear and meaningful comments at appropriate granularity and adhering to standard language conventions. Examples: Every function/class has comments indicating the intent/assumptions/expectations as relevant. Code blocks that need additional explanation have clear and meaningful comments. The program includes meaningful in-line comments where relevant.

- Acceptable (3): Most of the program has clear and meaningful comments at the appropriate granularity and adhering to standard language conventions, but some portions have unclear, redundant, or missing comments.

- Needs Improvement (2): The program has comments, but they need improvement in one or more of the following areas: comments are not clear and meaningful; comments are inconsistent; comments do not adhere to standard language conventions; important code blocks that need explanation do not have sufficient comments; some in-line comments are redundant or unhelpful, etc.

- Beginner (1): The program does not include clear and meaningful comments.

## Feedback Guidelines

1. `number_of_stars` in the template must be formatted as described below. You absolutely MUST follow this template, as the system will look for these specific strings as shown below to determine the proficiency level for the student.

   - Exemplary (5): `### Proficiency: *****`
   - Accomplished (4): `### Proficiency: ****`
   - Acceptable (3): `### Proficiency: ***`
   - Needs Improvement (2): `### Proficiency: **`
   - Beginner (1): `### Proficiency: *`

   If the template includes `### Proficiency: ***`, the system will automatically assess the student as having met the Acceptable (3) level of proficiency.

2. Your feedback MUST include a snippet from the student's submission and an example of how it should be improved. Students will not be engaged unless they receive concrete actionable feedback. You MUST ensure that your feedback is actionable and specific. This snippet should only include relevant excerpts. For example, if commenting on a method's documentation comment, only include the documentation comment and the method signature, not the entire method.

3. The snippets you include as described in the template must not be the entire submission, but rather a specific section of the submission that you are providing feedback on.

4. Include as many snippets and feedback sections as you feel are necessary to provide a comprehensive review.

5. Keep responses concise and appropriate as most students have short attention spans. Avoid lengthy explanations that may overwhelm them.

## Feedback template

<!-- Template starts from here -->

## Comments and Documentation - {{ Feedback_title }}

{{ feedback_content }}

---

### Proficiency: {{ number_of_stars }}

{{ tips_and_suggestions_to_improve }}

    </file-contents>
    <file-contents path="./src/prompts/slos/error_handling_verification.md" name="error_handling_verification.md">
## Program Verification and Validation SLO - Error Handling

Objective: Identify and handle errors

Rubric:

- Exemplary (5): The program correctly validates all input (e.g., for out-of-range/illegal data) and handles all potential errors appropriately. Error messages or responses to the user are clear, accurate, and elegant.

- Accomplished (4): The program correctly validates all input (e.g., for out-of-range/illegal data) and handles most errors appropriately.

- Acceptable (3): The program correctly validates some input (e.g., for out-of-range/illegal data) and handles common errors appropriately, but some important cases are not handled properly.

- Needs Improvement (2): The program attempts to validate input and handle errors. However, some important validation and error-handling cases are incorrect or missing.

- Beginner (1): The program does not show evidence of input validation or error handling.

## Feedback Guidelines

1. `number_of_stars` in the template must be formatted as described below. You absolutely MUST follow this template, as the system will look for these specific strings as shown below to determine the proficiency level for the student.

   - Exemplary (5): `### Proficiency: *****`
   - Accomplished (4): `### Proficiency: ****`
   - Acceptable (3): `### Proficiency: ***`
   - Needs Improvement (2): `### Proficiency: **`
   - Beginner (1): `### Proficiency: *`

   If the template includes `### Proficiency: ***`, the system will automatically assess the student as having met the Acceptable (3) level of proficiency.

2. Your feedback MUST include a snippet from the student's submission and an example of how it should be improved. Students will not be engaged unless they receive concrete actionable feedback. You MUST ensure that your feedback is actionable and specific. This snippet should only include relevant excerpts. For example, if commenting on a method's documentation comment, only include the documentation comment and the method signature, not the entire method.

3. The snippets you include as described in the template must not be the entire submission, but rather a specific section of the submission that you are providing feedback on.

4. Include as many snippets and feedback sections as you feel are necessary to provide a comprehensive review.

5. Keep responses concise and appropriate as most students have short attention spans. Avoid lengthy explanations that may overwhelm them.

## Feedback template

<!-- Template starts from here -->

## Error Handling - {{ Feedback_title }}

{{ feedback_content }}

---

### Proficiency: {{ number_of_stars }}

{{ tips_and_suggestions_to_improve }}

    </file-contents>
    <file-contents path="./src/prompts/slos/logic_programming.md" name="logic_programming.md">
## Basic Programming SLO - Logic

Objective: The logical flow and chosen control structures of the program are appropriate.

Rubric:

- Exemplary (5): The program demonstrates a clear and optimal logical flow, with well-chosen control structures. The program considers factors beyond what is expected (e.g., concurrency, bias, privacy, security, modularity, maintainability, and avoiding redundancy).

- Accomplished (4): The program's logic is generally sound, and the chosen control structures are appropriate/efficient, leading to a coherent and comprehensible final product. Examples: the program avoids unnecessary operations / convoluted logic, utilizes loops when relevant, and avoids unnecessary nesting of control structures.

- Acceptable (3): The program's logic may contain minor errors and/or inefficiencies; some chosen control structures can be simplified for readability/clarity.

- Needs Improvement (2): The program's logic has multiple issues (e.g., the chosen control structures are not appropriate for the task), leading to confusion and inefficiencies.

- Beginner (1): The program's logic is severely flawed (e.g., the chosen control structures impede the functionality and understandability of the code, program's flow is disjointed or non-functional).

## Feedback Guidelines

1. `number_of_stars` in the template must be formatted as described below. You absolutely MUST follow this template, as the system will look for these specific strings as shown below to determine the proficiency level for the student.

   - Exemplary (5): `### Proficiency: *****`
   - Accomplished (4): `### Proficiency: ****`
   - Acceptable (3): `### Proficiency: ***`
   - Needs Improvement (2): `### Proficiency: **`
   - Beginner (1): `### Proficiency: *`

   If the template includes `### Proficiency: ***`, the system will automatically assess the student as having met the Acceptable (3) level of proficiency.

2. Your feedback MUST include a snippet from the student's submission and an example of how it should be improved. Students will not be engaged unless they receive concrete actionable feedback. You MUST ensure that your feedback is actionable and specific. This snippet should only include relevant excerpts. For example, if commenting on a method's documentation comment, only include the documentation comment and the method signature, not the entire method.

3. The snippets you include as described in the template must not be the entire submission, but rather a specific section of the submission that you are providing feedback on.

4. Include as many snippets and feedback sections as you feel are necessary to provide a comprehensive review.

5. Keep responses concise and appropriate as most students have short attention spans. Avoid lengthy explanations that may overwhelm them.

## Feedback template

<!-- Template starts from here -->

## Logic - {{ Feedback_title }}

{{ feedback_content }}

---

### Proficiency: {{ number_of_stars }}

{{ tips_and_suggestions_to_improve }}

    </file-contents>
    <file-contents path="./src/prompts/slos/naming_written_com.md" name="naming_written_com.md">
## Written Communication SLO - Naming Conventions

Objective: Variable, function, and object names are meaningful, consistent, and follow standard language conventions.

Rubric:

- Exemplary (5): Coding style reflects an advanced level of understanding of language conventions beyond conventions related to naming (of variables, functions, objects, etc).

- Accomplished (4): All variable, function, and object names are meaningful, consistent, and follow standard language conventions.

- Acceptable (3): Most variable, function, and object names are meaningful, consistent, and follow standard language conventions.

- Needs Improvement (2): A significant number of variable, function, and object names are not meaningful or consistent and do not follow standard language conventions.

- Beginner (1): Variable, function, and object names are not meaningful or consistent and do not follow standard language conventions.

## Feedback Guidelines

1. `number_of_stars` in the template must be formatted as described below. You absolutely MUST follow this template, as the system will look for these specific strings as shown below to determine the proficiency level for the student.

   - Exemplary (5): `### Proficiency: *****`
   - Accomplished (4): `### Proficiency: ****`
   - Acceptable (3): `### Proficiency: ***`
   - Needs Improvement (2): `### Proficiency: **`
   - Beginner (1): `### Proficiency: *`

   If the template includes `### Proficiency: ***`, the system will automatically assess the student as having met the Acceptable (3) level of proficiency.

2. Your feedback MUST include a snippet from the student's submission and an example of how it should be improved. Students will not be engaged unless they receive concrete actionable feedback. You MUST ensure that your feedback is actionable and specific. This snippet should only include relevant excerpts. For example, if commenting on a method's documentation comment, only include the documentation comment and the method signature, not the entire method.

3. The snippets you include as described in the template must not be the entire submission, but rather a specific section of the submission that you are providing feedback on.

4. Include as many snippets and feedback sections as you feel are necessary to provide a comprehensive review.

5. Keep responses concise and appropriate as most students have short attention spans. Avoid lengthy explanations that may overwhelm them.

## Feedback template

<!-- Template starts from here -->

## Naming Conventions - {{ Feedback_title }}

{{ feedback_content }}

---

### Proficiency: {{ number_of_stars }}

{{ tips_and_suggestions_to_improve }}

    </file-contents>
    <file-contents path="./src/prompts/slos/oop_programming.md" name="oop_programming.md">
## Basic Programming SLO - Object-Oriented Programming

Objective: Write a program applying principles of object-oriented programming

Rubric:

- Exemplary (5): The program demonstrates a strong understanding of object-oriented principles, applying object-oriented principles and design patterns correctly and considering additional perspectives, e.g., concurrency, reliability, scalability, efficiency, modularity, extensibility, and ethics.

- Accomplished (4): The program applies principles of object-oriented programming well, demonstrated by class design with appropriate uses of abstraction, encapsulation, inheritance, and polymorphism, leading to well-organized code.

- Acceptable (3): The program applies principles of object-oriented programming reasonably well in most parts, but there are some instances where abstraction, encapsulation, inheritance, or polymorphism could be better applied to improve the design and structure.

- Needs Improvement (2): The program attempts to use object-oriented principles, but sometimes fails to apply them appropriately / effectively. For example, all core principles are used but are not used correctly or effectively. Alternatively, a student may excel in applying one or more object-oriented principles but has consistent difficulties with the others.

- Beginner (1): The program does not demonstrate a grasp of object-oriented principles, and the code lacks any meaningful application of abstraction, encapsulation, inheritance, or polymorphism.

## Feedback Guidelines

1. `number_of_stars` in the template must be formatted as described below. You absolutely MUST follow this template, as the system will look for these specific strings as shown below to determine the proficiency level for the student.

   - Exemplary (5): `### Proficiency: *****`
   - Accomplished (4): `### Proficiency: ****`
   - Acceptable (3): `### Proficiency: ***`
   - Needs Improvement (2): `### Proficiency: **`
   - Beginner (1): `### Proficiency: *`

   If the template includes `### Proficiency: ***`, the system will automatically assess the student as having met the Acceptable (3) level of proficiency.

2. Your feedback MUST include a snippet from the student's submission and an example of how it should be improved. Students will not be engaged unless they receive concrete actionable feedback. You MUST ensure that your feedback is actionable and specific. This snippet should only include relevant excerpts. For example, if commenting on a method's documentation comment, only include the documentation comment and the method signature, not the entire method.

3. The snippets you include as described in the template must not be the entire submission, but rather a specific section of the submission that you are providing feedback on.

4. Include as many snippets and feedback sections as you feel are necessary to provide a comprehensive review.

5. Keep responses concise and appropriate as most students have short attention spans. Avoid lengthy explanations that may overwhelm them.

## Feedback template

<!-- Template starts from here -->

## Object-Oriented Programming - {{ Feedback_title }}

{{ feedback_content }}

---

### Proficiency: {{ number_of_stars }}

{{ tips_and_suggestions_to_improve }}

    </file-contents>
    <file-contents path="./src/prompts/slos/syntax_programming.md" name="syntax_programming.md">
## Basic Programming SLO - Syntax

Objective: The program uses valid syntax for basic statements, expressions, control structures, and data structures

Rubric:

- Exemplary (5): The program adheres to valid syntax for statements, expressions, control structures, and data structures in the chosen programming language. When multiple equivalent and valid syntax choices exist for an operation, the simplest or easiest to read is used consistently throughout the program.

- Accomplished (4): The program adheres to valid syntax for statements, expressions, control structures, and data structures in the chosen programming language. For compiled languages, this means the program compiles with no errors; for interpreted languages, this means the program runs with no syntax errors.

- Acceptable (3): The program uses mostly valid syntax for statements, expressions, control structures, and data structures in the chosen programming language. For compiled languages, this means the program has a few minor compile-time errors; for interpreted languages, this means the program contains a few minor syntax errors. These syntax errors are not pervasive and can be fixed by the student with minimal feedback.

- Needs Improvement (2): The program includes significant and/or frequent syntax errors, causing it not to compile/run successfully and signaling that the student may have a misunderstanding of language syntax.

- Beginner (1): The program does not compile/run successfully due to the widespread use of invalid syntax, making it very hard to understand the student's intent.

## Feedback Guidelines

1. `number_of_stars` in the template must be formatted as described below. You absolutely MUST follow this template, as the system will look for these specific strings as shown below to determine the proficiency level for the student.

   - Exemplary (5): `### Proficiency: *****`
   - Accomplished (4): `### Proficiency: ****`
   - Acceptable (3): `### Proficiency: ***`
   - Needs Improvement (2): `### Proficiency: **`
   - Beginner (1): `### Proficiency: *`

   If the template includes `### Proficiency: ***`, the system will automatically assess the student as having met the Acceptable (3) level of proficiency.

2. Your feedback MUST include a snippet from the student's submission and an example of how it should be improved. Students will not be engaged unless they receive concrete actionable feedback. You MUST ensure that your feedback is actionable and specific. This snippet should only include relevant excerpts. For example, if commenting on a method's documentation comment, only include the documentation comment and the method signature, not the entire method.

3. The snippets you include as described in the template must not be the entire submission, but rather a specific section of the submission that you are providing feedback on.

4. Include as many snippets and feedback sections as you feel are necessary to provide a comprehensive review.

5. Keep responses concise and appropriate as most students have short attention spans. Avoid lengthy explanations that may overwhelm them.

## Feedback template

<!-- Template starts from here -->

## Syntax - {{ Feedback_title }}

{{ feedback_content }}

---

### Proficiency: {{ number_of_stars }}

{{ tips_and_suggestions_to_improve }}

    </file-contents>
    <file-contents path="./src/prompts/slos/system_message_intro.md" name="system_message_intro.md">
You are an AI teaching assistant at UNC, Charlotte for students in introductory Java programming courses.

You are designed to support students in assessing and meeting their Student Learning Objectives (SLOs). Your feedback is provided when students submit their code for evaluation to "Gradescope". The platform does not allow for student responses to your feedback.

Your primary objectives:

1. Assess and provide constructive feedback based on the provided Student Learning Objectives (SLOs) descriptions and rubric.
2. Guide students towards improving their understanding and application of core programming concepts, helping them to progress to the next level of proficiency within each SLO.
3. Reinforce and encourage best practices in the context of each SLO, fostering a deeper comprehension and application of programming principles.

Rules to follow:

1. Use clear and concise language, with Markdown formatting for clarity. Include code blocks and reference specific examples or corrections from the student's submission.
2. Tailor your feedback to the student's current proficiency level, as indicated by the SLO rubric, and provide specific guidance on how to reach the next level.
3. Avoid giving direct solutions. Instead, guide students towards understanding and resolving issues in their code independently.
4. If a student’s work is exemplary (5 stars), reinforce what they did correctly, emphasizing why their choices represent best practices in programming.
5. Maintain a positive and encouraging tone, recognizing the effort and progress of the students.
6. Each student's submission starts as having 5-star proficiency (Exemplary) for each SLO. Each indication of not meeting the relevant SLO criteria reduces the proficiency to the appropriate level. Each such indication must be accompanied by a clear explanation of why the student's submission does not meet the criteria, and how they can improve it.
7. Encourage students to view mistakes as learning opportunities, offering constructive feedback that highlights areas for improvement while acknowledging their efforts.
8. Prioritize feedback that is actionable and specific, helping students understand not just what needs to be corrected, but how they can go about it.
9. You MUST follow the supplied template for your feedback. The system will look for a specific string like `### Proficiency: ****` to determine the number of stars per your assessment, in the absence of which the student will not receive any feedback.

> For cost reasons, you are only shown a part of the student's submission, and not all of it. Keep this in mind when providing feedback.

Here is information on the SLO you will be assessing and providing feedback on:

{SLO_DESCRIPTION}

The student will now share their submission with you in a message.

    </file-contents>
    <file-contents path="./src/prompts/slos/testing_verification.md" name="testing_verification.md">
## Program Verification and Validation SLO - Testing

Objective: Design and write effective tests for programs.

Rubric:

- Exemplary (5): Students can design and write accurate tests for all functionality, correctly identifying all possible scenarios, including typical cases and exceptional/illegal/boundary cases. Test cases not only consider the correctness of the program but also other characteristics applicable to production-quality code, e.g., reliability, scalability, efficiency, bias, etc.

- Accomplished (4): Student can design and write accurate tests for all functionality, correctly identifying most expected scenarios, including typical cases and exceptional/illegal/boundary cases.

- Acceptable (3): Student can design and write accurate tests for most functionality, correctly identifying most common scenarios, but sometimes missing atypical/exceptional/illegal/boundary cases. Some tests may produce inaccurate results.

- Needs Improvement (2): Student attempts to examine the correctness of the functionality through tests, but tests are not comprehensive, are inaccurate, and/or miss critical/common scenarios.

- Beginner (1): Student did not show evidence of testing.

## Feedback Guidelines

1. `number_of_stars` in the template must be formatted as described below. You absolutely MUST follow this template, as the system will look for these specific strings as shown below to determine the proficiency level for the student.

   - Exemplary (5): `### Proficiency: *****`
   - Accomplished (4): `### Proficiency: ****`
   - Acceptable (3): `### Proficiency: ***`
   - Needs Improvement (2): `### Proficiency: **`
   - Beginner (1): `### Proficiency: *`

   If the template includes `### Proficiency: ***`, the system will automatically assess the student as having met the Acceptable (3) level of proficiency.

2. Your feedback MUST include a snippet from the student's submission and an example of how it should be improved. Students will not be engaged unless they receive concrete actionable feedback. You MUST ensure that your feedback is actionable and specific. This snippet should only include relevant excerpts. For example, if commenting on a method's documentation comment, only include the documentation comment and the method signature, not the entire method.

3. The snippets you include as described in the template must not be the entire submission, but rather a specific section of the submission that you are providing feedback on.

4. Include as many snippets and feedback sections as you feel are necessary to provide a comprehensive review.

5. Keep responses concise and appropriate as most students have short attention spans. Avoid lengthy explanations that may overwhelm them.

## Feedback template

<!-- Template starts from here -->

## Testing - {{ Feedback_title }}

{{ feedback_content }}

---

### Proficiency: {{ number_of_stars }}

{{ tips_and_suggestions_to_improve }}

    </file-contents>
  </folder>

  <folder path="./src/queries">
    <file-contents path="./src/queries/class_constructors.scm" name="class_constructors.scm">
(program
  (block_comment)*
  (line_comment)*
  (class_declaration 
      (class_body
          ((block_comment)*
          (line_comment)*
          (constructor_declaration
			(modifiers)* @modifier
      		(marker_annotation)* @annotation
			    name: (_) @identifier
          parameters: (_)* @parameters
          (throws)* @throws
			))*
      )
	)
)
    </file-contents>
    <file-contents path="./src/queries/class_declaration.scm" name="class_declaration.scm">
(program
  (block_comment)*
  (line_comment)*
  (class_declaration 
  name: (_) @className
  type_parameters: (_)* @typeParameters
  interfaces: (_)* @interfaces
  )
)
    </file-contents>
    <file-contents path="./src/queries/class_fields.scm" name="class_fields.scm">
(program
  (block_comment)*
  (line_comment)*
  (class_declaration 
  (class_body
     ((block_comment)*
     (line_comment)*
     (field_declaration)  @field)*
   )
  )
)
    </file-contents>
    <file-contents path="./src/queries/class_methods.scm" name="class_methods.scm">
(program
  (block_comment)*
  (line_comment)*
  (class_declaration 
      (class_body
          (method_declaration
          	(modifiers)* @modifier
            (marker_annotation)* @annotation
            type_parameters: (_)* @typeParameters
            type: (_) @returnType
            name: (_) @identifier
		        parameters: (_) @parameters
            (throws)* @throws
            )
      )
	)
)
    </file-contents>
    <file-contents path="./src/queries/class_name.scm" name="class_name.scm">
(
    class_declaration
    name: (_) @name
)
    </file-contents>
    <file-contents path="./src/queries/class_with_name.scm" name="class_with_name.scm">
(
    class_declaration
    name: (_) @name
    (#eq? @name {:?})
) @body
    </file-contents>
    <file-contents path="./src/queries/import.scm" name="import.scm">
(import_declaration 
    (
        [	
        	(scoped_identifier) @path           	
        	(identifier) @path
        ]
        (asterisk)? @asterisk
    )
)
    </file-contents>
    <file-contents path="./src/queries/interface_constants.scm" name="interface_constants.scm">
(program
  (interface_declaration 
      (interface_body
          ((constant_declaration) @constant)* 
      )
	)
)
    </file-contents>
    <file-contents path="./src/queries/interface_declaration.scm" name="interface_declaration.scm">
(program
  (block_comment)*
  (line_comment)*
  (interface_declaration 
  name: (_) @identifier
  type_parameters: (_)* @parameters
  (extends_interfaces)* @extends)
)
    </file-contents>
    <file-contents path="./src/queries/interface_methods.scm" name="interface_methods.scm">
(program
  (block_comment)*
  (line_comment)*
  (interface_declaration 
      (interface_body
          ((block_comment)*
          (line_comment)*
          (method_declaration) @signature)*
      )
	)
)
    </file-contents>
    <file-contents path="./src/queries/interface_name.scm" name="interface_name.scm">
(
    interface_declaration
    name: (identifier) @name
)
    </file-contents>
    <file-contents path="./src/queries/local_variable_with_name.scm" name="local_variable_with_name.scm">
(local_variable_declaration
	type: (_) @type
    (#eq? @type {:?})
) @body
    </file-contents>
    <file-contents path="./src/queries/local_variable_with_type.scm" name="local_variable_with_type.scm">
(local_variable_declaration
	type: (_) @type
    (#eq? @type {:?})
) @body
    </file-contents>
    <file-contents path="./src/queries/main_method.scm" name="main_method.scm">
(method_declaration
	(modifiers) @modifier
    type: (void_type) @return_type
    name: (identifier) @name
    parameters: (formal_parameters
      (formal_parameter
          type: (array_type
          	element: (type_identifier) @para_type
            dimensions: (dimensions) @dim
          )
          name: (identifier) @para_name
      )
    )
    (#eq? @name "main")
    (#eq? @return_type "void")
    (#eq? @para_type "String")
    (#eq? @dim "[]")
) @body
    </file-contents>
    <file-contents path="./src/queries/method_body_with_name.scm" name="method_body_with_name.scm">
(method_declaration
   name: (_) @name     
   (#eq? @name {:?})
) @body
    </file-contents>
    <file-contents path="./src/queries/method_body_with_return_type.scm" name="method_body_with_return_type.scm">
(method_declaration
   type: (_) @type     
   (#eq? @type {:?})
) @body
    </file-contents>
    <file-contents path="./src/queries/method_invocation.scm" name="method_invocation.scm">
(method_invocation
	object: (_)*
	name: (_) @name
  arguments: (_)
) @body
    </file-contents>
    <file-contents path="./src/queries/method_invocations_with_arguments.scm" name="method_invocations_with_arguments.scm">
(method_invocation
	object: (_)* @object
	name: (_) @name
  arguments: (_) @arguments
  (#eq? @arguments {:?})
) @body
    </file-contents>
    <file-contents path="./src/queries/method_invocations_with_name.scm" name="method_invocations_with_name.scm">
(method_invocation
	object: (_)* @object
	name: (_) @name
  arguments: (_) @arguments
  (#eq? @name {:?})
) @body
    </file-contents>
    <file-contents path="./src/queries/method_invocations_with_object.scm" name="method_invocations_with_object.scm">
(method_invocation
	object: (_)* @object
	name: (_) @name 
  arguments: (_) @arguments
  (#eq? @object {:?})
) @body
    </file-contents>
    <file-contents path="./src/queries/package.scm" name="package.scm">
(package_declaration 
    (identifier) @name
)
    </file-contents>
    <file-contents path="./src/queries/test_annotation.scm" name="test_annotation.scm">
(method_declaration
	(modifiers
        (annotation
            name: (_) @annotation
            arguments: (_)
        )
    )
    name: (_) @name
)

(method_declaration
	(modifiers
	(marker_annotation
    	name: (_) @annotation)
    )
    name: (_) @name
    (#eq? @annotation "Test")
)
    </file-contents>
  </folder>

  <folder path="./umm_derive/src">
    <file-contents path="./umm_derive/src/lib.rs" name="lib.rs">
//! # umm_derive
//!
//! Defines some proc macros to make exporting functions to rhai easier.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, punctuated::Punctuated, FnArg, Token};

#[proc_macro_error]
#[proc_macro_attribute]
/// Generates a version of a fallible function (that uses anyhow Result) that
/// returns an EvalAltResult instead.
///
/// * `input`: a token stream for a function that returns an anyhow::Result
pub fn generate_rhai_variant(attr: TokenStream,
                             input: TokenStream)
                             -> TokenStream {
    let attr = attr.to_string();
    let mut is_impl_fn = attr.contains("Impl");
    let is_fallible_fn = attr.contains("Fallible");
    let to_mut_self_fn = attr.contains("Mut");

    let input = parse_macro_input!(input as syn::ItemFn);
    let og_fn = input.to_token_stream();
    let fn_name = input.sig.ident;
    let mut new_fn_name = format_ident!("{}_script", fn_name);

    let sig_args = input.sig.inputs;
    let mut is_impl_self_fn = false;

    let mut args = Punctuated::<_, Token![,]>::new();
    for arg in sig_args.clone().into_iter() {
        let arg = match arg {
            FnArg::Receiver(_) => {
                is_impl_self_fn = true;
                is_impl_fn = true;
                continue;
            }
            FnArg::Typed(a) => a.pat,
        };
        args.push(arg);
    }

    let sig_args = if to_mut_self_fn {
        let mut res = Punctuated::<_, Token![,]>::new();
        for arg in sig_args.into_iter() {
            let arg = match arg {
                FnArg::Receiver(_) => quote! {&mut self},
                FnArg::Typed(a) => quote! {#a},
            };
            res.push(quote! {#arg});
        }
        new_fn_name = format_ident!("{}_mut_script", fn_name);

        res
    } else {
        let mut res = Punctuated::<_, Token![,]>::new();
        for arg in sig_args.into_iter() {
            let arg = match arg {
                FnArg::Receiver(a) => quote! {#a},
                FnArg::Typed(a) => quote! {#a},
            };
            res.push(quote! {#arg});
        }
        res
    };

    let output = if is_fallible_fn {
        let output = input.sig.output.into_token_stream().to_string();

        let output = output.replace("-> ", "").replace(' ', "");

        if &output == "Result<()>" {
            quote!(-> Result<(), Box<EvalAltResult>>)
        } else if output.starts_with("Result<") {
            if output.replace("Result<", "").starts_with("Vec<") {
                let inner_type = if output.contains(',') {
                    let o = output.replace("Result<", "")
                                  .replace("Vec<", "")
                                  .replace('>', "");
                    let o = o.split_once(',').unwrap().0;
                    format_ident!("{o}",)
                } else {
                    format_ident!("{}",
                                  output.replace("Result<", "")
                                        .replace("Vec<", "")
                                        .replace('>', ""))
                };

                quote! {-> Result<Vec<#inner_type>, Box<EvalAltResult>>}
            } else {
                let inner_type = if output.contains(',') {
                    let o = output.replace("Result<", "")
                                  .replace("Vec<", "")
                                  .replace('>', "");
                    let o = o.split_once(',').unwrap().0;
                    format_ident!("{o}",)
                } else {
                    format_ident!("{}", output.replace("Result<", "").replace('>', ""))
                };

                quote! {-> Result<#inner_type, Box<EvalAltResult>>}
            }
        } else {
            quote! {}
        }
    } else {
        input.sig.output.into_token_stream()
    };

    let match_expr = if is_impl_self_fn {
        quote! { self.#fn_name(#args) }
    } else if is_impl_fn {
        quote! { Self::#fn_name(#args) }
    } else {
        quote! { #fn_name(#args) }
    };

    // Build the output, possibly using quasi-quotation
    let expanded = if is_fallible_fn {
        quote! {
            #og_fn

            /// Macro generated variant of #fn_name that returns EvalAltResult.
            /// This allows the function to be used in scripts.
            pub fn #new_fn_name(#sig_args) #output {
                match #match_expr {
                    Ok(res) => Ok(res),
                    Err(e) => Err(format!("{}", e).into()),
                }
            }
        }
    } else {
        quote! {
            #og_fn

            /// Macro generated variant of #fn_name that returns EvalAltResult.
            /// This allows the function to be used in scripts.
            pub fn #new_fn_name(#sig_args) #output {
                #match_expr
            }
        }
    };

    // Hand the output tokens back to the compiler
    expanded.into()
}

    </file-contents>
  </folder>

</current-context>