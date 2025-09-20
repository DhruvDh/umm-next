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

use anyhow::{Context, Result, anyhow, ensure};
use async_openai::{
    Client as OpenAIClient,
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequest,
        CreateChatCompletionResponse, ReasoningEffort as ChatReasoningEffort,
    },
};
use colored::Colorize;
use itertools::Itertools;
use rhai::FnPtr;
#[allow(deprecated)]
use rhai::{Array, Dynamic};
use serde::{Deserialize, Serialize};
use similar::{Algorithm, ChangeTag, utils::diff_unicode_words};
use snailquote::unescape;
use tabled::{
    Table, Tabled,
    settings::{Alignment, Modify, Panel, Style, Width, object::Rows},
    tables::ExtendedTable,
};
use typed_builder::TypedBuilder;

use crate::{
    Dict,
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
    pub fn new(grade: f64, out_of: f64) -> Self {
        Self { grade, out_of }
    }

    /// Creates a new grade from a string -
    /// * `grade_string` - A string in the format `grade/out_of`, eg. `10/20`
    pub fn grade_from_string(grade_string: String) -> Result<Grade> {
        let (grade, out_of) = grade_string.split_once('/').unwrap_or(("0", "0"));
        Ok(Grade::new(
            grade.parse::<f64>().context("Failed to parse grade")?,
            out_of.parse::<f64>().context("Failed to parse out of")?,
        ))
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
    pub fn set_grade(mut self, grade: f64) -> Self {
        self.grade = grade;
        self
    }

    /// a setter for the out_of
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }
}

impl Display for Grade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    pub fn set_requirement(mut self, requirement: String) -> Self {
        self.requirement = requirement;
        self
    }

    /// a getter for Reason
    pub fn reason(&mut self) -> String {
        self.reason.clone()
    }

    /// a setter for Reason
    pub fn set_reason(mut self, reason: String) -> Self {
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
    pub fn set_grade(mut self, grade: f64) -> Self {
        self.grade = self.grade.set_grade(grade);
        self
    }

    /// a setter for the self.grade.out_of
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.grade = self.grade.set_out_of(out_of);
        self
    }

    /// a getter for the prompt
    pub fn prompt(&mut self) -> Option<Vec<ChatCompletionRequestMessage>> {
        self.prompt.clone()
    }

    /// a setter for the prompt
    pub fn set_prompt(mut self, prompt: Option<Vec<ChatCompletionRequestMessage>>) -> Self {
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
        LineRef {
            file_name:   val.file_name,
            line_number: val.line_number as usize,
        }
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
        LineRef {
            file_name:   val.source_file_name,
            line_number: val.line_number as usize,
        }
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
pub fn get_active_retrieval_context(
    proj: &Project,
    active_retrieval_context: Option<String>,
) -> Result<ChatCompletionRequestMessage> {
    ensure!(
        active_retrieval_context.is_some(),
        "Additional context must be provided when using active retrieval."
    );

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
    let response: CreateChatCompletionResponse = client
        .post("https://umm-feedback-openai-func.deno.dev/")
        .body(messages)
        .send()?
        .json()?;
    let response = response.choices[0].message.clone();
    println!(" done!");
    ensure!(response.tool_calls.is_some(), "No function call found in response.");
    let function_call_args: RetrievalFunctionCallParamsArray = serde_json::from_str(
        response
            .tool_calls
            .unwrap()
            .first()
            .unwrap()
            .function
            .arguments
            .as_str(),
    )?;

    let mut context = Vec::new();
    for function_call_arg in function_call_args.params {
        let file = proj.identify(&function_call_arg.class_name)?;
        let query = format!(
            include_str!("queries/method_body_with_name.scm"),
            &function_call_arg.method_name
        );

        let res = file
            .query(&query)
            .or_else(|_| Ok::<Vec<Dict>, anyhow::Error>(vec![]))
            .unwrap();

        for r in res {
            let body = r.get("body").unwrap().to_string();
            context.push(format!(
                "Method body from student's submission for `{}#{}`:",
                file.proper_name(),
                function_call_arg.method_name
            ));
            context.push(format!("\n```\n{}\n```\n", body));
        }
    }

    Ok(ChatCompletionRequestSystemMessageArgs::default()
        .content(context.join("\n"))
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
pub fn get_source_context<T: Into<LineRef>>(
    line_refs: Vec<T>,
    proj: Project,
    start_offset: usize,
    num_lines: usize,
    max_line_refs: usize,
    try_use_active_retrieval: bool,
    active_retrieval_context: Option<String>,
) -> Result<ChatCompletionRequestMessage> {
    if try_use_active_retrieval {
        match get_active_retrieval_context(&proj, active_retrieval_context) {
            Ok(message) => return Ok(message),
            Err(e) => {
                eprintln!("Failed to get active retrieval context: {e}");
            }
        }
    }

    let mut line_refs: Vec<(File, LineRef, RangeInclusive<usize>)> = line_refs
        .into_iter()
        .flat_map(|x| {
            let x = x.into();
            let file = proj.identify(&x.file_name)?;
            let start = match file.kind() {
                FileType::Test => x.line_number.saturating_sub(num_lines),
                _ => x.line_number.saturating_sub(start_offset),
            };
            let end = start + num_lines;
            Ok::<(File, LineRef, RangeInclusive<usize>), anyhow::Error>((file, x, start..=end))
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
        "You cannot see all of the student's submission as you are an AI language model, with \
         limited context length. Here are some snippets of code the stacktrace indicates might be \
         relevant:
:\n"
        .to_string(),
    );
    let end_ticks = "\n```\n".to_string();
    let mut methods: HashSet<String> = HashSet::new();

    line_refs
        .into_iter()
        .coalesce(|lhs, rhs| {
            if lhs.0 == rhs.0 {
                let lhs_start = *lhs.2.start();
                let lhs_end = *lhs.2.end();
                let rhs_start = *rhs.2.start();
                let rhs_end = *rhs.2.end();
                let expanded_range = rhs_start.saturating_sub(num_lines)..=(rhs_end + num_lines);

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

            context.push(format!(
                "- Lines {} to {} from {} -\n```",
                *r.start(),
                *r.end(),
                f.file_name
            ));

            let width = (count as f32).log10().ceil() as usize;

            let source_code_lines: Vec<String> =
                file.parser().code().lines().map(String::from).collect();

            let relevant_source = source_code_lines
                .clone()
                .iter()
                .skip(*r.start())
                .take(num_lines)
                .enumerate()
                .map(|(line_n, x)| {
                    format!("{:width$}|{}", *r.start() + line_n, x)
                        .replace("\\\\", "\\")
                        .replace("\\\"", "\"")
                })
                .collect::<Vec<String>>();

            context.append(&mut (relevant_source.clone()));
            context.push(end_ticks.clone());

            match Parser::new(relevant_source.join("\n")) {
                Ok(parser) => {
                    let method_names: Vec<Dict> = parser
                        .query(METHOD_CALL_QUERY)
                        .or_else(|_| Ok::<Vec<Dict>, anyhow::Error>(vec![]))
                        .unwrap();

                    for method in method_names {
                        let method_name = method.get("name").unwrap().to_string();
                        methods.insert(method_name.clone());

                        let query = format!(
                            include_str!("queries/method_body_with_name.scm"),
                            &method_name
                        );

                        for f in proj.files() {
                            if *f.kind() == FileType::Class || *f.kind() == FileType::ClassWithMain
                            {
                                let res = f
                                    .query(&query)
                                    .or_else(|_| Ok::<Vec<Dict>, anyhow::Error>(vec![]))
                                    .unwrap();

                                for r in res {
                                    let body = r.get("body").unwrap().to_string();
                                    let body_lines =
                                        body.lines().map(String::from).collect::<Vec<_>>();
                                    if !body_lines.is_empty() {
                                        let start_line_number = source_code_lines
                                            .iter()
                                            .find_position(|x| {
                                                x.contains(body_lines.first().unwrap().trim())
                                            })
                                            .unwrap_or((0, &String::new()))
                                            .0;

                                        let body = body_lines
                                            .iter()
                                            .enumerate()
                                            .map(|(line_n, x)| {
                                                if start_line_number != 0 {
                                                    format!(
                                                        "{:width$}|{}",
                                                        start_line_number + line_n + 1,
                                                        x
                                                    )
                                                } else {
                                                    x.to_string()
                                                }
                                            })
                                            .collect::<Vec<String>>()
                                            .join("\n");

                                        context.push(format!(
                                            "Method body from student's submission `{}#{}`:",
                                            f.proper_name(),
                                            method_name
                                        ));
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

    Ok(ChatCompletionRequestSystemMessageArgs::default()
        .content(context)
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
    pub fn set_project(mut self, project: Project) -> Self {
        self.project = project;
        self
    }

    /// Getter for files
    pub fn files(&mut self) -> Array {
        self.files.clone()
    }

    /// Setter for files
    pub fn set_files(mut self, files: Array) -> Self {
        self.files = files;
        self
    }

    /// Getter for out_of
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// Setter for out_of
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// Getter for req_name
    pub fn req_name(&mut self) -> String {
        self.req_name.clone()
    }

    /// Setter for req_name
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// Getter for penalty
    pub fn penalty(&mut self) -> f64 {
        self.penalty
    }

    /// Setter for penalty
    pub fn set_penalty(mut self, penalty: f64) -> Self {
        self.penalty = penalty;
        self
    }

    /// Grades documentation by using the -Xdoclint javac flag.
    /// Scans javac output for generated warnings and grades accordingly.
    pub fn grade_docs(self) -> Result<GradeResult> {
        let mut diags = vec![];
        let mut all_diags = vec![];
        let files: Vec<String> = self
            .files
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

                    return Ok(GradeResult {
                        requirement: self.req_name,
                        grade:       Grade::new(0.0, out_of),
                        reason:      String::from("See above."),
                        prompt:      Some(messages),
                    });
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

                    return Ok(GradeResult {
                        requirement: self.req_name,
                        grade:       Grade::new(0.0, out_of),
                        reason:      String::from("See above."),
                        prompt:      Some(messages),
                    });
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
        eprintln!(
            "{}",
            Table::new(&diags)
                .with(Panel::header(format!("Check javadoc for {}", files.join(", "))))
                .with(Panel::footer(format!("-{penalty} due to {num_diags} nits")))
                .with(Modify::new(Rows::new(1..)).with(Width::wrap(24).keep_words(true)))
                .with(
                    Modify::new(Rows::first())
                        .with(Alignment::center())
                        .with(Alignment::center_vertical()),
                )
                .with(
                    Modify::new(Rows::last())
                        .with(Alignment::center())
                        .with(Alignment::center_vertical()),
                )
                .with(Style::modern())
        );

        let prompt = if num_diags > 0 {
            let context = get_source_context(all_diags, self.project, 1, 3, 6, false, None)?;

            let mut outputs = outputs
                .iter()
                .map(|output| format!("```\n{output}\n```"))
                .collect::<Vec<String>>()
                .join("\n\n---\n\n");

            if outputs.len() > PROMPT_TRUNCATE {
                outputs.truncate(PROMPT_TRUNCATE);
                outputs.push_str("...[TRUNCATED]");
            }

            Some(vec![
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(SYSTEM_MESSAGE.to_string())
                    .name("Instructor".to_string())
                    .build()?
                    .into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content(outputs)
                    .name("Student".to_string())
                    .build()?
                    .into(),
                context,
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(include_str!("prompts/javadoc.md").to_string())
                    .name("Instructor".to_string())
                    .build()?
                    .into(),
            ])
        } else {
            None
        };
        Ok(GradeResult {
            requirement: self.req_name,
            grade: Grade::new(grade, out_of),
            reason: String::from("See above."),
            prompt,
        })
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
    pub fn set_test_files(mut self, test_files: Array) -> Self {
        self.test_files = test_files;
        self
    }

    /// Getter for expected_tests
    pub fn expected_tests(&mut self) -> Array {
        self.expected_tests.clone()
    }

    /// Setter for expected_tests
    pub fn set_expected_tests(mut self, expected_tests: Array) -> Self {
        self.expected_tests = expected_tests;
        self
    }

    /// Getter for project
    pub fn project(&mut self) -> Project {
        self.project.clone()
    }

    /// Setter for project
    pub fn set_project(mut self, project: Project) -> Self {
        self.project = project;
        self
    }

    /// Getter for out_of
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// Setter for out_of
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// Getter for req_name
    pub fn req_name(&mut self) -> String {
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

        let initial_message = new_system_message(SYSTEM_MESSAGE.to_string());

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
    pub fn grade_unit_tests(&mut self) -> Result<GradeResult> {
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

        let child = Command::new(java_path()?)
            .args([
                "--class-path",
                classpath()?.as_str(),
                "org.pitest.mutationtest.commandline.MutationCoverageReport",
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
                [
                    SOURCE_DIR.to_str().unwrap_or("."),
                    ROOT_DIR.to_str().unwrap_or("."),
                ]
                .join(",")
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
                avoid_calls_to.join(",").as_str(),
            ])
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
                let parse_result = parser::mutation_report_row(&line)
                    .context("While parsing test_reports/mutations.csv");

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

                let mut feedback = ExtendedTable::new(diags).to_string();
                eprintln!("{feedback}");

                if feedback.len() > PROMPT_TRUNCATE {
                    feedback.truncate(PROMPT_TRUNCATE);
                    feedback.push_str("...[TRUNCATED]");
                }

                Some(vec![
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(SYSTEM_MESSAGE.to_string())
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
                            include_str!("prompts/mutation_testing.md"),
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
            if output.len() > PROMPT_TRUNCATE {
                output.truncate(PROMPT_TRUNCATE);
                output.push_str("...[TRUNCATED]");
            }

            let prompt = if !output.is_empty() {
                Some(vec![
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(SYSTEM_MESSAGE.to_string())
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
                            include_str!("prompts/mutation_testing_2.md"),
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
    pub fn url(&mut self) -> String {
        self.url.clone()
    }

    /// sets the `url` field.
    pub fn set_url(mut self, url: String) -> Self {
        self.url = url;
        self
    }

    /// gets the `test_class_name` field
    pub fn test_class_name(&mut self) -> String {
        self.test_class_name.clone()
    }

    /// sets the `test_class_name` field
    pub fn set_test_class_name(mut self, test_class_name: String) -> Self {
        self.test_class_name = test_class_name;
        self
    }

    /// gets the `out_of` field
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// sets the `out_of` field
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// gets the `req_name` field
    pub fn req_name(&mut self) -> String {
        self.req_name.clone()
    }

    /// sets the `req_name` field
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// Grades using hidden tests. Test file is downloaded, ran, and then
    /// cleaned up before returning.
    pub fn grade_by_hidden_tests(&mut self) -> Result<GradeResult> {
        let url = self.url();
        let test_class_name = self.test_class_name();
        let out_of = self.out_of();
        let req_name = self.req_name();

        let test_source = reqwest::blocking::get(&url)
            .context(format!("Failed to download {url}"))?
            .bytes()
            .context(format!("Failed to get response as bytes: {url}"))?;

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

async fn generate_combined_slo_report(
    slo_responses: Vec<(&str, Result<CreateChatCompletionResponse, OpenAIError>)>,
) -> Result<String> {
    let mut individual_feedbacks = Vec::new();

    for (name, resp) in slo_responses {
        match resp {
            Ok(response) => {
                let content = response
                    .choices
                    .first()
                    .and_then(|choice| choice.message.content.clone())
                    .unwrap_or_default();
                individual_feedbacks.push(format!("SLO: {}\n\n{}", name, content));
            }
            Err(e) => {
                // Log the error or handle it as appropriate for your use case
                eprintln!("Error processing SLO '{}': {:?}", name, e);
                individual_feedbacks
                    .push(format!("SLO: {}\n\nError: Unable to process this SLO.", name));
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

    let messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(
                "You are an AI assistant tasked with creating a concise, well-structured report \
                 that combines feedback from multiple Student Learning Outcomes (SLOs). Your goal \
                 is to provide a comprehensive overview of the student's performance across all \
                 SLOs, highlighting strengths, areas for improvement, and specific \
                 recommendations.",
            )
            .name("Instructor")
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(format!(
                "Please create a combined report based on the following individual SLO \
                 feedbacks:\n\n{}",
                combined_feedback
            ))
            .name("Student")
            .build()?
            .into(),
    ];

    let temp_opt = std::env::var("OPENAI_TEMPERATURE")
        .ok()
        .and_then(|s| s.parse::<f32>().ok());
    let top_p_opt = std::env::var("OPENAI_TOP_P")
        .ok()
        .and_then(|s| s.parse::<f32>().ok());
    let effort = parse_reasoning_effort(std::env::var("OPENAI_REASONING_EFFORT").ok());

    let response = openai_client
        .chat()
        .create(CreateChatCompletionRequest {
            model: std::env::var("OPENAI_MODEL")
                .context("OPENAI_MODEL must be set for SLO feedback")?,
            messages,
            temperature: temp_opt,
            top_p: top_p_opt,
            n: Some(1),
            stream: Some(false),
            reasoning_effort: Some(effort),
            ..Default::default()
        })
        .await?;

    response
        .choices
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
    enabled_slos: &HashSet<String>, // New parameter
) -> Result<Vec<(&'static str, Result<CreateChatCompletionResponse, OpenAIError>)>> {
    let slos = vec![
        (
            "slo_algorithmic_solutions",
            "Algorithmic Solutions",
            ALGORITHMIC_SOLUTIONS_SLO.as_str(),
            SLOFileType::Source,
        ),
        (
            "slo_code_readability",
            "Code Readability and Formatting",
            CODE_READABILITY_SLO.as_str(),
            SLOFileType::SourceAndTest,
        ),
        (
            "slo_comments",
            "Comments",
            COMMENTS_WRITTEN_SLO.as_str(),
            SLOFileType::SourceAndTest,
        ),
        (
            "slo_error_handling",
            "Error Handling",
            ERROR_HANDLING_SLO.as_str(),
            SLOFileType::SourceAndTest,
        ),
        ("slo_logic", "Logic", LOGIC_SLO.as_str(), SLOFileType::SourceAndTest),
        (
            "slo_naming_conventions",
            "Naming Conventions",
            NAMING_CONVENTIONS_SLO.as_str(),
            SLOFileType::SourceAndTest,
        ),
        (
            "slo_oop_programming",
            "Object Oriented Programming",
            OBJECT_ORIENTED_PROGRAMMING_SLO.as_str(),
            SLOFileType::SourceAndTest,
        ),
        ("slo_syntax", "Syntax", SYNTAX_SLO.as_str(), SLOFileType::SourceAndTest),
        ("slo_testing", "Testing", TESTING_SLO.as_str(), SLOFileType::Test),
    ];

    let mut slo_requests = Vec::new();

    for (slo_key, slo_name, slo_system_message, slo_file_type) in slos {
        if !enabled_slos.contains(slo_key) {
            continue;
        }

        let relevant_files: Vec<File> = match slo_file_type {
            SLOFileType::Source => source_files
                .iter()
                .filter_map(|x| project.identify(x).ok())
                .collect(),
            SLOFileType::Test => test_files
                .iter()
                .filter_map(|x| project.identify(x).ok())
                .collect(),
            SLOFileType::SourceAndTest => source_files
                .iter()
                .chain(test_files.iter())
                .filter_map(|x| project.identify(x).ok())
                .collect(),
        };

        let relevant_file_codes: Vec<String> =
            relevant_files.iter().map(|x| x.parser().code()).collect();

        ensure!(
            !relevant_file_codes.is_empty(),
            "No relevant files ({:?}) with source code found for SLO {}",
            slo_file_type,
            slo_name
        );

        let mut student_message = vec![format!(
            "# Submission for {project_title}\n\nDescription: {project_description}"
        )];

        for (file, code) in relevant_files.iter().zip(relevant_file_codes.iter()) {
            student_message.push(format!(
                "\n\n## Contents of {file_name}\n\n```java\n{code}\n```",
                file_name = file.proper_name(),
                code = code
            ));
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

            let temp_opt = std::env::var("OPENAI_TEMPERATURE")
                .ok()
                .and_then(|s| s.parse::<f32>().ok());
            let top_p_opt = std::env::var("OPENAI_TOP_P")
                .ok()
                .and_then(|s| s.parse::<f32>().ok());
            let effort = parse_reasoning_effort(std::env::var("OPENAI_REASONING_EFFORT").ok());

            let response = openai_client
                .chat()
                .create(CreateChatCompletionRequest {
                    model: std::env::var("OPENAI_MODEL")
                        .expect("OPENAI_MODEL must be set for SLO feedback"),
                    messages: messages.clone(),
                    temperature: temp_opt,
                    top_p: top_p_opt,
                    n: Some(1),
                    stream: Some(false),
                    reasoning_effort: Some(effort),
                    ..Default::default()
                })
                .await;

            (slo_name, response)
        });
    }

    let slo_responses = futures::future::join_all(slo_requests).await;
    Ok(slo_responses)
}

/// Convert an optional environment string into a `ChatReasoningEffort`, falling
/// back to `Medium`.
fn parse_reasoning_effort(val: Option<String>) -> ChatReasoningEffort {
    match val
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
        .unwrap_or("medium")
    {
        "low" => ChatReasoningEffort::Low,
        "high" => ChatReasoningEffort::High,
        _ => ChatReasoningEffort::Medium,
    }
}

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
pub fn show_result(results: Array, gradescope_config: rhai::Map) -> Result<()> {
    let results: Vec<GradeResult> = results
        .iter()
        .map(|f| f.clone().cast::<GradeResult>())
        .collect();
    let source_files = gradescope_config
        .get("source_files")
        .unwrap_or(&Dynamic::from(Array::new()))
        .clone()
        .cast::<Array>()
        .iter()
        .map(|f| f.clone().cast::<String>())
        .collect::<Vec<String>>();

    let test_files = gradescope_config
        .get("test_files")
        .unwrap_or(&Dynamic::from(Array::new()))
        .clone()
        .cast::<Array>()
        .iter()
        .map(|f| f.clone().cast::<String>())
        .collect::<Vec<String>>();

    let project_title = gradescope_config
        .get("project_title")
        .unwrap_or(&Dynamic::from(String::new()))
        .clone()
        .cast::<String>();
    let project_description = gradescope_config
        .get("project_description")
        .unwrap_or(&Dynamic::from(String::new()))
        .clone()
        .cast::<String>();
    let pass_threshold = gradescope_config
        .get("pass_threshold")
        .unwrap_or(&Dynamic::from(0.7))
        .clone()
        .cast::<f64>();

    let get_or_default = |f: &str, d: bool| -> bool {
        gradescope_config
            .get(f)
            .unwrap_or(&Dynamic::from(d))
            .clone()
            .cast::<bool>()
    };
    let show_table = get_or_default("show_table", true);
    let gradescope_json = get_or_default("results_json", false);
    let gradescope_feedback = get_or_default("feedback", false);
    let gradescope_debug = get_or_default("debug", false);

    let enabled_slos: HashSet<String> = vec![
        "slo_algorithmic_solutions",
        "slo_code_readability",
        "slo_comments",
        "slo_error_handling",
        "slo_logic",
        "slo_naming_conventions",
        "slo_oop_programming",
        "slo_syntax",
        "slo_testing",
    ]
    .into_iter()
    .filter(|&slo| get_or_default(slo, false))
    .map(String::from)
    .collect();

    let (grade, out_of) = results
        .iter()
        .fold((0f64, 0f64), |acc, r| (acc.0 + r.grade.grade, acc.1 + r.grade.out_of));

    if show_table {
        eprintln!(
            "{}",
            Table::new(&results)
                .with(Panel::header("Grading Overview"))
                .with(Panel::footer(format!("Total: {grade:.2}/{out_of:.2}")))
                .with(Modify::new(Rows::new(1..)).with(Width::wrap(24).keep_words(true)))
                .with(
                    Modify::new(Rows::first())
                        .with(Alignment::center())
                        .with(Alignment::center_vertical()),
                )
                .with(
                    Modify::new(Rows::last())
                        .with(Alignment::center())
                        .with(Alignment::center_vertical()),
                )
                .with(Style::modern())
        );
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

            let test_case = GradescopeTestCase::builder()
                .name(result.requirement())
                .name_format(GradescopeOutputFormat::Text)
                .max_score(result.out_of())
                .score(result.grade())
                .status(if result.grade() > pass_threshold * result.out_of() {
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

            ensure!(
                !project_title.is_empty(),
                "Project title must be specified to generate SLO feedback"
            );
            ensure!(
                !project_description.is_empty(),
                "Project description must be specified to generate SLO feedback"
            );

            let slo_responses = runtime.block_on(async {
                generate_slo_responses(
                    &project,
                    &source_files,
                    &test_files,
                    &project_title,
                    &project_description,
                    &enabled_slos,
                )
                .await
            })?;

            let combined_report =
                runtime.block_on(async { generate_combined_slo_report(slo_responses).await })?;

            test_cases.push(
                GradescopeTestCase::builder()
                    .name("Student Learning Outcomes (SLOs) Feedback".to_string())
                    .name_format(GradescopeOutputFormat::Text)
                    .output(combined_report)
                    .output_format(GradescopeOutputFormat::Md)
                    .max_score(0f64)
                    .score(0f64)
                    .build(),
            );
        }
        let submission = GradescopeSubmission::builder()
            .tests(Some(test_cases))
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
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// gets the `out_of` field
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// sets the `out_of` field
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// gets the `expected` field
    pub fn expected(&mut self) -> Array {
        self.expected.clone()
    }

    /// sets the `expected` field
    pub fn set_expected(mut self, expected: Array) -> Self {
        self.expected = expected;
        self
    }

    /// gets the `actual` field
    pub fn input(&mut self) -> Array {
        self.input.clone()
    }

    /// sets the `actual` field
    pub fn set_input(mut self, input: Array) -> Self {
        self.input = input;
        self
    }

    /// gets the `project` field
    pub fn project(&mut self) -> Project {
        self.project.clone()
    }

    /// sets the `project` field
    pub fn set_project(mut self, project: Project) -> Self {
        self.project = project;
        self
    }

    /// gets the `file` field
    pub fn file(&mut self) -> String {
        self.file.clone()
    }

    /// sets the `file` field
    pub fn set_file(mut self, file: String) -> Self {
        self.file = file;
        self
    }

    /// gets the `ignore_case` field
    pub fn ignore_case(&mut self) -> bool {
        self.ignore_case
    }

    /// sets the `ignore_case` field
    pub fn set_ignore_case(mut self, ignore_case: bool) -> Self {
        self.ignore_case = ignore_case;
        self
    }

    /// Grades by diffing the `expected` and `actual` strings.
    pub fn grade_by_diff(&mut self) -> Result<GradeResult> {
        ensure!(
            !self.expected.is_empty() & !self.input.is_empty(),
            "At least one test case (input-expected pair) must be provided"
        );
        ensure!(
            self.expected.len() == self.input.len(),
            "expected and input case arrays must be of the same length"
        );

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
                        return Ok(GradeResult {
                            requirement: self.req_name.clone(),
                            grade:       Grade::new(0.0, self.out_of),
                            reason:      "Error running file for some cases.".to_string(),
                            prompt:      Some(messages),
                        });
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
                        return Ok(GradeResult {
                            requirement: self.req_name.clone(),
                            grade:       Grade::new(0.0, self.out_of),
                            reason:      "Unknown error while running file for some cases."
                                .to_string(),
                            prompt:      Some(messages),
                        });
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
                let prompt = format!(
                    "Comparing expected and actual output for \
                     {}:\n```{inp}Expected:\n{}\nActual:\n{}\n```\n",
                    file.file_name(),
                    expected,
                    actual,
                    inp = if self.input.is_empty() {
                        String::new()
                    } else {
                        format!("\nInput:\n`{}`\n", input)
                    },
                );

                eprintln!("{prompt}");
                prompts.push(prompt);
            }
        }

        if prompts.is_empty() {
            Ok(GradeResult {
                requirement: self.req_name.clone(),
                grade:       Grade {
                    grade:  self.out_of,
                    out_of: self.out_of,
                },
                reason:      "Got expected output".to_string(),
                prompt:      None,
            })
        } else {
            let context = format!(
                "{prompt}\n\nSource code:\n```java\n{code}\n```\nMy tests are failing due to the \
                 above.",
                prompt = prompts.join("\n\n"),
                code = file.parser().code()
            );

            Ok(GradeResult {
                requirement: self.req_name.clone(),
                grade:       Grade {
                    grade:  0.0,
                    out_of: self.out_of,
                },
                reason:      "See above.".to_string(),
                prompt:      Some(vec![
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(SYSTEM_MESSAGE.to_string())
                        .name("Instructor".to_string())
                        .build()
                        .context("Failed to build system message")?
                        .into(),
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(context)
                        .name("Student".to_string())
                        .build()
                        .context("Failed to build system message")?
                        .into(),
                ]),
            })
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

/// Generates feedback for a single `GradeResult` and posts it to the database.
fn generate_single_feedback(result: &GradeResult) -> Result<String> {
    let rt = RUNTIME.handle().clone();

    if result.grade.grade < result.grade.out_of {
        let id = uuid::Uuid::new_v4().to_string();
        let mut result = result.clone();
        let body = PromptRow {
            id:               id.clone(),
            messages:         result.prompt(),
            requirement_name: result.requirement(),
            reason:           result.reason(),
            grade:            result.grade.to_string(),
            status:           "not_started".into(),
        };

        let messages = serde_json::to_string(&body)?;

        // Post to the database
        rt.block_on(async {
            POSTGREST_CLIENT
                .from("prompts")
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
        Ok(String::from(
            "This type of feedback cannot be generated for submissions without penalty.",
        ))
    }
}

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
    pub fn set_query(mut self, query: String) -> Self {
        self.query = query;
        self
    }

    /// Gets the captures to extract from the query.
    pub fn capture(&self) -> String {
        self.capture.clone()
    }

    /// Sets the captures to extract from the query.
    pub fn set_capture(mut self, capture: String) -> Self {
        self.capture = capture;
        self
    }

    /// Gets the function to filter the results of the query.
    pub fn filter(&self) -> Option<FnPtr> {
        self.filter.clone()
    }

    /// Set the function to filter the results of the query.
    pub fn set_filter(mut self, filter: FnPtr) -> Self {
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
    #[error(
        "This query could not be run, likely due to a syntax \
         error.\nQuery:\n```\n{q}\n```\nError:\n```\n{e}\n```"
    )]
    DuringQueryExecution {
        /// The query that could not be run.
        q: String,
        /// The error that occurred.
        e: String,
    },
    /// No matches found for a previously selected capture, all subsequent
    /// queries will return nothing.
    #[error(
        "No matches found for a previously selected capture: `{0}`, all subsequent queries will \
         return nothing."
    )]
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
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// Gets the "out of" grade for the requirement.
    pub fn out_of(&self) -> f64 {
        self.out_of
    }

    /// Sets the "out of" grade for the requirement.
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// Gets the file to run the query on.
    pub fn file(&self) -> &str {
        &self.file
    }

    /// Sets the file to run the query on.
    pub fn set_file(mut self, file: String) -> Self {
        self.file = file;
        self
    }

    /// Gets the project to run the query on.
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Sets the project to run the query on.
    pub fn set_project(mut self, project: Project) -> Self {
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
    pub fn must_match_exactly_n_times(mut self, n: usize) -> Self {
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
    pub fn set_reason(mut self, reason: String) -> Self {
        self.reason = reason;
        self
    }

    /// Adds a query to run.
    /// If no file has been selected, this will throw an error.
    pub fn query(#[allow(unused_mut)] mut self, q: String) -> Result<Self, QueryError> {
        if self.file.is_empty() {
            return Err(QueryError::NoFileSelected);
        }

        self.queries.push(Query {
            query:   q,
            capture: String::new(),
            filter:  None,
        });

        Ok(self)
    }

    /// Adds a capture to the last query.
    /// If no queries have been added, this will throw an error.
    pub fn capture(#[allow(unused_mut)] mut self, c: String) -> Result<Self, QueryError> {
        if let Some(last) = self.queries.last_mut() {
            *last = last.clone().set_capture(c);
            Ok(self)
        } else {
            Err(QueryError::NoPreviousQuery)
        }
    }

    /// Adds a capture to the last query.
    /// If no queries have been added, this will throw an error.
    pub fn filter(#[allow(unused_mut)] mut self, f: FnPtr) -> Result<Self, QueryError> {
        if let Some(last) = self.queries.last_mut() {
            *last = last.clone().set_filter(f);
            Ok(self)
        } else {
            Err(QueryError::NoPreviousQuery)
        }
    }

    /// Selects entire method body and returns
    pub fn method_body_with_name(mut self, method_name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("queries/method_body_with_name.scm"), method_name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects entire method body and returns
    pub fn method_body_with_return_type(mut self, return_type: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("queries/method_body_with_return_type.scm"), return_type),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects and returns the entire main method
    pub fn main_method(mut self) -> Self {
        self.queries.push(Query {
            query:   include_str!("queries/main_method.scm").to_string(),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects entire class body with name
    pub fn class_body_with_name(mut self, class_name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("queries/class_with_name.scm"), class_name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects local variable declaration statements
    pub fn local_variables(mut self) -> Self {
        self.queries.push(Query {
            query:   String::from("((local_variable_declaration) @var)"),
            capture: "var".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects local variable declaration statements with supplied name
    pub fn local_variables_with_name(mut self, name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("queries/local_variable_with_name.scm"), name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects local variable declaration statements with supplied type
    pub fn local_variables_with_type(mut self, type_name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("queries/local_variable_with_type.scm"), type_name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects if statements (entire, including else if and else)
    pub fn if_statements(mut self) -> Self {
        self.queries.push(Query {
            query:   String::from("((if_statement) @if)"),
            capture: "if".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects for loops
    pub fn for_loops(mut self) -> Self {
        self.queries.push(Query {
            query:   String::from("((for_statement) @for)"),
            capture: "for".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects while loops
    pub fn while_loops(mut self) -> Self {
        self.queries.push(Query {
            query:   String::from("((while_statement) @while)"),
            capture: "while".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects method invocations
    pub fn method_invocations(mut self) -> Self {
        self.queries.push(Query {
            query:   include_str!("queries/method_invocation.scm").to_string(),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects method invocations with supplied name
    pub fn method_invocations_with_name(mut self, name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("queries/method_invocations_with_name.scm"), name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects method invocations with supplied arguments
    pub fn method_invocations_with_arguments(mut self, name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("queries/method_invocations_with_arguments.scm"), name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects method invocations with supplied object
    pub fn method_invocations_with_object(mut self, name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("queries/method_invocations_with_object.scm"), name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Runs the queries, and returns the result.
    /// TODO: Make it so that it doesn't parse a new piece of code, just filters
    /// out the irrelevant line ranges. This performs better but more
    /// importantly is more accurate.
    pub fn run_query(&self) -> Result<Dynamic, QueryError> {
        let engine = create_engine();
        let ast = std::sync::Arc::clone(&SCRIPT_AST);
        let ast = ast.lock().unwrap();

        let first = self
            .queries
            .first()
            .ok_or_else(|| QueryError::NoMatchesFound("No queries to run".to_string()))?;

        let file = self
            .project
            .identify(self.file())
            .map_err(|_| QueryError::FileNotFound(self.file().to_string()))?;

        let mut matches: Vec<String> = match file.query(&first.query()) {
            Ok(m) => {
                if first.capture().is_empty() {
                    return Err(QueryError::NoCaptureSelected(format!("{:#?}", first)));
                }
                let result = m
                    .iter()
                    .filter_map(|map| map.get(&first.capture()))
                    .cloned();

                let result: Vec<String> = if let Some(f) = first.filter() {
                    result
                        .filter(|x| f.call(&engine, &ast, (x.clone(),)).unwrap_or(false))
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
                return Err(QueryError::DuringQueryExecution {
                    q: first.query(),
                    e: format!("{:#?}", e),
                });
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
                let parser = Parser::new(code)
                    .context(format!("Failed to create parser for query: `{}`", q.query()))?;

                match parser.query(&q.query()) {
                    Ok(m) => {
                        let result = m.iter().filter_map(|map| map.get(&q.capture())).cloned();

                        let mut result: Vec<String> = if let Some(f) = q.filter() {
                            result
                                .filter(|x| f.call(&engine, &ast, (x.clone(),)).unwrap_or(false))
                                .collect()
                        } else {
                            result.collect()
                        };

                        new_matches.append(&mut result)
                    }
                    Err(e) => {
                        return Err(QueryError::DuringQueryExecution {
                            q: q.query(),
                            e: format!("{:#?}", e),
                        });
                    }
                };
            }

            matches = new_matches;
        }

        Ok(matches.into())
    }

    /// Grades the file according to the supplied queries, captures, and
    /// constraints.
    pub fn grade_by_query(self) -> Result<GradeResult> {
        let reason = if self.reason.trim().is_empty() {
            eprintln!(
                "Warning: No reason provided for query grading. Feedback to student will not be \
                 very helpful."
            );
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
                return Ok(GradeResult {
                    requirement: self.req_name.clone(),
                    grade: Grade {
                        grade:  0.0,
                        out_of: self.out_of,
                    },
                    reason,
                    prompt: Some(vec![
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content(SYSTEM_MESSAGE.to_string())
                            .name("Instructor".to_string())
                            .build()
                            .context("Failed to build system message")?
                            .into(),
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content(format!(
                                "Something went wrong when using treesitter queries to grade \
                                 `{}`. Error message:\n\n```\n{}\n```\n",
                                self.file, e
                            ))
                            .name("Instructor".to_string())
                            .build()
                            .context("Failed to build system message")?
                            .into(),
                    ]),
                });
            }
        };

        match self.constraint {
            QueryConstraint::MustMatchAtLeastOnce => {
                if result.is_empty() {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  0.0,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: Some(vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(SYSTEM_MESSAGE.to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(format!("For file `{}`: {}.", self.file, self.reason))
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                        ]),
                    })
                } else {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  self.out_of,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: None,
                    })
                }
            }
            QueryConstraint::MustMatchExactlyNTimes(n) => {
                if result.len() == n {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  self.out_of,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: None,
                    })
                } else {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  0.0,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: Some(vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(SYSTEM_MESSAGE.to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(format!("For file `{}`: {}", self.file, self.reason))
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                        ]),
                    })
                }
            }
            QueryConstraint::MustNotMatch => {
                if result.is_empty() {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  self.out_of,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: None,
                    })
                } else {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  0.0,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: Some(vec![
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(SYSTEM_MESSAGE.to_string())
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(format!("For file `{}`: {}", self.file, self.reason))
                                .name("Instructor".to_string())
                                .build()
                                .context("Failed to build system message")?
                                .into(),
                        ]),
                    })
                }
            }
        }
    }
}
