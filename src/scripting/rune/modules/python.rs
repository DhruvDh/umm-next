//! Rune bindings for Python graders.
//!
//! This module exposes the Python grading infrastructure to Rune scripts,
//! allowing for scriptable autograding of Python assignments.

use std::path::PathBuf;

use rune::{
    Any, ContextError, Module, Ref,
    support::{Error as RuneError, Result as RuneResult},
};
use serde_json;

use crate::{
    python::grade::{self, GradeResult as InnerGradeResult},
    scripting::rune::modules::gradescope::GradescopeConfig as RuneGradescopeConfig,
};

/// Free constructor: discover the current Python project.
#[rune::function(path = new_project)]
pub fn new_project() -> RuneResult<Project> {
    Ok(Project {
        inner: crate::python::Project::new().map_err(host_err)?,
    })
}

/// Free constructor: build a project from explicit paths.
#[rune::function(path = new_project_from_paths)]
pub fn new_project_from_paths(paths: ProjectPaths) -> RuneResult<Project> {
    Ok(Project {
        inner: crate::python::Project::from_paths(paths.inner).map_err(host_err)?,
    })
}

/// Free constructor: build a project from explicit paths and a run context.
#[rune::function(path = new_project_from_paths_with_context)]
pub fn new_project_from_paths_with_context(
    paths: ProjectPaths,
    ctx: RunContext,
) -> RuneResult<Project> {
    Ok(Project {
        inner: crate::python::Project::from_paths_with_context(paths.inner, ctx.inner)
            .map_err(host_err)?,
    })
}

/// Free constructor: start a paths builder for fine-grained overrides.
#[rune::function(path = new_project_paths)]
pub fn new_project_paths() -> ProjectPathsBuilder {
    ProjectPathsBuilder {
        root_dir:   None,
        source_dir: None,
        test_dir:   None,
        venv_dir:   None,
        data_dir:   None,
        report_dir: None,
        umm_dir:    None,
    }
}

/// Free constructor: start a run-context builder.
#[rune::function(path = new_run_context)]
pub fn new_run_context() -> RunContextBuilder {
    RunContextBuilder {
        root_dir:    None,
        working_dir: None,
        env_path:    None,
        overlays:    Vec::new(),
        locked:      false,
        no_project:  false,
        no_sync:     false,
        frozen:      false,
        no_config:   true,
        no_env_file: true,
        pythonpath:  None,
    }
}

/// Free constructor: start building a diff grader.
#[rune::function(path = new_diff_grader)]
pub fn new_diff_grader() -> DiffGraderBuilder {
    DiffGraderBuilder {
        req_name:            None,
        out_of:              None,
        project:             None,
        file:                None,
        cases:               Vec::new(),
        ignore_case:         false,
        preserve_whitespace: false,
    }
}

/// Free constructor: start building a query grader.
#[rune::function(path = new_query_grader)]
pub fn new_query_grader() -> QueryGraderBuilder {
    QueryGraderBuilder {
        req_name:   None,
        out_of:     None,
        project:    None,
        file:       None,
        queries:    Vec::new(),
        constraint: None,
        reason:     None,
    }
}

/// Free constructor: start building a docs grader.
#[rune::function(path = new_docs_grader)]
pub fn new_docs_grader() -> DocsGraderBuilder {
    DocsGraderBuilder {
        project:  None,
        files:    Vec::new(),
        req_name: None,
        out_of:   None,
        penalty:  None,
    }
}

/// Free constructor: start building a test grader.
#[rune::function(path = new_test_grader)]
pub fn new_test_grader() -> TestGraderBuilder {
    TestGraderBuilder {
        project:    None,
        test_files: Vec::new(),
        req_name:   None,
        out_of:     None,
    }
}

/// Free constructor: start building a code review grader.
#[rune::function(path = new_code_review_grader)]
pub fn new_code_review_grader() -> CodeReviewGraderBuilder {
    CodeReviewGraderBuilder {
        project:             None,
        files:               Vec::new(),
        instructions_path:   None,
        weekly_context_path: None,
        req_name:            None,
        out_of:              None,
        execute_files:       true,
    }
}

/// Map host errors into Rune errors with readable messages.
fn host_err<E: std::fmt::Display>(e: E) -> RuneError {
    RuneError::msg(e.to_string())
}

/// Helper to extract required builder fields without panicking.
fn take_required<T>(opt: Option<T>, field: &str) -> RuneResult<T> {
    opt.ok_or_else(|| host_err(format!("Missing required field: {field}")))
}

/// Rune-exposed wrapper around the Python `Project` discovery.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct Project {
    /// Underlying Rust project instance.
    inner: crate::python::Project,
}

impl Project {
    /// Replace the run context on this project.
    pub fn with_run_context(mut self, ctx: RunContext) -> Self {
        self.inner = self.inner.clone().with_run_context(ctx.inner);
        self
    }
}

/// Workspace path set bridged into Rune.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct ProjectPaths {
    /// Underlying Rust `ProjectPaths`.
    inner: crate::python::ProjectPaths,
}

/// Execution context used to run Python tools/scripts.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct RunContext {
    /// Underlying Rust `UvRunContext`.
    inner: crate::python::util::UvRunContext,
}

/// Builder for `RunContext` with optional overrides.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct RunContextBuilder {
    /// Project root directory (defaults to .).
    root_dir:    Option<PathBuf>,
    /// Working directory (defaults to root).
    working_dir: Option<PathBuf>,
    /// Environment path (defaults to .umm/venv under root).
    env_path:    Option<PathBuf>,
    /// Overlay deps for a single run.
    overlays:    Vec<String>,
    /// Whether to pass --locked.
    locked:      bool,
    /// Whether to disable project installation/detection.
    no_project:  bool,
    /// Whether to skip env sync.
    no_sync:     bool,
    /// Whether to forbid resolution (lockfile only).
    frozen:      bool,
    /// Disable uv config discovery.
    no_config:   bool,
    /// Disable .env loading.
    no_env_file: bool,
    /// Explicit PYTHONPATH entries.
    pythonpath:  Option<Vec<String>>,
}

/// Builder for `ProjectPaths` with optional overrides.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct ProjectPathsBuilder {
    /// Project root directory.
    root_dir:   Option<PathBuf>,
    /// Source directory (defaults to root).
    source_dir: Option<PathBuf>,
    /// Test sources directory (defaults to `root/tests`).
    test_dir:   Option<PathBuf>,
    /// Virtual environment directory (defaults to `root/.venv`).
    venv_dir:   Option<PathBuf>,
    /// Data files directory (defaults to root).
    data_dir:   Option<PathBuf>,
    /// Report directory (defaults to `root/.umm/reports`).
    report_dir: Option<PathBuf>,
    /// UMM metadata directory (defaults to `root/.umm`).
    umm_dir:    Option<PathBuf>,
}

impl ProjectPathsBuilder {
    /// Override project root.
    pub fn root_dir(mut self, path: String) -> Self {
        self.root_dir = Some(PathBuf::from(path));
        self
    }
    /// Override source directory.
    pub fn source_dir(mut self, path: String) -> Self {
        self.source_dir = Some(PathBuf::from(path));
        self
    }
    /// Override test directory.
    pub fn test_dir(mut self, path: String) -> Self {
        self.test_dir = Some(PathBuf::from(path));
        self
    }
    /// Override virtual environment directory.
    pub fn venv_dir(mut self, path: String) -> Self {
        self.venv_dir = Some(PathBuf::from(path));
        self
    }
    /// Override data directory.
    pub fn data_dir(mut self, path: String) -> Self {
        self.data_dir = Some(PathBuf::from(path));
        self
    }
    /// Override report directory.
    pub fn report_dir(mut self, path: String) -> Self {
        self.report_dir = Some(PathBuf::from(path));
        self
    }
    /// Override UMM metadata directory.
    pub fn umm_dir(mut self, path: String) -> Self {
        self.umm_dir = Some(PathBuf::from(path));
        self
    }

    /// Build a concrete `ProjectPaths`.
    pub fn build(self) -> RuneResult<ProjectPaths> {
        let root = self.root_dir.unwrap_or_else(|| PathBuf::from("."));

        let paths = crate::python::paths::ProjectPaths::from_parts(
            root,
            self.source_dir,
            self.test_dir,
            self.venv_dir,
            self.data_dir,
            self.report_dir,
            self.umm_dir,
        );

        Ok(ProjectPaths { inner: paths })
    }
}

impl RunContextBuilder {
    /// Override project root used for uv project scoping.
    pub fn root_dir(mut self, path: String) -> Self {
        self.root_dir = Some(PathBuf::from(path));
        self
    }
    /// Override working directory.
    pub fn working_dir(mut self, path: String) -> Self {
        self.working_dir = Some(PathBuf::from(path));
        self
    }
    /// Override environment path (virtualenv location).
    pub fn env_path(mut self, path: String) -> Self {
        self.env_path = Some(PathBuf::from(path));
        self
    }
    /// Add an overlay dependency (e.g., pytest).
    pub fn overlay(mut self, dep: String) -> Self {
        self.overlays.push(dep);
        self
    }
    /// Replace overlays wholesale.
    pub fn overlays(mut self, deps: Vec<String>) -> Self {
        self.overlays = deps;
        self
    }
    /// Require uv to run in locked mode.
    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }
    /// Disable project installation/detection.
    pub fn no_project(mut self, no_project: bool) -> Self {
        self.no_project = no_project;
        self
    }
    /// Skip syncing the environment.
    pub fn no_sync(mut self, no_sync: bool) -> Self {
        self.no_sync = no_sync;
        self
    }
    /// Use frozen lockfile mode (no resolution).
    pub fn frozen(mut self, frozen: bool) -> Self {
        self.frozen = frozen;
        self
    }
    /// Disable uv config discovery.
    pub fn no_config(mut self, no: bool) -> Self {
        self.no_config = no;
        self
    }
    /// Disable .env loading.
    pub fn no_env_file(mut self, no: bool) -> Self {
        self.no_env_file = no;
        self
    }
    /// Set PYTHONPATH entries explicitly.
    pub fn pythonpath(mut self, entries: Vec<String>) -> Self {
        self.pythonpath = Some(entries);
        self
    }

    /// Build a concrete `RunContext` using defaults for unspecified fields.
    pub fn build(self) -> RuneResult<RunContext> {
        let root = self.root_dir.unwrap_or_else(|| PathBuf::from("."));
        let paths = crate::python::paths::ProjectPaths::from_parts(
            root.clone(),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let mut ctx = crate::python::util::UvRunContext::for_paths(&paths);

        if let Some(dir) = self.working_dir {
            ctx = ctx.working_dir(dir);
        }
        if let Some(env) = self.env_path {
            ctx = ctx.env_path(env);
        }
        if self.locked {
            ctx = ctx.locked(true);
        }
        if self.no_project {
            ctx = ctx.no_project(true);
        }
        if self.no_sync {
            ctx = ctx.no_sync(true);
        }
        if self.frozen {
            ctx = ctx.frozen(true);
        }
        ctx = ctx.no_config(self.no_config);
        ctx = ctx.no_env_file(self.no_env_file);
        if !self.overlays.is_empty() {
            ctx = ctx.with_overlays(self.overlays);
        }
        if let Some(py_paths) = self.pythonpath {
            let sep = paths.separator();
            let mut pythonpath = std::ffi::OsString::new();
            for (idx, entry) in py_paths.iter().enumerate() {
                if idx > 0 {
                    pythonpath.push(sep);
                }
                pythonpath.push(entry);
            }
            ctx = ctx.with_pythonpath(pythonpath);
        }

        Ok(RunContext { inner: ctx })
    }
}

/// Grade result passed between Rune and Rust.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct GradeResult {
    /// Wrapped Rust `GradeResult`.
    inner: InnerGradeResult,
}

impl From<InnerGradeResult> for GradeResult {
    fn from(inner: InnerGradeResult) -> Self {
        Self { inner }
    }
}

impl GradeResult {
    /// Name of the graded requirement.
    pub fn requirement(&self) -> String {
        self.inner.requirement.clone()
    }

    /// Score achieved for this requirement.
    pub fn score(&self) -> f64 {
        self.inner.grade_value()
    }

    /// Maximum score for this requirement.
    pub fn out_of(&self) -> f64 {
        self.inner.out_of_value()
    }

    /// Serialized prompt messages, if present.
    pub fn prompt(&self) -> Option<String> {
        self.inner
            .prompt
            .as_ref()
            .and_then(|msgs| serde_json::to_string_pretty(msgs).ok())
    }

    /// Consume the wrapper and return the underlying Rust result.
    pub(crate) fn into_inner(self) -> InnerGradeResult {
        self.inner
    }
}

/// Diff case wrapper allowing construction from Rune scripts.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct DiffCase {
    /// Underlying diff case.
    inner: grade::DiffCase,
}

impl DiffCase {
    /// Create a new diff case with expected output and optional input.
    pub fn new(expected: String, input: Option<String>) -> Self {
        let mut case = grade::DiffCase::new(expected);
        if let Some(inp) = input {
            case = case.with_input(inp);
        }
        Self { inner: case }
    }

    /// Consume the wrapper and return the underlying Rust diff case.
    pub fn into_inner(self) -> grade::DiffCase {
        self.inner
    }
}

/// Namespace for diff-based grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct DiffGrader;

/// Builder for diff-based grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct DiffGraderBuilder {
    /// Requirement name.
    req_name:            Option<String>,
    /// Maximum score.
    out_of:              Option<f64>,
    /// Project to grade.
    project:             Option<Project>,
    /// File to execute.
    file:                Option<String>,
    /// Expected/actual cases.
    cases:               Vec<(String, Option<String>)>,
    /// Whether to ignore case.
    ignore_case:         bool,
    /// Whether to preserve whitespace.
    preserve_whitespace: bool,
}

impl DiffGrader {}

impl DiffGraderBuilder {
    /// Set requirement name.
    pub fn req_name(mut self, name: String) -> Self {
        self.req_name = Some(name);
        self
    }
    /// Set maximum score.
    pub fn out_of(mut self, out_of: f64) -> Self {
        self.out_of = Some(out_of);
        self
    }
    /// Attach project.
    pub fn project(mut self, project: Ref<Project>) -> Self {
        self.project = Some(project.clone());
        self
    }
    /// Set file to run against.
    pub fn file(mut self, file: String) -> Self {
        self.file = Some(file);
        self
    }
    /// Provide expected/actual cases.
    pub fn cases(mut self, cases: Vec<(String, Option<String>)>) -> Self {
        self.cases = cases;
        self
    }
    /// Toggle case-insensitive comparison.
    pub fn ignore_case(mut self, ignore: bool) -> Self {
        self.ignore_case = ignore;
        self
    }
    /// Preserve whitespace differences.
    pub fn preserve_whitespace(mut self, preserve: bool) -> Self {
        self.preserve_whitespace = preserve;
        self
    }

    /// Add a single expected output case (no input).
    /// This is a clearer alternative to `.cases([(..., None)])`.
    pub fn expect(mut self, expected: String) -> Self {
        self.cases.push((expected, None));
        self
    }

    /// Add an expected output case with stdin input.
    /// This is a clearer alternative to `.cases([(..., Some(...))])`.
    pub fn expect_with_input(mut self, expected: String, input: String) -> Self {
        self.cases.push((expected, Some(input)));
        self
    }

    /// Run the diff grader.
    pub async fn run(self) -> RuneResult<GradeResult> {
        let project = take_required(self.project, "project")?.inner;
        let file = take_required(self.file, "file")?;
        let req_name = take_required(self.req_name, "req_name")?;
        let out_of = take_required(self.out_of, "out_of")?;

        // Convert cases to DiffCase
        let cases: Vec<grade::DiffCase> = self
            .cases
            .into_iter()
            .map(|(expected, input)| {
                let mut case = grade::DiffCase::new(expected);
                if let Some(inp) = input {
                    case = case.with_input(inp);
                }
                case
            })
            .collect();

        let builder = grade::DiffGrader::builder()
            .project(project)
            .file(file)
            .cases(cases)
            .ignore_case(self.ignore_case)
            .preserve_whitespace(self.preserve_whitespace)
            .req_name(req_name)
            .out_of(out_of);

        builder
            .build()
            .run()
            .await
            .map(GradeResult::from)
            .map_err(host_err)
    }
}

/// Constraint applied to query results.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct QueryConstraint {
    /// Wrapped Rust constraint.
    inner: grade::QueryConstraint,
}

impl QueryConstraint {
    #[rune::function(path = QueryConstraint::must_match_at_least_once)]
    /// Require at least one match.
    pub fn must_match_at_least_once() -> Self {
        Self {
            inner: grade::QueryConstraint::MustMatchAtLeastOnce,
        }
    }

    #[rune::function(path = QueryConstraint::must_match_exactly_n)]
    /// Require exactly `n` matches.
    pub fn must_match_exactly_n(times: usize) -> Self {
        Self {
            inner: grade::QueryConstraint::MustMatchExactlyNTimes(times),
        }
    }

    #[rune::function(path = QueryConstraint::must_not_match)]
    /// Require zero matches.
    pub fn must_not_match() -> Self {
        Self {
            inner: grade::QueryConstraint::MustNotMatch,
        }
    }
}

/// Namespace for tree-sitter query graders.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct QueryGrader;

/// Builder for query graders.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct QueryGraderBuilder {
    /// Requirement name.
    req_name:   Option<String>,
    /// Maximum score.
    out_of:     Option<f64>,
    /// Project to grade.
    project:    Option<Project>,
    /// Target file name.
    file:       Option<String>,
    /// Queries to execute. Each entry may optionally include a capture name.
    queries:    Vec<(String, Option<String>)>,
    /// Optional constraint.
    constraint: Option<QueryConstraint>,
    /// Optional reason presented on failure.
    reason:     Option<String>,
}

impl QueryGrader {}

impl QueryGraderBuilder {
    /// Set requirement name.
    pub fn req_name(mut self, name: String) -> Self {
        self.req_name = Some(name);
        self
    }
    /// Set maximum score.
    pub fn out_of(mut self, out_of: f64) -> Self {
        self.out_of = Some(out_of);
        self
    }
    /// Attach project to grade.
    pub fn project(mut self, project: Ref<Project>) -> Self {
        self.project = Some(project.clone());
        self
    }
    /// Set file to run queries against.
    pub fn file(mut self, file: String) -> Self {
        self.file = Some(file);
        self
    }
    /// Provide queries (capture defaults to "body").
    pub fn queries(mut self, queries: Vec<String>) -> Self {
        self.queries = queries.into_iter().map(|q| (q, None)).collect();
        self
    }
    /// Provide queries with explicit captures.
    pub fn queries_with_capture(mut self, queries: Vec<(String, String)>) -> Self {
        self.queries = queries
            .into_iter()
            .map(|(q, capture)| (q, Some(capture)))
            .collect();
        self
    }
    /// Apply a constraint to the queries.
    pub fn constraint(mut self, constraint: QueryConstraint) -> Self {
        self.constraint = Some(constraint);
        self
    }
    /// Provide a reason used in failure messaging.
    pub fn reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }

    /// Convenience: query for a function with a specific name.
    pub fn function_with_name(mut self, name: String) -> Self {
        let query = format!(
            r#"(function_definition name: (identifier) @name body: (_) @body (#eq? @name "{}"))"#,
            name
        );
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: query for a class with a specific name.
    pub fn class_with_name(mut self, name: String) -> Self {
        let query = format!(
            r#"(class_definition name: (identifier) @name body: (_) @body (#eq? @name "{}"))"#,
            name
        );
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check for list comprehension usage.
    pub fn uses_list_comprehension(mut self) -> Self {
        let query = "(list_comprehension) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check for for loop usage.
    pub fn uses_for_loop(mut self) -> Self {
        let query = "(for_statement) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check for while loop usage.
    pub fn uses_while_loop(mut self) -> Self {
        let query = "(while_statement) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check for if statement usage.
    pub fn uses_if_statement(mut self) -> Self {
        let query = "(if_statement) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check for try/except usage.
    pub fn uses_try_except(mut self) -> Self {
        let query = "(try_statement) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check for lambda expression usage.
    pub fn uses_lambda(mut self) -> Self {
        let query = "(lambda) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check for decorator usage.
    pub fn uses_decorator(mut self) -> Self {
        let query = "(decorator) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check for context manager (with statement) usage.
    pub fn uses_with_statement(mut self) -> Self {
        let query = "(with_statement) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check for generator/yield usage.
    pub fn uses_yield(mut self) -> Self {
        let query = "(yield) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check for dictionary comprehension usage.
    pub fn uses_dict_comprehension(mut self) -> Self {
        let query = "(dictionary_comprehension) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check for set comprehension usage.
    pub fn uses_set_comprehension(mut self) -> Self {
        let query = "(set_comprehension) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check for generator expression usage.
    pub fn uses_generator_expression(mut self) -> Self {
        let query = "(generator_expression) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check for assert statement usage.
    pub fn uses_assert(mut self) -> Self {
        let query = "(assert_statement) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check for raise statement usage.
    pub fn uses_raise(mut self) -> Self {
        let query = "(raise_statement) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self
    }

    /// Convenience: check that a specific module is imported.
    pub fn imports_module(mut self, module_name: String) -> Self {
        let query = format!(
            r#"(import_statement name: (dotted_name) @name (#eq? @name "{}"))"#,
            module_name
        );
        self.queries.push((query, Some("name".to_string())));
        self
    }

    /// Convenience: check that a specific item is imported from a module.
    pub fn imports_from(mut self, module_name: String) -> Self {
        let query = format!(
            r#"(import_from_statement module_name: (dotted_name) @name (#eq? @name "{}"))"#,
            module_name
        );
        self.queries.push((query, Some("name".to_string())));
        self
    }

    /// Convenience: shorter alias for function_with_name.
    pub fn defines_function(self, name: String) -> Self {
        self.function_with_name(name)
    }

    /// Convenience: shorter alias for class_with_name.
    pub fn defines_class(self, name: String) -> Self {
        self.class_with_name(name)
    }

    /// Negated convenience: check that code does NOT use a for loop.
    pub fn must_not_use_for_loop(mut self) -> Self {
        let query = "(for_statement) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self.constraint = Some(QueryConstraint {
            inner: grade::QueryConstraint::MustNotMatch,
        });
        self
    }

    /// Negated convenience: check that code does NOT use a while loop.
    pub fn must_not_use_while_loop(mut self) -> Self {
        let query = "(while_statement) @body".to_string();
        self.queries.push((query, Some("body".to_string())));
        self.constraint = Some(QueryConstraint {
            inner: grade::QueryConstraint::MustNotMatch,
        });
        self
    }

    /// Negated convenience: check that code does NOT use recursion.
    /// Note: This is a heuristic check for function calls matching the function
    /// name.
    pub fn must_not_use_recursion(mut self, function_name: String) -> Self {
        let query = format!(r#"(call function: (identifier) @fn (#eq? @fn "{}"))"#, function_name);
        self.queries.push((query, Some("fn".to_string())));
        self.constraint = Some(QueryConstraint {
            inner: grade::QueryConstraint::MustNotMatch,
        });
        self
    }

    /// Run the query grader.
    pub async fn run(self) -> RuneResult<GradeResult> {
        let queries: Vec<grade::Query> = self
            .queries
            .into_iter()
            .map(|(q, capture)| {
                let mut query = grade::Query::new().set_query(q);
                let cap = capture.unwrap_or_else(|| "body".to_string());
                query = query.set_capture(cap);
                query
            })
            .collect();

        // Default to MustMatchAtLeastOnce if no constraint specified
        let constraint = self
            .constraint
            .map(|c| c.inner)
            .unwrap_or(grade::QueryConstraint::MustMatchAtLeastOnce);
        let builder = grade::QueryGrader::builder()
            .queries(queries)
            .constraint(constraint)
            .reason(self.reason.unwrap_or_default());
        let builder = builder
            .req_name(take_required(self.req_name, "req_name")?)
            .out_of(take_required(self.out_of, "out_of")?)
            .project(take_required(self.project, "project")?.inner)
            .file(take_required(self.file, "file")?);

        builder
            .build()
            .run()
            .await
            .map(GradeResult::from)
            .map_err(host_err)
    }
}

/// Namespace for docs grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct DocsGrader;

/// Builder for docs grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct DocsGraderBuilder {
    /// Project to grade.
    project:  Option<Project>,
    /// Source files to lint.
    files:    Vec<String>,
    /// Requirement name.
    req_name: Option<String>,
    /// Maximum score.
    out_of:   Option<f64>,
    /// Penalty per violation.
    penalty:  Option<f64>,
}

impl DocsGrader {}

impl DocsGraderBuilder {
    /// Set the project to grade.
    pub fn project(mut self, project: Ref<Project>) -> Self {
        self.project = Some(project.clone());
        self
    }

    /// Set the files to check documentation for.
    pub fn files(mut self, files: Vec<String>) -> Self {
        self.files = files;
        self
    }

    /// Set the requirement name.
    pub fn req_name(mut self, name: String) -> Self {
        self.req_name = Some(name);
        self
    }

    /// Set the points available.
    pub fn out_of(mut self, points: f64) -> Self {
        self.out_of = Some(points);
        self
    }

    /// Set the per-violation penalty.
    pub fn penalty(mut self, penalty: f64) -> Self {
        self.penalty = Some(penalty);
        self
    }

    /// Build with bon defaults and run.
    pub async fn run(self) -> RuneResult<GradeResult> {
        let builder = grade::DocsGrader::builder()
            .project(take_required(self.project, "project")?.inner)
            .files(self.files)
            .req_name(take_required(self.req_name, "req_name")?)
            .out_of(take_required(self.out_of, "out_of")?)
            .penalty(self.penalty.unwrap_or(3.0));

        builder
            .build()
            .run()
            .await
            .map(GradeResult::from)
            .map_err(host_err)
    }
}

/// Namespace for test grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct TestGrader;

/// Builder for test grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct TestGraderBuilder {
    /// Project to grade.
    project:    Option<Project>,
    /// Test files to run.
    test_files: Vec<String>,
    /// Requirement name.
    req_name:   Option<String>,
    /// Maximum score.
    out_of:     Option<f64>,
}

impl TestGrader {}

impl TestGraderBuilder {
    /// Set the project to grade.
    pub fn project(mut self, project: Ref<Project>) -> Self {
        self.project = Some(project.clone());
        self
    }

    /// Set test files to run.
    pub fn test_files(mut self, files: Vec<String>) -> Self {
        self.test_files = files;
        self
    }

    /// Set the requirement name.
    pub fn req_name(mut self, name: String) -> Self {
        self.req_name = Some(name);
        self
    }

    /// Set the points available.
    pub fn out_of(mut self, points: f64) -> Self {
        self.out_of = Some(points);
        self
    }

    /// Run the test grader.
    pub async fn run(self) -> RuneResult<GradeResult> {
        let builder = grade::TestGrader::builder()
            .project(take_required(self.project, "project")?.inner)
            .test_files(self.test_files)
            .req_name(take_required(self.req_name, "req_name")?)
            .out_of(take_required(self.out_of, "out_of")?);

        builder
            .build()
            .run()
            .await
            .map(GradeResult::from)
            .map_err(host_err)
    }
}

/// Namespace for code review grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct CodeReviewGrader;

/// Builder for code review grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::python)]
pub struct CodeReviewGraderBuilder {
    /// Project to grade.
    project:             Option<Project>,
    /// Files to grade.
    files:               Vec<String>,
    /// Path to assignment instructions.
    instructions_path:   Option<String>,
    /// Path to weekly context.
    weekly_context_path: Option<String>,
    /// Requirement name.
    req_name:            Option<String>,
    /// Maximum score.
    out_of:              Option<f64>,
    /// Whether to execute files.
    execute_files:       bool,
}

impl CodeReviewGrader {}

impl CodeReviewGraderBuilder {
    /// Set the project to grade.
    pub fn project(mut self, project: Ref<Project>) -> Self {
        self.project = Some(project.clone());
        self
    }

    /// Set files to grade.
    pub fn files(mut self, files: Vec<String>) -> Self {
        self.files = files;
        self
    }

    /// Set the path to assignment instructions.
    pub fn instructions_path(mut self, path: String) -> Self {
        self.instructions_path = Some(path);
        self
    }

    /// Set the path to weekly context.
    pub fn weekly_context_path(mut self, path: String) -> Self {
        self.weekly_context_path = Some(path);
        self
    }

    /// Set the requirement name.
    pub fn req_name(mut self, name: String) -> Self {
        self.req_name = Some(name);
        self
    }

    /// Set the points available.
    pub fn out_of(mut self, points: f64) -> Self {
        self.out_of = Some(points);
        self
    }

    /// Set whether to execute files.
    pub fn execute_files(mut self, execute: bool) -> Self {
        self.execute_files = execute;
        self
    }

    /// Run the code review grader.
    pub async fn run(self) -> RuneResult<GradeResult> {
        let builder = grade::CodeReviewGrader::builder()
            .project(take_required(self.project, "project")?.inner)
            .files(self.files)
            .req_name(take_required(self.req_name, "req_name")?)
            .out_of(take_required(self.out_of, "out_of")?)
            .execute_files(self.execute_files)
            .maybe_instructions_path(self.instructions_path)
            .maybe_weekly_context_path(self.weekly_context_path);

        builder
            .build()
            .run()
            .await
            .map(GradeResult::from)
            .map_err(host_err)
    }
}

/// Collect results into a Vec (helper for Rune scripts).
pub fn grade_all(results: Vec<GradeResult>) -> RuneResult<Vec<GradeResult>> {
    Ok(results)
}

/// Render results using default Gradescope config.
pub fn show_result(results: Vec<GradeResult>) -> RuneResult<()> {
    let config = crate::java::grade::gradescope::GradescopeConfig::default();
    let inner_results: Vec<_> = results.into_iter().map(|r| r.into_inner()).collect();
    crate::java::grade::show_result(inner_results, config).map_err(host_err)
}

/// Render results alias.
pub fn show_results(results: Vec<GradeResult>) -> RuneResult<()> {
    show_result(results)
}

/// Render results with an explicit Gradescope config.
pub fn show_result_with_config(
    results: Vec<GradeResult>,
    config: RuneGradescopeConfig,
) -> RuneResult<()> {
    let inner_results: Vec<_> = results.into_iter().map(|r| r.into_inner()).collect();
    crate::java::grade::show_result(inner_results, config.inner).map_err(host_err)
}

/// Render results with config alias.
pub fn show_results_with_config(
    results: Vec<GradeResult>,
    config: RuneGradescopeConfig,
) -> RuneResult<()> {
    show_result_with_config(results, config)
}

/// Build and return the Python Rune module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("umm", ["python"])?;

    // Register types
    module.ty::<Project>()?;
    module.ty::<ProjectPaths>()?;
    module.ty::<ProjectPathsBuilder>()?;
    module.ty::<RunContext>()?;
    module.ty::<RunContextBuilder>()?;
    module.ty::<GradeResult>()?;
    module.ty::<DiffCase>()?;
    module.ty::<DiffGrader>()?;
    module.ty::<DiffGraderBuilder>()?;
    module.ty::<QueryConstraint>()?;
    module.ty::<QueryGrader>()?;
    module.ty::<QueryGraderBuilder>()?;
    module.ty::<DocsGrader>()?;
    module.ty::<DocsGraderBuilder>()?;
    module.ty::<TestGrader>()?;
    module.ty::<TestGraderBuilder>()?;
    module.ty::<CodeReviewGrader>()?;
    module.ty::<CodeReviewGraderBuilder>()?;

    // GradeResult methods
    module.associated_function("prompt", GradeResult::prompt)?;

    // Free constructors
    module.function_meta(new_project)?;
    module.function_meta(new_project_from_paths)?;
    module.function_meta(new_project_from_paths_with_context)?;
    module.function_meta(new_project_paths)?;
    module.function_meta(new_run_context)?;
    module.function_meta(new_diff_grader)?;
    module.function_meta(new_query_grader)?;
    module.function_meta(new_docs_grader)?;
    module.function_meta(new_test_grader)?;
    module.function_meta(new_code_review_grader)?;

    // ProjectPathsBuilder methods
    module.associated_function("root_dir", ProjectPathsBuilder::root_dir)?;
    module.associated_function("source_dir", ProjectPathsBuilder::source_dir)?;
    module.associated_function("test_dir", ProjectPathsBuilder::test_dir)?;
    module.associated_function("venv_dir", ProjectPathsBuilder::venv_dir)?;
    module.associated_function("data_dir", ProjectPathsBuilder::data_dir)?;
    module.associated_function("report_dir", ProjectPathsBuilder::report_dir)?;
    module.associated_function("umm_dir", ProjectPathsBuilder::umm_dir)?;
    module.associated_function("build", ProjectPathsBuilder::build)?;

    // RunContextBuilder methods
    module.associated_function("root_dir", RunContextBuilder::root_dir)?;
    module.associated_function("working_dir", RunContextBuilder::working_dir)?;
    module.associated_function("env_path", RunContextBuilder::env_path)?;
    module.associated_function("overlay", RunContextBuilder::overlay)?;
    module.associated_function("overlays", RunContextBuilder::overlays)?;
    module.associated_function("locked", RunContextBuilder::locked)?;
    module.associated_function("no_project", RunContextBuilder::no_project)?;
    module.associated_function("no_sync", RunContextBuilder::no_sync)?;
    module.associated_function("frozen", RunContextBuilder::frozen)?;
    module.associated_function("no_config", RunContextBuilder::no_config)?;
    module.associated_function("no_env_file", RunContextBuilder::no_env_file)?;
    module.associated_function("pythonpath", RunContextBuilder::pythonpath)?;
    module.associated_function("build", RunContextBuilder::build)?;

    // Project run-context helper
    module.associated_function("with_run_context", Project::with_run_context)?;

    // DiffGraderBuilder methods
    module.associated_function("req_name", DiffGraderBuilder::req_name)?;
    module.associated_function("out_of", DiffGraderBuilder::out_of)?;
    module.associated_function("project", DiffGraderBuilder::project)?;
    module.associated_function("file", DiffGraderBuilder::file)?;
    module.associated_function("cases", DiffGraderBuilder::cases)?;
    module.associated_function("expect", DiffGraderBuilder::expect)?;
    module.associated_function("expect_with_input", DiffGraderBuilder::expect_with_input)?;
    module.associated_function("ignore_case", DiffGraderBuilder::ignore_case)?;
    module.associated_function("preserve_whitespace", DiffGraderBuilder::preserve_whitespace)?;
    module.associated_function("run", DiffGraderBuilder::run)?;

    // QueryConstraint static methods
    module.function_meta(QueryConstraint::must_match_at_least_once)?;
    module.function_meta(QueryConstraint::must_match_exactly_n)?;
    module.function_meta(QueryConstraint::must_not_match)?;

    // QueryGraderBuilder methods
    module.associated_function("req_name", QueryGraderBuilder::req_name)?;
    module.associated_function("out_of", QueryGraderBuilder::out_of)?;
    module.associated_function("project", QueryGraderBuilder::project)?;
    module.associated_function("file", QueryGraderBuilder::file)?;
    module.associated_function("queries", QueryGraderBuilder::queries)?;
    module.associated_function("queries_with_capture", QueryGraderBuilder::queries_with_capture)?;
    module.associated_function("constraint", QueryGraderBuilder::constraint)?;
    module.associated_function("reason", QueryGraderBuilder::reason)?;
    module.associated_function("function_with_name", QueryGraderBuilder::function_with_name)?;
    module.associated_function("class_with_name", QueryGraderBuilder::class_with_name)?;
    module.associated_function(
        "uses_list_comprehension",
        QueryGraderBuilder::uses_list_comprehension,
    )?;
    module.associated_function("uses_for_loop", QueryGraderBuilder::uses_for_loop)?;
    module.associated_function("uses_while_loop", QueryGraderBuilder::uses_while_loop)?;
    module.associated_function("uses_if_statement", QueryGraderBuilder::uses_if_statement)?;
    module.associated_function("uses_try_except", QueryGraderBuilder::uses_try_except)?;
    module.associated_function("uses_lambda", QueryGraderBuilder::uses_lambda)?;
    module.associated_function("uses_decorator", QueryGraderBuilder::uses_decorator)?;
    module.associated_function("uses_with_statement", QueryGraderBuilder::uses_with_statement)?;
    module.associated_function("uses_yield", QueryGraderBuilder::uses_yield)?;
    module.associated_function(
        "uses_dict_comprehension",
        QueryGraderBuilder::uses_dict_comprehension,
    )?;
    module.associated_function(
        "uses_set_comprehension",
        QueryGraderBuilder::uses_set_comprehension,
    )?;
    module.associated_function(
        "uses_generator_expression",
        QueryGraderBuilder::uses_generator_expression,
    )?;
    module.associated_function("uses_assert", QueryGraderBuilder::uses_assert)?;
    module.associated_function("uses_raise", QueryGraderBuilder::uses_raise)?;
    module.associated_function("imports_module", QueryGraderBuilder::imports_module)?;
    module.associated_function("imports_from", QueryGraderBuilder::imports_from)?;
    module.associated_function("defines_function", QueryGraderBuilder::defines_function)?;
    module.associated_function("defines_class", QueryGraderBuilder::defines_class)?;
    module
        .associated_function("must_not_use_for_loop", QueryGraderBuilder::must_not_use_for_loop)?;
    module.associated_function(
        "must_not_use_while_loop",
        QueryGraderBuilder::must_not_use_while_loop,
    )?;
    module.associated_function(
        "must_not_use_recursion",
        QueryGraderBuilder::must_not_use_recursion,
    )?;
    module.associated_function("run", QueryGraderBuilder::run)?;

    // DocsGraderBuilder methods
    module.associated_function("project", DocsGraderBuilder::project)?;
    module.associated_function("files", DocsGraderBuilder::files)?;
    module.associated_function("req_name", DocsGraderBuilder::req_name)?;
    module.associated_function("out_of", DocsGraderBuilder::out_of)?;
    module.associated_function("penalty", DocsGraderBuilder::penalty)?;
    module.associated_function("run", DocsGraderBuilder::run)?;

    // TestGraderBuilder methods
    module.associated_function("project", TestGraderBuilder::project)?;
    module.associated_function("test_files", TestGraderBuilder::test_files)?;
    module.associated_function("req_name", TestGraderBuilder::req_name)?;
    module.associated_function("out_of", TestGraderBuilder::out_of)?;
    module.associated_function("run", TestGraderBuilder::run)?;

    // CodeReviewGraderBuilder methods
    module.associated_function("project", CodeReviewGraderBuilder::project)?;
    module.associated_function("files", CodeReviewGraderBuilder::files)?;
    module.associated_function("instructions_path", CodeReviewGraderBuilder::instructions_path)?;
    module
        .associated_function("weekly_context_path", CodeReviewGraderBuilder::weekly_context_path)?;
    module.associated_function("req_name", CodeReviewGraderBuilder::req_name)?;
    module.associated_function("out_of", CodeReviewGraderBuilder::out_of)?;
    module.associated_function("execute_files", CodeReviewGraderBuilder::execute_files)?;
    module.associated_function("run", CodeReviewGraderBuilder::run)?;

    // Helper functions
    module.function("grade_all", grade_all).build()?;
    module.function("show_result", show_result).build()?;
    module.function("show_results", show_results).build()?;
    module
        .function("show_result_with_config", show_result_with_config)
        .build()?;
    module
        .function("show_results_with_config", show_results_with_config)
        .build()?;

    Ok(module)
}
