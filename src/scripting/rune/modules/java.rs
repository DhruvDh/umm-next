use rune::{
    Any, ContextError, Module,
    support::{Error as RuneError, Result as RuneResult},
};

use crate::{
    java::grade::{self, GradeResult as InnerGradeResult},
    scripting::rune::modules::gradescope::GradescopeConfig as RuneGradescopeConfig,
};

/// Map host errors into Rune errors with readable messages.
fn host_err<E: std::fmt::Display>(e: E) -> RuneError {
    RuneError::msg(e.to_string())
}

/// Rune-exposed wrapper around the Java `Project` discovery.
#[derive(Any, Clone)]
#[rune(item = ::umm::java)]
pub struct Project {
    /// Underlying Rust project instance.
    inner: crate::java::Project,
}

impl Project {
    #[rune::function(path = Project::new)]
    /// Discover the current Java project and return a wrapped `Project`.
    pub fn new() -> RuneResult<Self> {
        Ok(Self {
            inner: crate::java::Project::new().map_err(host_err)?,
        })
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

impl DocsGrader {
    #[rune::function(path = DocsGrader::builder)]
    /// Start building a docs grader.
    pub fn builder() -> DocsGraderBuilder {
        DocsGraderBuilder {
            project:  None,
            files:    Vec::new(),
            req_name: None,
            out_of:   None,
            penalty:  None,
        }
    }
}

impl DocsGraderBuilder {
    /// Set the project to grade.
    pub fn project(mut self, project: Project) -> Self {
        self.project = Some(project);
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

    /// Validate required fields and run the grader.
    pub async fn run(self) -> RuneResult<GradeResult> {
        let mut missing = Vec::new();
        if self.project.is_none() {
            missing.push("project");
        }
        if self.files.is_empty() {
            missing.push("files");
        }
        if self.req_name.is_none() {
            missing.push("req_name");
        }
        if self.out_of.is_none() {
            missing.push("out_of");
        }
        if !missing.is_empty() {
            return Err(host_err(format!(
                "DocsGrader missing required fields: {}",
                missing.join(", ")
            )));
        }

        let grader = grade::DocsGrader {
            project:  self.project.unwrap().inner,
            files:    self.files,
            req_name: self.req_name.unwrap(),
            out_of:   self.out_of.unwrap(),
            penalty:  self.penalty.unwrap_or(3.0),
        };

        grader.run().await.map(GradeResult::from).map_err(host_err)
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

impl ByUnitTestGrader {
    #[rune::function(path = ByUnitTestGrader::builder)]
    /// Start building a visible unit-test grader.
    pub fn builder() -> ByUnitTestGraderBuilder {
        ByUnitTestGraderBuilder {
            test_files:     Vec::new(),
            expected_tests: Vec::new(),
            project:        None,
            out_of:         None,
            req_name:       None,
        }
    }
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
    pub fn project(mut self, project: Project) -> Self {
        self.project = Some(project);
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
        let mut missing = Vec::new();
        if self.project.is_none() {
            missing.push("project");
        }
        if self.test_files.is_empty() {
            missing.push("test_files");
        }
        if self.req_name.is_none() {
            missing.push("req_name");
        }
        if self.out_of.is_none() {
            missing.push("out_of");
        }
        if !missing.is_empty() {
            return Err(host_err(format!(
                "ByUnitTestGrader missing required fields: {}",
                missing.join(", ")
            )));
        }

        let builder = grade::ByUnitTestGrader::builder()
            .test_files(self.test_files)
            .expected_tests(self.expected_tests)
            .project(self.project.unwrap().inner)
            .out_of(self.out_of.unwrap())
            .req_name(self.req_name.unwrap());

        builder.run().await.map(GradeResult::from).map_err(host_err)
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
    /// Test classes to run.
    target_test:      Vec<String>,
    /// Classes under mutation.
    target_class:     Vec<String>,
    /// Methods excluded from mutation.
    excluded_methods: Vec<String>,
    /// Classes to avoid calling.
    avoid_calls_to:   Vec<String>,
}

impl UnitTestGrader {
    #[rune::function(path = UnitTestGrader::builder)]
    /// Start building a mutation-testing grader.
    pub fn builder() -> UnitTestGraderBuilder {
        UnitTestGraderBuilder {
            req_name:         None,
            out_of:           None,
            target_test:      Vec::new(),
            target_class:     Vec::new(),
            excluded_methods: Vec::new(),
            avoid_calls_to:   Vec::new(),
        }
    }
}

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
        let mut missing = Vec::new();
        if self.req_name.is_none() {
            missing.push("req_name");
        }
        if self.out_of.is_none() {
            missing.push("out_of");
        }
        if self.target_test.is_empty() {
            missing.push("target_test");
        }
        if self.target_class.is_empty() {
            missing.push("target_class");
        }
        if !missing.is_empty() {
            return Err(host_err(format!(
                "UnitTestGrader missing required fields: {}",
                missing.join(", ")
            )));
        }

        let grader = grade::UnitTestGrader {
            req_name:         self.req_name.unwrap(),
            out_of:           self.out_of.unwrap(),
            target_test:      self.target_test,
            target_class:     self.target_class,
            excluded_methods: self.excluded_methods,
            avoid_calls_to:   self.avoid_calls_to,
        };

        grader.run().await.map(GradeResult::from).map_err(host_err)
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

impl ByHiddenTestGrader {
    #[rune::function(path = ByHiddenTestGrader::builder)]
    /// Start building a hidden-test grader.
    pub fn builder() -> ByHiddenTestGraderBuilder {
        ByHiddenTestGraderBuilder {
            url:             None,
            test_class_name: None,
            out_of:          None,
            req_name:        None,
        }
    }
}

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
        let mut missing = Vec::new();
        if self.url.is_none() {
            missing.push("url");
        }
        if self.test_class_name.is_none() {
            missing.push("test_class_name");
        }
        if self.req_name.is_none() {
            missing.push("req_name");
        }
        if self.out_of.is_none() {
            missing.push("out_of");
        }
        if !missing.is_empty() {
            return Err(host_err(format!(
                "ByHiddenTestGrader missing required fields: {}",
                missing.join(", ")
            )));
        }

        let grader = grade::ByHiddenTestGrader {
            url:             self.url.unwrap(),
            test_class_name: self.test_class_name.unwrap(),
            out_of:          self.out_of.unwrap(),
            req_name:        self.req_name.unwrap(),
        };

        grader.run().await.map(GradeResult::from).map_err(host_err)
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

impl DiffGrader {
    #[rune::function(path = DiffGrader::builder)]
    /// Start building a diff grader.
    pub fn builder() -> DiffGraderBuilder {
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
}

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
    pub fn project(mut self, project: Project) -> Self {
        self.project = Some(project);
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
        let mut missing = Vec::new();
        if self.project.is_none() {
            missing.push("project");
        }
        if self.file.is_none() {
            missing.push("file");
        }
        if self.req_name.is_none() {
            missing.push("req_name");
        }
        if self.out_of.is_none() {
            missing.push("out_of");
        }
        if self.cases.is_empty() {
            missing.push("cases");
        }
        if !missing.is_empty() {
            return Err(host_err(format!(
                "DiffGrader missing required fields: {}",
                missing.join(", ")
            )));
        }

        let builder = grade::DiffGrader::builder()
            .req_name(self.req_name.unwrap())
            .out_of(self.out_of.unwrap())
            .project(self.project.unwrap().inner)
            .file(self.file.unwrap())
            .cases(self.cases)
            .ignore_case(self.ignore_case)
            .preserve_whitespace(self.preserve_whitespace);

        builder.run().await.map(GradeResult::from).map_err(host_err)
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

impl QueryGrader {
    #[rune::function(path = QueryGrader::builder)]
    /// Start building a query grader.
    pub fn builder() -> QueryGraderBuilder {
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
}

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
    pub fn project(mut self, project: Project) -> Self {
        self.project = Some(project);
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
        let mut missing = Vec::new();
        if self.req_name.is_none() {
            missing.push("req_name");
        }
        if self.out_of.is_none() {
            missing.push("out_of");
        }
        if self.project.is_none() {
            missing.push("project");
        }
        if self.file.is_none() {
            missing.push("file");
        }
        if self.queries.is_empty() {
            missing.push("queries");
        }
        if !missing.is_empty() {
            return Err(host_err(format!(
                "QueryGrader missing required fields: {}",
                missing.join(", ")
            )));
        }

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

        let grader = grade::QueryGrader::builder()
            .req_name(self.req_name.unwrap())
            .out_of(self.out_of.unwrap())
            .queries(queries)
            .project(self.project.unwrap().inner)
            .file(self.file.unwrap())
            .constraint(constraint)
            .reason(self.reason.unwrap_or_default());

        grader.run().map(GradeResult::from).map_err(host_err)
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

    module.function_meta(Project::new)?;

    module.function_meta(DocsGrader::builder)?;
    module.associated_function("project", DocsGraderBuilder::project)?;
    module.associated_function("files", DocsGraderBuilder::files)?;
    module.associated_function("req_name", DocsGraderBuilder::req_name)?;
    module.associated_function("out_of", DocsGraderBuilder::out_of)?;
    module.associated_function("penalty", DocsGraderBuilder::penalty)?;
    module.associated_function("run", DocsGraderBuilder::run)?;

    module.function_meta(ByUnitTestGrader::builder)?;
    module.associated_function("test_files", ByUnitTestGraderBuilder::test_files)?;
    module.associated_function("expected_tests", ByUnitTestGraderBuilder::expected_tests)?;
    module.associated_function("project", ByUnitTestGraderBuilder::project)?;
    module.associated_function("out_of", ByUnitTestGraderBuilder::out_of)?;
    module.associated_function("req_name", ByUnitTestGraderBuilder::req_name)?;
    module.associated_function("run", ByUnitTestGraderBuilder::run)?;

    module.function_meta(UnitTestGrader::builder)?;
    module.associated_function("req_name", UnitTestGraderBuilder::req_name)?;
    module.associated_function("out_of", UnitTestGraderBuilder::out_of)?;
    module.associated_function("target_test", UnitTestGraderBuilder::target_test)?;
    module.associated_function("target_class", UnitTestGraderBuilder::target_class)?;
    module.associated_function("excluded_methods", UnitTestGraderBuilder::excluded_methods)?;
    module.associated_function("avoid_calls_to", UnitTestGraderBuilder::avoid_calls_to)?;
    module.associated_function("run", UnitTestGraderBuilder::run)?;

    module.function_meta(ByHiddenTestGrader::builder)?;
    module.associated_function("url", ByHiddenTestGraderBuilder::url)?;
    module.associated_function("test_class_name", ByHiddenTestGraderBuilder::test_class_name)?;
    module.associated_function("out_of", ByHiddenTestGraderBuilder::out_of)?;
    module.associated_function("req_name", ByHiddenTestGraderBuilder::req_name)?;
    module.associated_function("run", ByHiddenTestGraderBuilder::run)?;

    module.function_meta(DiffGrader::builder)?;
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

    module.function_meta(QueryGrader::builder)?;
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
