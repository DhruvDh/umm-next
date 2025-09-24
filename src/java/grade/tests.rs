use std::{
    collections::HashSet,
    fs,
    io::{BufRead, BufReader, Write},
    process::Command,
};

use anyhow::{Context, Result, anyhow};
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use itertools::Itertools;
use rhai::{Array, Dynamic};
use tabled::tables::ExtendedTable;

use super::{
    context::get_source_context,
    results::{Grade, GradeResult},
};
use crate::{
    config,
    java::{JavaFileError, Project, ProjectPaths},
    parsers::parser,
    util::{classpath, java_path},
};
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
    pub fn test_files(&self) -> Array {
        self.test_files.clone()
    }

    /// Setter for test_files
    pub fn set_test_files(mut self, test_files: Array) -> Self {
        self.test_files = test_files;
        self
    }

    /// Getter for expected_tests
    pub fn expected_tests(&self) -> Array {
        self.expected_tests.clone()
    }

    /// Setter for expected_tests
    pub fn set_expected_tests(mut self, expected_tests: Array) -> Self {
        self.expected_tests = expected_tests;
        self
    }

    /// Getter for project
    pub fn project(&self) -> Project {
        self.project.clone()
    }

    /// Setter for project
    pub fn set_project(mut self, project: Project) -> Self {
        self.project = project;
        self
    }

    /// Getter for out_of
    pub fn out_of(&self) -> f64 {
        self.out_of
    }

    /// Setter for out_of
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// Getter for req_name
    pub fn req_name(&self) -> String {
        self.req_name.clone()
    }

    /// Setter for req_name
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// Grades by running tests, and reports how many tests pass.
    /// Final grade is the same percentage of maximum grade as the number of
    /// tests passing.
    pub fn grade_by_tests(self) -> Result<GradeResult> {
        let convert_to_string = |f: Vec<Dynamic>| -> Result<Vec<String>> {
            f.iter()
                .map(|f| match f.clone().into_string() {
                    Ok(n) => Ok(n),
                    Err(e) => {
                        Err(anyhow!("test_files array has something that's not a string: {}", e))
                    }
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
                let expected_full: HashSet<&str> = expected_tests
                    .iter()
                    .filter(|value| value.contains('#'))
                    .map(|value| value.as_str())
                    .collect();
                let expected_methods: HashSet<&str> = expected_tests
                    .iter()
                    .filter(|value| !value.contains('#'))
                    .map(|value| value.as_str())
                    .collect();

                for expected in &expected_tests {
                    let method_name = expected
                        .split_once('#')
                        .map(|(_, method)| method)
                        .unwrap_or(expected.as_str());
                    let missing = if expected.contains('#') {
                        !actual_tests.contains(expected)
                    } else {
                        !actual_tests.iter().any(|actual| {
                            actual
                                .split_once('#')
                                .map(|(_, method)| method)
                                .unwrap_or(actual.as_str())
                                == method_name
                        })
                    };
                    if missing {
                        reasons.push(format!("- {method_name} not found."));
                    }
                }

                for actual in &actual_tests {
                    let method_name = actual
                        .split_once('#')
                        .map(|(_, method)| method)
                        .unwrap_or(actual.as_str());
                    let expected_match = expected_full.contains(actual.as_str())
                        || expected_methods.contains(method_name);
                    if !expected_match {
                        reasons.push(format!("- Unexpected test called {method_name}"));
                    }
                }
            }

            reasons
        };

        let new_user_message = |content: String| {
            let mut content = content;
            if content.len() > config::PROMPT_TRUNCATE {
                content.truncate(config::PROMPT_TRUNCATE);
                content.push_str("...[TRUNCATED]");
            }

            ChatCompletionRequestUserMessageArgs::default()
                .content(content)
                .name("Student".to_string())
                .build()
                .unwrap()
                .into()
        };
        let new_system_message = |content: String| {
            ChatCompletionRequestSystemMessageArgs::default()
                .content(content)
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
                        updated_stacktrace
                            .push(line.replace("\\\\", "\\").replace("\\\"", "\"").to_string());
                    }
                    all_diags.push(diag);
                } else {
                    updated_stacktrace
                        .push(line.replace("\\\\", "\\").replace("\\\"", "\"").to_string());
                }
            }

            (updated_stacktrace, all_diags)
        };

        let system_message = config::prompts().system_message().to_string();
        let initial_message = new_system_message(system_message.clone());

        if !reasons.is_empty() {
            reasons.push("Tests will not be run until above is fixed.".into());
            let reasons = reasons.join("\n");
            let messages = vec![initial_message, new_user_message(reasons.clone())];
            Ok(GradeResult {
                requirement: req_name,
                grade:       Grade::new(0.0, out_of),
                reason:      reasons,
                prompt:      Some(messages),
            })
        } else {
            let mut num_tests_passed = 0.0;
            let mut num_tests_total = 0.0;
            let mut messages = vec![initial_message.clone()];

            for test_file in test_files {
                let res = match project
                    .identify(test_file.as_str())?
                    .test(Vec::new(), Some(&project))
                {
                    Ok(res) => res,
                    Err(JavaFileError::FailedTests {
                        test_results,
                        diags,
                    }) => {
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
                                config::active_retrieval_enabled(),
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
                        messages.extend(vec![
                            new_user_message(out.clone()),
                            get_source_context(diags, project.clone(), 3, 6, 6, false, None)?,
                        ]);
                        out
                    }
                    Err(JavaFileError::AtRuntime { output, diags }) => {
                        let out = format!("Error at runtime -\n```\n{}\n```", output);
                        messages.extend(vec![
                            new_user_message(out.clone()),
                            get_source_context(diags, project.clone(), 3, 6, 6, false, None)?,
                        ]);
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

            Ok(GradeResult {
                requirement: req_name,
                grade:       Grade::new(grade, out_of),
                reason:      format!("- {num_tests_passed}/{num_tests_total} tests passing."),
                prompt:      Some(messages),
            })
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
    pub fn get_req_name(&self) -> String {
        self.req_name.clone()
    }

    /// A getter for the maximum possible grade.
    pub fn get_out_of(&self) -> f64 {
        self.out_of
    }

    /// A getter for the list of test classes to run.
    pub fn get_target_test(&self) -> Array {
        self.target_test.clone()
    }

    /// A getter for the list of classes to mutate.
    pub fn get_target_class(&self) -> Array {
        self.target_class.clone()
    }

    /// A getter for the list of methods to exclude from mutation.
    pub fn get_excluded_methods(&self) -> Array {
        self.excluded_methods.clone()
    }

    /// A getter for the list of classes to avoid mutating.
    pub fn get_avoid_calls_to(&self) -> Array {
        self.avoid_calls_to.clone()
    }

    /// A setter for the name of the requirement.
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// A setter for the maximum possible grade.
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// A setter for the list of test classes to run.
    pub fn set_target_test(mut self, target_test: Array) -> Self {
        self.target_test = target_test;
        self
    }

    /// A setter for the list of classes to mutate.
    pub fn set_target_class(mut self, target_class: Array) -> Self {
        self.target_class = target_class;
        self
    }

    /// A setter for the list of methods to exclude from mutation.
    pub fn set_excluded_methods(mut self, excluded_methods: Array) -> Self {
        self.excluded_methods = excluded_methods;
        self
    }

    /// A setter for the list of classes to avoid mutating.
    pub fn set_avoid_calls_to(mut self, avoid_calls_to: Array) -> Self {
        self.avoid_calls_to = avoid_calls_to;
        self
    }

    /// Runs mutation tests using ![Pitest](http://pitest.org/) to grade unit tests written by students.
    pub fn grade_unit_tests(&self) -> Result<GradeResult> {
        let req_name = self.get_req_name();
        let out_of = self.get_out_of();
        let target_test = self.get_target_test();
        let target_class = self.get_target_class();
        let excluded_methods = self.get_excluded_methods();
        let avoid_calls_to = self.get_avoid_calls_to();
        let project = Project::new()?;

        eprintln!("Running Mutation tests -");
        let target_test: Vec<String> = target_test
            .iter()
            .map(|f| match f.clone().into_string() {
                Ok(n) => Ok(n),
                Err(e) => {
                    Err(anyhow!("target_test array has something that's not a string: {}", e))
                }
            })
            .try_collect()?;
        let target_class: Vec<String> = target_class
            .iter()
            .map(|f| match f.clone().into_string() {
                Ok(n) => Ok(n),
                Err(e) => {
                    Err(anyhow!("target_class array has something that's not a string: {}", e))
                }
            })
            .try_collect()?;
        let excluded_methods: Vec<String> = excluded_methods
            .iter()
            .map(|f| match f.clone().into_string() {
                Ok(n) => Ok(n),
                Err(e) => {
                    Err(anyhow!("excluded_methods array has something that's not a string: {}", e))
                }
            })
            .try_collect()?;
        let avoid_calls_to: Vec<String> = avoid_calls_to
            .iter()
            .map(|f| match f.clone().into_string() {
                Ok(n) => Ok(n),
                Err(e) => {
                    Err(anyhow!("avoid_calls_to array has something that's not a string: {}", e))
                }
            })
            .try_collect()?;

        let class_path = classpath(project.paths())?;
        let source_dirs = [
            project.paths().source_dir().to_str().unwrap_or("."),
            project.paths().root_dir().to_str().unwrap_or("."),
        ]
        .join(",");

        let child = Command::new(java_path()?)
            .arg("--class-path")
            .arg(class_path.as_str())
            .arg("org.pitest.mutationtest.commandline.MutationCoverageReport")
            .arg("--reportDir")
            .arg("test_reports")
            .arg("--failWhenNoMutations")
            .arg("true")
            .arg("--threads")
            .arg("6")
            .arg("--targetClasses")
            .arg(target_class.join(","))
            .arg("--targetTests")
            .arg(target_test.join(","))
            .arg("--sourceDirs")
            .arg(source_dirs)
            .arg("--timestampedReports")
            .arg("false")
            .arg("--outputFormats")
            .arg("HTML,CSV")
            .arg("--mutators")
            .arg("STRONGER")
            .arg("--excludedMethods")
            .arg(excluded_methods.join(","))
            .arg("--avoidCallsTo")
            .arg(avoid_calls_to.join(","))
            .output()
            .context("Failed to spawn javac process.")?;

        let prompts = config::prompts();

        if child.status.success() {
            fs::create_dir_all("test_reports")?;
            let file = fs::File::open(
                project
                    .paths()
                    .root_dir()
                    .join("test_reports")
                    .join("mutations.csv"),
            )
            .context("Could not read ./test_reports/mutations.csv file".to_string())?;
            let reader = BufReader::new(file);
            let mut diags = vec![];

            for line in reader.lines() {
                let line = line?;
                let parse_result = parser::mutation_report_row(&line)
                    .context("While parsing test_reports/mutations.csv");

                match parse_result {
                    Ok(r) => {
                        if r.result() == "SURVIVED" {
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

                let mut feedback = ExtendedTable::new(diags).to_string();
                eprintln!("{feedback}");

                if feedback.len() > config::PROMPT_TRUNCATE {
                    feedback.truncate(config::PROMPT_TRUNCATE);
                    feedback.push_str("...[TRUNCATED]");
                }

                Some(vec![
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(prompts.system_message().to_string())
                        .name("Instructor".to_string())
                        .build()
                        .context("Failed to build system message")?
                        .into(),
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(feedback)
                        .name("Student".to_string())
                        .build()
                        .context("Failed to build user message")?
                        .into(),
                    context,
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(format!(
                            include_str!("../../prompts/mutation_testing.md"),
                            test = target_test.join(", "),
                            class = target_class.join(", ")
                        ))
                        .name("Instructor".to_string())
                        .build()
                        .context("Failed to build system message")?
                        .into(),
                ])
            } else {
                None
            };

            Ok(GradeResult {
                requirement: req_name,
                grade: Grade::new((out_of as u32).saturating_sub(penalty).into(), out_of),
                reason: format!("-{penalty} Penalty due to surviving mutations"),
                prompt,
            })
        } else {
            let mut output = [
                String::from_utf8(child.stderr)?,
                String::from_utf8(child.stdout)?,
            ]
            .concat();
            eprintln!("{output}");
            if output.len() > config::PROMPT_TRUNCATE {
                output.truncate(config::PROMPT_TRUNCATE);
                output.push_str("...[TRUNCATED]");
            }

            let prompt = if !output.is_empty() {
                Some(vec![
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(prompts.system_message().to_string())
                        .name("Instructor".to_string())
                        .build()
                        .context("Failed to build system message")?
                        .into(),
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(output)
                        .name("Student".to_string())
                        .build()
                        .context("Failed to build user message")?
                        .into(),
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(format!(
                            include_str!("../../prompts/mutation_testing_2.md"),
                            test = target_test.join(", "),
                            class = target_class.join(", ")
                        ))
                        .name("Instructor".to_string())
                        .build()
                        .context("Failed to build system message")?
                        .into(),
                ])
            } else {
                None
            };
            Ok(GradeResult {
                requirement: req_name,
                grade: Grade::new(0.0, out_of),
                reason: String::from(
                    "Something went wrong while running mutation tests, skipping.",
                ),
                prompt,
            })
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
    pub fn url(&self) -> String {
        self.url.clone()
    }

    /// sets the `url` field.
    pub fn set_url(mut self, url: String) -> Self {
        self.url = url;
        self
    }

    /// gets the `test_class_name` field
    pub fn test_class_name(&self) -> String {
        self.test_class_name.clone()
    }

    /// sets the `test_class_name` field
    pub fn set_test_class_name(mut self, test_class_name: String) -> Self {
        self.test_class_name = test_class_name;
        self
    }

    /// gets the `out_of` field
    pub fn out_of(&self) -> f64 {
        self.out_of
    }

    /// sets the `out_of` field
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// gets the `req_name` field
    pub fn req_name(&self) -> String {
        self.req_name.clone()
    }

    /// sets the `req_name` field
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// Grades using hidden tests. Test file is downloaded, ran, and then
    /// cleaned up before returning.
    pub fn grade_by_hidden_tests(&self) -> Result<GradeResult> {
        let url = self.url();
        let test_class_name = self.test_class_name();
        let out_of = self.out_of();
        let req_name = self.req_name();

        let test_source = reqwest::blocking::get(&url)
            .context(format!("Failed to download {url}"))?
            .bytes()
            .context(format!("Failed to get response as bytes: {url}"))?;

        let root_paths = ProjectPaths::default();
        let path = root_paths
            .root_dir()
            .join(format!("{test_class_name}.java"));
        let mut file = fs::File::create(&path)?;
        file.write_all(&test_source)?;

        let project = match Project::new() {
            Ok(a) => a,
            Err(e) => {
                fs::remove_file(&path)?;
                return Err(e);
            }
        };

        let grader = ByUnitTestGrader {
            test_files: vec![Dynamic::from(test_class_name)],
            expected_tests: Array::new(),
            project,
            out_of,
            req_name,
        };

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
