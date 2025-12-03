use std::{collections::HashSet, ffi::OsString, time::Duration};

use anyhow::{Context, Result, anyhow};
use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
};
use tabled::tables::ExtendedTable;
use tokio::fs as async_fs;

use super::{
    diagnostics::MutationDiagnostic,
    results::{Grade, GradeResult},
};
use crate::{
    config,
    java::{
        File, JavaFileError, Project, ProjectPaths,
        parsers::parser,
        util::{classpath, java_path},
    },
    process::{self, StdinSource},
    retrieval::build_context_message,
    types::LineRef,
};

/// Aggregated result of running a single test file.
struct TestRunOutcome {
    /// Number of tests that passed inside the file.
    tests_passed: f64,
    /// Total number of tests executed inside the file.
    tests_total:  f64,
    /// Prompt messages to append to the overall feedback.
    messages:     Vec<ChatCompletionRequestMessage>,
}

/// Normalized configuration extracted from the Rhai mutation grader inputs.
struct MutationInputs {
    /// Fully qualified test classes to run.
    target_tests:     Vec<String>,
    /// Source classes subject to mutation testing.
    target_classes:   Vec<String>,
    /// Methods to ignore when mutating.
    excluded_methods: Vec<String>,
    /// Classes whose calls should be avoided during mutation.
    avoid_calls_to:   Vec<String>,
}
#[derive(Clone, Default)]
/// Grades by running tests, and reports how many tests pass.
/// Final grade is the same percentage of maximum grade as the number of tests
/// passing.
pub struct ByUnitTestGrader {
    /// A list of test files to run.
    test_files:     Vec<String>,
    /// A list of test names that should be found. Grade returned is 0 if any
    /// are not found.
    expected_tests: Vec<String>,
    /// A reference to the project the test files belong to.
    project:        Project,
    /// Maximum possible grade.
    out_of:         f64,
    /// Display name for requirement to use while displaying grade result
    req_name:       String,
}

impl ByUnitTestGrader {
    /// Getter for test_files
    pub fn test_files(&self) -> &[String] {
        &self.test_files
    }

    /// Setter for test_files
    pub fn set_test_files<I>(mut self, test_files: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.test_files = test_files
            .into_iter()
            .map(|item| item.as_ref().to_owned())
            .collect();
        self
    }

    /// Getter for expected_tests
    pub fn expected_tests(&self) -> &[String] {
        &self.expected_tests
    }

    /// Setter for expected_tests
    pub fn set_expected_tests<I>(mut self, expected_tests: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.expected_tests = expected_tests
            .into_iter()
            .map(|item| item.as_ref().to_owned())
            .collect();
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
    pub async fn grade_by_tests(self) -> Result<GradeResult> {
        let ByUnitTestGrader {
            test_files,
            expected_tests,
            project,
            out_of,
            req_name,
        } = self;

        let prompts = config::java_prompts();
        let files = Self::resolve_test_files(&project, &test_files)
            .context("While resolving test files for execution")?;

        let mut reasons = Self::expected_mismatches(&files, &expected_tests);
        let system_prompt = prompts.system_message().to_string();
        let system_message = Self::build_system_message(system_prompt.clone())
            .context("Failed to build initial system message")?;

        if !reasons.is_empty() {
            reasons.push("Tests will not be run until above is fixed.".into());
            let reasons_body = reasons.join("\n");
            let user_message = Self::build_user_message(reasons_body.clone())
                .context("Failed to build expected-test failure message")?;

            return Ok(GradeResult {
                requirement: req_name,
                grade:       Grade::new(0.0, out_of),
                reason:      reasons_body,
                prompt:      Some(vec![system_message, user_message]),
            });
        }

        let mut total_passed = 0.0;
        let mut total_tests = 0.0;
        let mut messages = vec![system_message];

        for file in &files {
            let outcome = Self::run_tests_for_file(&project, file)
                .await
                .with_context(|| format!("While executing tests in {}", file.proper_name()))?;
            total_passed += outcome.tests_passed;
            total_tests += outcome.tests_total;
            messages.extend(outcome.messages);
        }

        let grade_value = if total_tests > 0.0 {
            (total_passed / total_tests) * out_of
        } else {
            0.0
        };

        Ok(GradeResult {
            requirement: req_name,
            grade:       Grade::new(grade_value, out_of),
            reason:      format!("- {total_passed}/{total_tests} tests passing."),
            prompt:      Some(messages),
        })
    }

    /// Resolves string-based test file names into `File` handles.
    fn resolve_test_files(project: &Project, names: &[String]) -> Result<Vec<File>> {
        names
            .iter()
            .map(|name| {
                project
                    .identify(name)
                    .with_context(|| format!("Failed to identify test file \"{name}\""))
            })
            .collect()
    }

    /// Compares expected test names against discovered tests and reports
    /// mismatches.
    fn expected_mismatches(files: &[File], expected_tests: &[String]) -> Vec<String> {
        let mut reasons = Vec::new();
        if expected_tests.is_empty() {
            return reasons;
        }

        let mut actual_tests = Vec::new();
        for file in files {
            actual_tests.extend(file.test_methods());
        }
        actual_tests.sort();

        let mut expected = expected_tests.to_vec();
        expected.sort();

        let expected_full: HashSet<&str> = expected
            .iter()
            .filter(|value| value.contains('#'))
            .map(|value| value.as_str())
            .collect();
        let expected_methods: HashSet<&str> = expected
            .iter()
            .filter(|value| !value.contains('#'))
            .map(|value| value.as_str())
            .collect();

        for expected_entry in &expected {
            let method_name = expected_entry
                .split_once('#')
                .map(|(_, method)| method)
                .unwrap_or(expected_entry.as_str());
            let missing = if expected_entry.contains('#') {
                !actual_tests.contains(expected_entry)
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
            let expected_match =
                expected_full.contains(actual.as_str()) || expected_methods.contains(method_name);
            if !expected_match {
                reasons.push(format!("- Unexpected test called {method_name}"));
            }
        }

        reasons
    }

    /// Builds a user message suitable for inclusion in prompts, truncating if
    /// required.
    fn build_user_message(mut content: String) -> Result<ChatCompletionRequestMessage> {
        if content.len() > config::PROMPT_TRUNCATE {
            content.truncate(config::PROMPT_TRUNCATE);
            content.push_str("...[TRUNCATED]");
        }

        ChatCompletionRequestUserMessageArgs::default()
            .content(content)
            .name("Student".to_string())
            .build()
            .map(Into::into)
            .map_err(|err| anyhow!("Failed to build user message: {err}"))
    }

    /// Builds a system message from the supplied content.
    fn build_system_message(content: String) -> Result<ChatCompletionRequestMessage> {
        ChatCompletionRequestSystemMessageArgs::default()
            .content(content)
            .name("Instructor".to_string())
            .build()
            .map(Into::into)
            .map_err(|err| anyhow!("Failed to build system message: {err}"))
    }

    /// Drops diagnostics that refer to files outside the current project.
    fn filter_known_diags<T>(project: &Project, diags: Vec<T>) -> Vec<T>
    where
        T: Into<LineRef> + Clone,
    {
        diags
            .into_iter()
            .filter(|d| {
                let lr: LineRef = d.clone().into();
                project.identify(lr.file_name()).is_ok()
            })
            .collect()
    }

    /// Normalizes JUnit stacktraces, removing external frames and decoding
    /// escapes.
    fn process_junit_stacktrace(
        project: &Project,
        stacktrace: &str,
    ) -> (Vec<String>, Vec<LineRef>) {
        let mut updated = Vec::new();
        let mut diags = Vec::new();

        for line in stacktrace.lines() {
            if line.contains("MethodSource") || line.contains("Native Method") {
                continue;
            }

            if line.contains("Test run finished after") {
                break;
            }

            if let Ok(diag) = parser::junit_stacktrace_line_ref(line) {
                if project.contains(&diag.file_name) {
                    updated.push(Self::unescape_stacktrace_line(line));
                }
                diags.push(diag);
            } else {
                updated.push(Self::unescape_stacktrace_line(line));
            }
        }

        (updated, diags)
    }

    /// Replaces escaped characters emitted by JUnit with literal values.
    fn unescape_stacktrace_line(line: &str) -> String {
        line.replace("\\\\", "\\").replace("\\\"", "\"")
    }

    /// Parses the JUnit summary table to collect total and passing test counts.
    fn parse_summary_counts(summary: &str) -> (f64, f64) {
        let mut passed = 0.0;
        let mut total = 0.0;
        for line in summary.lines() {
            if let Ok(value) = parser::num_tests_passed(line) {
                passed = value as f64;
            }
            if let Ok(value) = parser::num_tests_found(line) {
                total = value as f64;
            }
        }
        (passed, total)
    }

    /// Runs the given test file and returns aggregated output and prompt
    /// messages.
    async fn run_tests_for_file(project: &Project, file: &File) -> Result<TestRunOutcome> {
        match file.test(Vec::new(), Some(project)).await {
            Ok(output) => {
                let (tests_passed, tests_total) = Self::parse_summary_counts(&output);
                Ok(TestRunOutcome {
                    tests_passed,
                    tests_total,
                    messages: Vec::new(),
                })
            }
            Err(JavaFileError::FailedTests {
                test_results,
                diags,
            }) => {
                let (normalized, _) = Self::process_junit_stacktrace(project, &test_results);
                let grader_output = normalized.join("\n");
                let mut messages = Vec::new();
                messages.push(
                    Self::build_user_message(format!(
                        "Failed tests -\n```\n{}\n```",
                        grader_output
                    ))
                    .context("Failed to build failed-tests message")?,
                );
                messages.push(
                    build_context_message(
                        project,
                        Some(grader_output.clone()),
                        Self::filter_known_diags(project, diags),
                    )
                    .with_context(|| {
                        format!(
                            "Failed to build retrieval context for failed tests in {}",
                            file.proper_name()
                        )
                    })?,
                );
                let (tests_passed, tests_total) = Self::parse_summary_counts(&test_results);
                Ok(TestRunOutcome {
                    tests_passed,
                    tests_total,
                    messages,
                })
            }
            Err(JavaFileError::Unknown(err)) => {
                let body = format!("Unknown error -\n```\n{:#?}\n```", err);
                let message = Self::build_user_message(body)
                    .context("Failed to build unknown-error message")?;
                Ok(TestRunOutcome {
                    tests_passed: 0.0,
                    tests_total:  0.0,
                    messages:     vec![message],
                })
            }
            Err(JavaFileError::DuringCompilation { stacktrace, diags }) => {
                let body = format!("Compiler error -\n```\n{}\n```", stacktrace);
                let mut messages = Vec::new();
                messages.push(
                    Self::build_user_message(body)
                        .context("Failed to build compiler-error message")?,
                );
                messages.push(
                    build_context_message(project, None, Self::filter_known_diags(project, diags))
                        .with_context(|| {
                            format!(
                                "Failed to build retrieval context for compiler errors in {}",
                                file.proper_name()
                            )
                        })?,
                );
                Ok(TestRunOutcome {
                    tests_passed: 0.0,
                    tests_total: 0.0,
                    messages,
                })
            }
            Err(JavaFileError::AtRuntime { output, diags }) => {
                let body = format!("Error at runtime -\n```\n{}\n```", output);
                let mut messages = Vec::new();
                messages.push(
                    Self::build_user_message(body)
                        .context("Failed to build runtime-error message")?,
                );
                messages.push(
                    build_context_message(project, None, Self::filter_known_diags(project, diags))
                        .with_context(|| {
                            format!(
                                "Failed to build retrieval context for runtime errors in {}",
                                file.proper_name()
                            )
                        })?,
                );
                Ok(TestRunOutcome {
                    tests_passed: 0.0,
                    tests_total: 0.0,
                    messages,
                })
            }
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
    pub target_test:      Vec<String>,
    /// List of classes to mutate.
    pub target_class:     Vec<String>,
    /// List of methods to exclude from mutation.
    pub excluded_methods: Vec<String>,
    /// List of classes to avoid mutating.
    pub avoid_calls_to:   Vec<String>,
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
    pub fn get_target_test(&self) -> &[String] {
        &self.target_test
    }

    /// A getter for the list of classes to mutate.
    pub fn get_target_class(&self) -> &[String] {
        &self.target_class
    }

    /// A getter for the list of methods to exclude from mutation.
    pub fn get_excluded_methods(&self) -> &[String] {
        &self.excluded_methods
    }

    /// A getter for the list of classes to avoid mutating.
    pub fn get_avoid_calls_to(&self) -> &[String] {
        &self.avoid_calls_to
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
    pub fn set_target_test<I>(mut self, target_test: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.target_test = target_test
            .into_iter()
            .map(|item| item.as_ref().to_owned())
            .collect();
        self
    }

    /// A setter for the list of classes to mutate.
    pub fn set_target_class<I>(mut self, target_class: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.target_class = target_class
            .into_iter()
            .map(|item| item.as_ref().to_owned())
            .collect();
        self
    }

    /// A setter for the list of methods to exclude from mutation.
    pub fn set_excluded_methods<I>(mut self, excluded_methods: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.excluded_methods = excluded_methods
            .into_iter()
            .map(|item| item.as_ref().to_owned())
            .collect();
        self
    }

    /// A setter for the list of classes to avoid mutating.
    pub fn set_avoid_calls_to<I>(mut self, avoid_calls_to: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.avoid_calls_to = avoid_calls_to
            .into_iter()
            .map(|item| item.as_ref().to_owned())
            .collect();
        self
    }

    /// Runs mutation tests using ![Pitest](http://pitest.org/) to grade unit tests written by students.
    pub async fn grade_unit_tests(&self) -> Result<GradeResult> {
        eprintln!("Running Mutation tests -");

        let req_name = self.get_req_name();
        let out_of = self.get_out_of();
        let inputs = self
            .normalize_inputs()
            .context("Failed to interpret mutation grader configuration")?;
        let project = Project::new().context("Failed to discover project for mutation grader")?;

        let args = Self::build_mutation_args(&project, &inputs)
            .context("Failed to assemble mutation testing arguments")?;

        let collected = Self::run_mutation_command(&project, &args)
            .await
            .context("Failed to execute PIT mutation coverage report")?;

        let prompts = config::java_prompts();

        if collected.status.success() {
            Self::handle_success(&project, &prompts, inputs, req_name, out_of).await
        } else {
            Self::handle_failure(&prompts, collected, inputs, req_name, out_of)
        }
    }

    /// Normalizes configured mutation grader inputs into owned collections.
    fn normalize_inputs(&self) -> Result<MutationInputs> {
        Ok(MutationInputs {
            target_tests:     self.target_test.clone(),
            target_classes:   self.target_class.clone(),
            excluded_methods: self.excluded_methods.clone(),
            avoid_calls_to:   self.avoid_calls_to.clone(),
        })
    }

    /// Builds the argument list used to invoke PIT mutation testing.
    fn build_mutation_args(project: &Project, inputs: &MutationInputs) -> Result<Vec<OsString>> {
        let class_path = classpath(project.paths())
            .context("Failed to construct classpath for mutation grader")?;
        let source_dirs = [
            project.paths().source_dir().to_str().unwrap_or("."),
            project.paths().root_dir().to_str().unwrap_or("."),
        ]
        .join(",");

        Ok(vec![
            "--class-path".into(),
            class_path.into(),
            "org.pitest.mutationtest.commandline.MutationCoverageReport".into(),
            "--reportDir".into(),
            "test_reports".into(),
            "--failWhenNoMutations".into(),
            "true".into(),
            "--threads".into(),
            "6".into(),
            "--targetClasses".into(),
            inputs.target_classes.join(",").into(),
            "--targetTests".into(),
            inputs.target_tests.join(",").into(),
            "--sourceDirs".into(),
            source_dirs.into(),
            "--timestampedReports".into(),
            "false".into(),
            "--outputFormats".into(),
            "HTML,CSV".into(),
            "--mutators".into(),
            "STRONGER".into(),
            "--excludedMethods".into(),
            inputs.excluded_methods.join(",").into(),
            "--avoidCallsTo".into(),
            inputs.avoid_calls_to.join(",").into(),
        ])
    }

    /// Executes PIT mutation testing and returns the collected process output.
    async fn run_mutation_command(
        project: &Project,
        args: &[OsString],
    ) -> Result<process::Collected> {
        let java = java_path().context("Failed to locate java runtime for mutation grader")?;
        process::run_collect(
            java.as_os_str(),
            args,
            StdinSource::Null,
            Some(project.paths().root_dir()),
            &[],
            Some(config::java_timeout()),
        )
        .await
        .with_context(|| "Failed to spawn or monitor mutation coverage process")
    }

    /// Processes a successful PIT run by parsing reports and assembling
    /// prompts.
    async fn handle_success(
        project: &Project,
        prompts: &crate::java::JavaPrompts,
        inputs: MutationInputs,
        req_name: String,
        out_of: f64,
    ) -> Result<GradeResult> {
        let surviving = Self::load_surviving_mutations(project)
            .await
            .context("While loading mutation report")?;
        let penalty = surviving.len() as f64 * 4.0;

        eprintln!("Ran mutation tests for {} -", inputs.target_tests.join(", "));
        eprintln!("Problematic mutation test failures printed above.");

        let prompt = Self::build_mutation_success_prompt(project, prompts, &inputs, &surviving)
            .context("Failed to build mutation failure prompt")?;

        let grade_value = (out_of - penalty).max(0.0);

        Ok(GradeResult {
            requirement: req_name,
            grade: Grade::new(grade_value, out_of),
            reason: format!("-{penalty:.0} Penalty due to surviving mutations"),
            prompt,
        })
    }

    /// Processes a failed PIT run by capturing stderr/stdout into prompts.
    fn handle_failure(
        prompts: &crate::java::JavaPrompts,
        collected: process::Collected,
        inputs: MutationInputs,
        req_name: String,
        out_of: f64,
    ) -> Result<GradeResult> {
        let process::Collected { stdout, stderr, .. } = collected;

        let mut output = [
            String::from_utf8(stderr).context("Failed to decode mutation stderr as utf8")?,
            String::from_utf8(stdout).context("Failed to decode mutation stdout as utf8")?,
        ]
        .concat();

        eprintln!("{output}");
        if output.len() > config::PROMPT_TRUNCATE {
            output.truncate(config::PROMPT_TRUNCATE);
            output.push_str("...[TRUNCATED]");
        }

        let prompt = Self::build_mutation_failure_prompt(prompts, &inputs, output)
            .context("Failed to build mutation failure prompt")?;

        Ok(GradeResult {
            requirement: req_name,
            grade: Grade::new(0.0, out_of),
            reason: String::from("Something went wrong while running mutation tests, skipping."),
            prompt,
        })
    }

    /// Loads the mutation CSV report and extracts surviving mutations.
    async fn load_surviving_mutations(project: &Project) -> Result<Vec<MutationDiagnostic>> {
        let reports_dir = project.paths().root_dir().join("test_reports");
        async_fs::create_dir_all(&reports_dir)
            .await
            .with_context(|| {
                format!("Failed to create reports directory {}", reports_dir.display())
            })?;
        let csv_path = reports_dir.join("mutations.csv");
        let csv_bytes = async_fs::read(&csv_path)
            .await
            .with_context(|| format!("Could not read {}", csv_path.display()))?;
        let csv_contents =
            String::from_utf8(csv_bytes).context("Failed to decode mutations.csv as utf8")?;
        let mut surviving = Vec::new();
        for (index, line) in csv_contents.lines().enumerate() {
            let diag = parser::mutation_report_row(line).with_context(|| {
                format!("While parsing test_reports/mutations.csv (line {})", index + 1)
            })?;
            if diag.result() == "SURVIVED" {
                surviving.push(diag);
            }
        }
        Ok(surviving)
    }

    /// Builds prompt messages describing surviving mutations, if any.
    fn build_mutation_success_prompt(
        project: &Project,
        prompts: &crate::java::JavaPrompts,
        inputs: &MutationInputs,
        surviving: &[MutationDiagnostic],
    ) -> Result<Option<Vec<ChatCompletionRequestMessage>>> {
        if surviving.is_empty() {
            return Ok(None);
        }

        let context = build_context_message(project, None, surviving.to_vec())
            .context("Failed to build retrieval context for surviving mutations")?;

        let mut feedback = ExtendedTable::new(surviving.to_vec()).to_string();
        eprintln!("{feedback}");

        if feedback.len() > config::PROMPT_TRUNCATE {
            feedback.truncate(config::PROMPT_TRUNCATE);
            feedback.push_str("...[TRUNCATED]");
        }

        let mut messages = Vec::new();
        messages.push(
            ByUnitTestGrader::build_system_message(prompts.system_message().to_string())
                .context("Failed to build system prompt for mutation failures")?,
        );
        messages.push(
            ByUnitTestGrader::build_user_message(feedback)
                .context("Failed to build mutation feedback message")?,
        );
        messages.push(context);
        messages.push(
            ByUnitTestGrader::build_system_message(format!(
                include_str!("../prompts/mutation_testing.md"),
                test = inputs.target_tests.join(", "),
                class = inputs.target_classes.join(", ")
            ))
            .context("Failed to build mutation follow-up prompt")?,
        );

        Ok(Some(messages))
    }

    /// Builds prompt messages when the mutation command itself fails.
    fn build_mutation_failure_prompt(
        prompts: &crate::java::JavaPrompts,
        inputs: &MutationInputs,
        output: String,
    ) -> Result<Option<Vec<ChatCompletionRequestMessage>>> {
        if output.is_empty() {
            return Ok(None);
        }

        let mut messages = Vec::new();
        messages.push(
            ByUnitTestGrader::build_system_message(prompts.system_message().to_string())
                .context("Failed to build system prompt for mutation failure")?,
        );
        messages.push(
            ByUnitTestGrader::build_user_message(output)
                .context("Failed to build mutation stderr/stdout message")?,
        );
        messages.push(
            ByUnitTestGrader::build_system_message(format!(
                include_str!("../prompts/mutation_testing_2.md"),
                test = inputs.target_tests.join(", "),
                class = inputs.target_classes.join(", ")
            ))
            .context("Failed to build mutation recovery prompt")?,
        );

        Ok(Some(messages))
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
    pub async fn grade_by_hidden_tests(&self) -> Result<GradeResult> {
        const MAX_HIDDEN_TEST_BYTES: usize = 5 * 1024 * 1024;

        let url = self.url();
        let test_class_name = self.test_class_name();
        let out_of = self.out_of();
        let req_name = self.req_name();

        let client = config::http_client();
        let response = client
            .get(&url)
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .context(format!("Failed to download {url}"))?
            .error_for_status()
            .context(format!("Hidden test download returned error status: {url}"))?;

        if let Some(len) = response.content_length()
            && len as usize > MAX_HIDDEN_TEST_BYTES
        {
            anyhow::bail!(
                "Hidden test download exceeds allowed size ({} bytes > {} bytes)",
                len,
                MAX_HIDDEN_TEST_BYTES
            );
        }

        let test_source = response
            .bytes()
            .await
            .context(format!("Failed to get response as bytes: {url}"))?;

        if test_source.len() > MAX_HIDDEN_TEST_BYTES {
            anyhow::bail!(
                "Hidden test download exceeds allowed size ({} bytes > {} bytes)",
                test_source.len(),
                MAX_HIDDEN_TEST_BYTES
            );
        }

        let root_paths = ProjectPaths::default();
        let path = root_paths
            .root_dir()
            .join(format!("{test_class_name}.java"));
        let tmp_path = path.with_extension("java.download");

        let write_outcome = async {
            async_fs::write(&tmp_path, &test_source)
                .await
                .context("Failed to write hidden test source")?;
            // On Windows, rename fails if the destination exists; remove any stale file
            // first.
            let _ = async_fs::remove_file(&path).await;
            async_fs::rename(&tmp_path, &path)
                .await
                .context("Failed to move hidden test into place")
        }
        .await;

        if let Err(err) = write_outcome {
            let _ = async_fs::remove_file(&tmp_path).await;
            return Err(err);
        }

        let project = match Project::new() {
            Ok(a) => a,
            Err(e) => {
                let _ = async_fs::remove_file(&path).await;
                let _ = async_fs::remove_file(&tmp_path).await;
                return Err(e);
            }
        };

        let grader = ByUnitTestGrader {
            test_files: vec![test_class_name],
            expected_tests: Vec::new(),
            project,
            out_of,
            req_name,
        };

        let out = match grader.grade_by_tests().await {
            Ok(o) => o,
            Err(e) => {
                let _ = async_fs::remove_file(&path).await;
                let _ = async_fs::remove_file(&tmp_path).await;
                return Err(e);
            }
        };

        async_fs::remove_file(&path)
            .await
            .context("Failed to remove hidden test source")?;
        let _ = async_fs::remove_file(&tmp_path).await;
        Ok(out)
    }
}
