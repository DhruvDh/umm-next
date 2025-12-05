use std::path::PathBuf;

use rune::{
    Any, ContextError, Module, Ref,
    support::{Error as RuneError, Result as RuneResult},
};
use serde_json;

use crate::{
    java::grade::{self, GradeResult as InnerGradeResult},
    scripting::rune::modules::gradescope::GradescopeConfig as RuneGradescopeConfig,
};

/// Free constructor: discover the current Java project.
#[rune::function(path = new_project)]
pub fn new_project() -> RuneResult<Project> {
    Ok(Project {
        inner: crate::java::Project::new().map_err(host_err)?,
    })
}

/// Free constructor: build a project from explicit paths.
#[rune::function(path = new_project_from_paths)]
pub fn new_project_from_paths(paths: ProjectPaths) -> RuneResult<Project> {
    Ok(Project {
        inner: crate::java::Project::from_paths(paths.inner).map_err(host_err)?,
    })
}

/// Free constructor: start a paths builder for fine-grained overrides.
/// Defaults mirror `ProjectPaths::from_parts` defaults; callers set only what
/// they need (e.g., `.lib_dir("...").build()?`).
#[rune::function(path = new_project_paths)]
pub fn new_project_paths() -> ProjectPathsBuilder {
    ProjectPathsBuilder {
        root_dir:   None,
        source_dir: None,
        build_dir:  None,
        test_dir:   None,
        lib_dir:    None,
        umm_dir:    None,
        report_dir: None,
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

/// Free constructor: start building a visible unit-test grader.
#[rune::function(path = new_by_unit_test_grader)]
pub fn new_by_unit_test_grader() -> ByUnitTestGraderBuilder {
    ByUnitTestGraderBuilder {
        test_files:     Vec::new(),
        expected_tests: Vec::new(),
        project:        None,
        out_of:         None,
        req_name:       None,
    }
}

/// Free constructor: start building a mutation-testing grader.
#[rune::function(path = new_unit_test_grader)]
pub fn new_unit_test_grader() -> UnitTestGraderBuilder {
    UnitTestGraderBuilder {
        req_name:         None,
        out_of:           None,
        project:          None,
        target_test:      Vec::new(),
        target_class:     Vec::new(),
        excluded_methods: Vec::new(),
        avoid_calls_to:   Vec::new(),
    }
}

/// Free constructor: start building a hidden-test grader.
#[rune::function(path = new_by_hidden_test_grader)]
pub fn new_by_hidden_test_grader() -> ByHiddenTestGraderBuilder {
    ByHiddenTestGraderBuilder {
        url:             None,
        test_class_name: None,
        out_of:          None,
        req_name:        None,
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

// Convenience constructors live on ProjectPaths for Rune ergonomics.

/// Map host errors into Rune errors with readable messages.
fn host_err<E: std::fmt::Display>(e: E) -> RuneError {
    RuneError::msg(e.to_string())
}

/// Helper to extract required builder fields without panicking.
fn take_required<T>(opt: Option<T>, field: &str) -> RuneResult<T> {
    opt.ok_or_else(|| host_err(format!("Missing required field: {field}")))
}

/// Rune-exposed wrapper around the Java `Project` discovery.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
pub struct Project {
    /// Underlying Rust project instance.
    inner: crate::java::Project,
}

/// Workspace path set bridged into Rune.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
pub struct ProjectPaths {
    /// Underlying Rust `ProjectPaths`.
    inner: crate::java::ProjectPaths,
}

/// Builder for `ProjectPaths` with optional overrides.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
pub struct ProjectPathsBuilder {
    /// Project root directory.
    root_dir:   Option<PathBuf>,
    /// Source directory (defaults to `root/src`).
    source_dir: Option<PathBuf>,
    /// Build output directory (defaults to `root/target`).
    build_dir:  Option<PathBuf>,
    /// Test sources directory (defaults to `root/test`).
    test_dir:   Option<PathBuf>,
    /// JAR library directory (defaults to `root/lib`).
    lib_dir:    Option<PathBuf>,
    /// UMM metadata directory (defaults to `root/.umm`).
    umm_dir:    Option<PathBuf>,
    /// Report directory (defaults to `root/test_reports`).
    report_dir: Option<PathBuf>,
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
    /// Override build output directory.
    pub fn build_dir(mut self, path: String) -> Self {
        self.build_dir = Some(PathBuf::from(path));
        self
    }
    /// Override test directory.
    pub fn test_dir(mut self, path: String) -> Self {
        self.test_dir = Some(PathBuf::from(path));
        self
    }
    /// Override library directory.
    pub fn lib_dir(mut self, path: String) -> Self {
        self.lib_dir = Some(PathBuf::from(path));
        self
    }
    /// Override UMM metadata directory.
    pub fn umm_dir(mut self, path: String) -> Self {
        self.umm_dir = Some(PathBuf::from(path));
        self
    }
    /// Override report directory.
    pub fn report_dir(mut self, path: String) -> Self {
        self.report_dir = Some(PathBuf::from(path));
        self
    }

    /// Build a concrete `ProjectPaths`.
    pub fn build(self) -> RuneResult<ProjectPaths> {
        let root = self.root_dir.unwrap_or_else(|| PathBuf::from("."));

        let paths = crate::java::paths::ProjectPaths::from_parts(
            root,
            self.source_dir,
            self.build_dir,
            self.test_dir,
            self.lib_dir,
            self.umm_dir,
            self.report_dir,
        );

        Ok(ProjectPaths { inner: paths })
    }
}

/// Grade result passed between Rune and Rust.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
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
#[rune(item = ::umm::java)]
pub struct DiffCase {
    /// Underlying diff case.
    inner: grade::DiffCase,
}

impl DiffCase {
    /// Create a new diff case with expected output and optional input.
    pub fn new(expected: String, input: Option<String>) -> Self {
        Self {
            inner: grade::DiffCase { expected, input },
        }
    }

    /// Consume the wrapper and return the underlying Rust diff case.
    pub fn into_inner(self) -> grade::DiffCase {
        self.inner
    }
}

/// Docs grader namespace.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
pub struct DocsGrader;

/// State-erased docs grader builder exposed to Rune.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
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

    /// Build with bon defaults and run; bon enforces required fields.
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

/// Namespace for visible unit-test grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
pub struct ByUnitTestGrader;

/// Builder for the visible unit-test grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
pub struct ByUnitTestGraderBuilder {
    /// Test files to run.
    test_files:     Vec<String>,
    /// Expected test names.
    expected_tests: Vec<String>,
    /// Project to grade.
    project:        Option<Project>,
    /// Maximum score.
    out_of:         Option<f64>,
    /// Requirement name.
    req_name:       Option<String>,
}

impl ByUnitTestGraderBuilder {
    /// Specify test files to run.
    pub fn test_files(mut self, files: Vec<String>) -> Self {
        self.test_files = files;
        self
    }

    /// Specify expected test names.
    pub fn expected_tests(mut self, tests: Vec<String>) -> Self {
        self.expected_tests = tests;
        self
    }

    /// Attach the project.
    pub fn project(mut self, project: Ref<Project>) -> Self {
        self.project = Some(project.clone());
        self
    }

    /// Set maximum score.
    pub fn out_of(mut self, out_of: f64) -> Self {
        self.out_of = Some(out_of);
        self
    }

    /// Set requirement name.
    pub fn req_name(mut self, name: String) -> Self {
        self.req_name = Some(name);
        self
    }

    /// Run the grader.
    pub async fn run(self) -> RuneResult<GradeResult> {
        let builder = grade::ByUnitTestGrader::builder()
            .test_files(self.test_files)
            .expected_tests(self.expected_tests)
            .project(take_required(self.project, "project")?.inner)
            .out_of(take_required(self.out_of, "out_of")?)
            .req_name(take_required(self.req_name, "req_name")?);

        builder
            .build()
            .run()
            .await
            .map(GradeResult::from)
            .map_err(host_err)
    }
}

/// Namespace for mutation-testing grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
pub struct UnitTestGrader;

/// Builder for mutation-testing grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
pub struct UnitTestGraderBuilder {
    /// Requirement name.
    req_name:         Option<String>,
    /// Maximum score.
    out_of:           Option<f64>,
    /// Project under test.
    project:          Option<Project>,
    /// Test classes to run.
    target_test:      Vec<String>,
    /// Classes under mutation.
    target_class:     Vec<String>,
    /// Methods excluded from mutation.
    excluded_methods: Vec<String>,
    /// Classes to avoid calling.
    avoid_calls_to:   Vec<String>,
}

impl UnitTestGrader {}

impl UnitTestGraderBuilder {
    /// Set requirement name.
    pub fn req_name(mut self, name: String) -> Self {
        self.req_name = Some(name);
        self
    }
    /// Set maximum score.
    pub fn out_of(mut self, value: f64) -> Self {
        self.out_of = Some(value);
        self
    }
    /// Set the project under test.
    pub fn project(mut self, project: Ref<Project>) -> Self {
        self.project = Some(project.clone());
        self
    }
    /// Set target tests.
    pub fn target_test(mut self, tests: Vec<String>) -> Self {
        self.target_test = tests;
        self
    }
    /// Set target classes.
    pub fn target_class(mut self, classes: Vec<String>) -> Self {
        self.target_class = classes;
        self
    }
    /// Exclude specific methods.
    pub fn excluded_methods(mut self, methods: Vec<String>) -> Self {
        self.excluded_methods = methods;
        self
    }
    /// Avoid calls to the specified classes.
    pub fn avoid_calls_to(mut self, classes: Vec<String>) -> Self {
        self.avoid_calls_to = classes;
        self
    }

    /// Run the mutation-testing grader.
    pub async fn run(self) -> RuneResult<GradeResult> {
        let builder = grade::UnitTestGrader::builder()
            .target_test(self.target_test)
            .target_class(self.target_class)
            .excluded_methods(self.excluded_methods)
            .avoid_calls_to(self.avoid_calls_to)
            .req_name(take_required(self.req_name, "req_name")?)
            .out_of(take_required(self.out_of, "out_of")?)
            .project(take_required(self.project, "project")?.inner);

        builder
            .build()
            .run()
            .await
            .map(GradeResult::from)
            .map_err(host_err)
    }
}

/// Namespace for hidden-test grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
pub struct ByHiddenTestGrader;

/// Builder for hidden-test grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
pub struct ByHiddenTestGraderBuilder {
    /// URL to fetch hidden tests.
    url:             Option<String>,
    /// Name of hidden test class.
    test_class_name: Option<String>,
    /// Maximum score.
    out_of:          Option<f64>,
    /// Requirement name.
    req_name:        Option<String>,
}

impl ByHiddenTestGrader {}

impl ByHiddenTestGraderBuilder {
    /// Set URL to fetch hidden tests.
    pub fn url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }
    /// Set the test class name.
    pub fn test_class_name(mut self, name: String) -> Self {
        self.test_class_name = Some(name);
        self
    }
    /// Set maximum score.
    pub fn out_of(mut self, out_of: f64) -> Self {
        self.out_of = Some(out_of);
        self
    }
    /// Set requirement name.
    pub fn req_name(mut self, name: String) -> Self {
        self.req_name = Some(name);
        self
    }

    /// Run the hidden-test grader.
    pub async fn run(self) -> RuneResult<GradeResult> {
        let builder = grade::ByHiddenTestGrader::builder()
            .url(take_required(self.url, "url")?)
            .test_class_name(take_required(self.test_class_name, "test_class_name")?)
            .out_of(take_required(self.out_of, "out_of")?)
            .req_name(take_required(self.req_name, "req_name")?);

        builder
            .build()
            .run()
            .await
            .map(GradeResult::from)
            .map_err(host_err)
    }
}

/// Namespace for diff-based grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
pub struct DiffGrader;

/// Builder for diff-based grader.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
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

    /// Run the diff grader.
    pub async fn run(self) -> RuneResult<GradeResult> {
        let builder = grade::DiffGrader::builder()
            .req_name(take_required(self.req_name, "req_name")?)
            .out_of(take_required(self.out_of, "out_of")?)
            .project(take_required(self.project, "project")?.inner)
            .file(take_required(self.file, "file")?)
            .cases(self.cases)
            .ignore_case(self.ignore_case)
            .preserve_whitespace(self.preserve_whitespace);

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
#[rune(item = ::umm::java)]
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
/// Namespace for tree-sitter query graders.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
pub struct QueryGrader;

/// Builder for query graders.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
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

        let constraint = self.constraint.map(|c| c.inner).unwrap_or_default();
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
    grade::show_result(inner_results, config).map_err(host_err)
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
    grade::show_result(inner_results, config.inner).map_err(host_err)
}

/// Render results with config alias.
pub fn show_results_with_config(
    results: Vec<GradeResult>,
    config: RuneGradescopeConfig,
) -> RuneResult<()> {
    show_result_with_config(results, config)
}

/// Install the `umm::java` Rune module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("umm", ["java"])?;

    module.ty::<Project>()?;
    module.ty::<GradeResult>()?;
    module.ty::<DiffCase>()?;
    module.ty::<ProjectPaths>()?;
    module.ty::<ProjectPathsBuilder>()?;
    module.ty::<DocsGrader>()?;
    module.ty::<DocsGraderBuilder>()?;
    module.ty::<ByUnitTestGrader>()?;
    module.ty::<ByUnitTestGraderBuilder>()?;
    module.ty::<UnitTestGrader>()?;
    module.ty::<UnitTestGraderBuilder>()?;
    module.ty::<ByHiddenTestGrader>()?;
    module.ty::<ByHiddenTestGraderBuilder>()?;
    module.ty::<DiffGrader>()?;
    module.ty::<DiffGraderBuilder>()?;
    module.ty::<QueryConstraint>()?;
    module.ty::<QueryGrader>()?;
    module.ty::<QueryGraderBuilder>()?;
    module.associated_function("prompt", GradeResult::prompt)?;

    // Free constructors.
    module.function_meta(new_project)?;
    module.function_meta(new_project_from_paths)?;
    module.function_meta(new_project_paths)?;
    module.function_meta(new_docs_grader)?;
    module.function_meta(new_by_unit_test_grader)?;
    module.function_meta(new_unit_test_grader)?;
    module.function_meta(new_by_hidden_test_grader)?;
    module.function_meta(new_diff_grader)?;
    module.function_meta(new_query_grader)?;

    // Builder setters.
    module.associated_function("root_dir", ProjectPathsBuilder::root_dir)?;
    module.associated_function("source_dir", ProjectPathsBuilder::source_dir)?;
    module.associated_function("build_dir", ProjectPathsBuilder::build_dir)?;
    module.associated_function("test_dir", ProjectPathsBuilder::test_dir)?;
    module.associated_function("lib_dir", ProjectPathsBuilder::lib_dir)?;
    module.associated_function("umm_dir", ProjectPathsBuilder::umm_dir)?;
    module.associated_function("report_dir", ProjectPathsBuilder::report_dir)?;
    module.associated_function("build", ProjectPathsBuilder::build)?;

    module.associated_function("project", DocsGraderBuilder::project)?;
    module.associated_function("files", DocsGraderBuilder::files)?;
    module.associated_function("req_name", DocsGraderBuilder::req_name)?;
    module.associated_function("out_of", DocsGraderBuilder::out_of)?;
    module.associated_function("penalty", DocsGraderBuilder::penalty)?;
    module.associated_function("run", DocsGraderBuilder::run)?;

    module.associated_function("test_files", ByUnitTestGraderBuilder::test_files)?;
    module.associated_function("expected_tests", ByUnitTestGraderBuilder::expected_tests)?;
    module.associated_function("project", ByUnitTestGraderBuilder::project)?;
    module.associated_function("out_of", ByUnitTestGraderBuilder::out_of)?;
    module.associated_function("req_name", ByUnitTestGraderBuilder::req_name)?;
    module.associated_function("run", ByUnitTestGraderBuilder::run)?;

    module.associated_function("req_name", UnitTestGraderBuilder::req_name)?;
    module.associated_function("out_of", UnitTestGraderBuilder::out_of)?;
    module.associated_function("project", UnitTestGraderBuilder::project)?;
    module.associated_function("target_test", UnitTestGraderBuilder::target_test)?;
    module.associated_function("target_class", UnitTestGraderBuilder::target_class)?;
    module.associated_function("excluded_methods", UnitTestGraderBuilder::excluded_methods)?;
    module.associated_function("avoid_calls_to", UnitTestGraderBuilder::avoid_calls_to)?;
    module.associated_function("run", UnitTestGraderBuilder::run)?;

    module.associated_function("url", ByHiddenTestGraderBuilder::url)?;
    module.associated_function("test_class_name", ByHiddenTestGraderBuilder::test_class_name)?;
    module.associated_function("out_of", ByHiddenTestGraderBuilder::out_of)?;
    module.associated_function("req_name", ByHiddenTestGraderBuilder::req_name)?;
    module.associated_function("run", ByHiddenTestGraderBuilder::run)?;

    module.associated_function("req_name", DiffGraderBuilder::req_name)?;
    module.associated_function("out_of", DiffGraderBuilder::out_of)?;
    module.associated_function("project", DiffGraderBuilder::project)?;
    module.associated_function("file", DiffGraderBuilder::file)?;
    module.associated_function("cases", DiffGraderBuilder::cases)?;
    module.associated_function("ignore_case", DiffGraderBuilder::ignore_case)?;
    module.associated_function("preserve_whitespace", DiffGraderBuilder::preserve_whitespace)?;
    module.associated_function("run", DiffGraderBuilder::run)?;

    module.function_meta(QueryConstraint::must_match_at_least_once)?;
    module.function_meta(QueryConstraint::must_match_exactly_n)?;
    module.function_meta(QueryConstraint::must_not_match)?;

    // Query grader builder setters.
    module.associated_function("req_name", QueryGraderBuilder::req_name)?;
    module.associated_function("out_of", QueryGraderBuilder::out_of)?;
    module.associated_function("project", QueryGraderBuilder::project)?;
    module.associated_function("file", QueryGraderBuilder::file)?;
    module.associated_function("queries", QueryGraderBuilder::queries)?;
    module.associated_function("queries_with_capture", QueryGraderBuilder::queries_with_capture)?;
    module.associated_function("constraint", QueryGraderBuilder::constraint)?;
    module.associated_function("reason", QueryGraderBuilder::reason)?;
    module.associated_function("run", QueryGraderBuilder::run)?;

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
