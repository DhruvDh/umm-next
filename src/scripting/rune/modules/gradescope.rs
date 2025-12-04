use std::collections::HashSet;

use rune::{
    Any, ContextError, Module,
    support::{Error as RuneError, Result as RuneResult},
};

use crate::{
    java::grade::{self, gradescope::GradescopeConfig as InnerGradescopeConfig},
    scripting::rune::modules::java::GradeResult,
};

/// Map host errors into Rune errors with string messages.
fn host_err<E: std::fmt::Display>(e: E) -> RuneError {
    RuneError::msg(e.to_string())
}

/// Output formats supported when emitting Gradescope artifacts.
#[derive(Any, Clone, Copy)]
#[rune(item = ::umm::gradescope)]
pub enum GradescopeOutputFormat {
    /// Plain text output.
    Text,
    /// HTML output.
    Html,
    /// Simple HTML with newline conversion.
    SimpleFormat,
    /// Markdown output.
    Md,
    /// ANSI-colored text output.
    Ansi,
}

impl GradescopeOutputFormat {
    #[rune::function(path = GradescopeOutputFormat::Text)]
    /// Plain text output format.
    pub fn text() -> Self {
        GradescopeOutputFormat::Text
    }

    #[rune::function(path = GradescopeOutputFormat::Html)]
    /// HTML output format.
    pub fn html() -> Self {
        GradescopeOutputFormat::Html
    }

    #[rune::function(path = GradescopeOutputFormat::SimpleFormat)]
    /// Simple HTML output that converts newlines.
    pub fn simple_format() -> Self {
        GradescopeOutputFormat::SimpleFormat
    }

    #[rune::function(path = GradescopeOutputFormat::Md)]
    /// Markdown output format.
    pub fn md() -> Self {
        GradescopeOutputFormat::Md
    }

    #[rune::function(path = GradescopeOutputFormat::Ansi)]
    /// ANSI-colored text output format.
    pub fn ansi() -> Self {
        GradescopeOutputFormat::Ansi
    }
}

/// Visibility options for Gradescope output.
#[derive(Any, Clone, Copy)]
#[rune(item = ::umm::gradescope)]
pub enum GradescopeVisibility {
    /// Hidden from students.
    Hidden,
    /// Visible after due date.
    AfterDueDate,
    /// Visible after grades are published.
    AfterPublished,
    /// Always visible.
    Visible,
}

impl GradescopeVisibility {
    #[rune::function(path = GradescopeVisibility::Hidden)]
    /// Hidden from students.
    pub fn hidden() -> Self {
        GradescopeVisibility::Hidden
    }

    #[rune::function(path = GradescopeVisibility::AfterDueDate)]
    /// Visible after the due date.
    pub fn after_due_date() -> Self {
        GradescopeVisibility::AfterDueDate
    }

    #[rune::function(path = GradescopeVisibility::AfterPublished)]
    /// Visible after grades are published.
    pub fn after_published() -> Self {
        GradescopeVisibility::AfterPublished
    }

    #[rune::function(path = GradescopeVisibility::Visible)]
    /// Always visible to students.
    pub fn visible() -> Self {
        GradescopeVisibility::Visible
    }
}

/// Rune wrapper around the Gradescope configuration.
#[derive(Any, Clone)]
#[rune(item = ::umm::gradescope)]
pub struct GradescopeConfig {
    /// Wrapped Rust configuration.
    pub(crate) inner: InnerGradescopeConfig,
}

/// Builder exposed to Rune for configuring Gradescope output.
#[derive(Any, Clone)]
#[rune(item = ::umm::gradescope)]
pub struct GradescopeConfigBuilder {
    /// Source files included in summaries.
    source_files:        Vec<String>,
    /// Test files included in summaries.
    test_files:          Vec<String>,
    /// Gradescope project title.
    project_title:       Option<String>,
    /// Gradescope project description.
    project_description: Option<String>,
    /// Pass threshold.
    pass_threshold:      Option<f64>,
    /// Whether to show overview table.
    show_table:          Option<bool>,
    /// Emit Gradescope JSON.
    results_json:        Option<bool>,
    /// Emit feedback via Supabase.
    feedback:            Option<bool>,
    /// Write JSON locally for debugging.
    debug:               Option<bool>,
    /// Enabled SLO identifiers.
    enabled_slos:        HashSet<String>,
}

impl GradescopeConfig {
    #[rune::function(path = GradescopeConfig::builder)]
    /// Start a Gradescope configuration builder.
    pub fn builder() -> GradescopeConfigBuilder {
        GradescopeConfigBuilder {
            source_files:        Vec::new(),
            test_files:          Vec::new(),
            project_title:       None,
            project_description: None,
            pass_threshold:      None,
            show_table:          None,
            results_json:        None,
            feedback:            None,
            debug:               None,
            enabled_slos:        HashSet::new(),
        }
    }
}

impl GradescopeConfigBuilder {
    /// Set source files to include in summaries.
    pub fn source_files(mut self, files: Vec<String>) -> Self {
        self.source_files = files;
        self
    }
    /// Set test files to include in summaries.
    pub fn test_files(mut self, files: Vec<String>) -> Self {
        self.test_files = files;
        self
    }
    /// Set project title.
    pub fn project_title(mut self, title: String) -> Self {
        self.project_title = Some(title);
        self
    }
    /// Set project description.
    pub fn project_description(mut self, description: String) -> Self {
        self.project_description = Some(description);
        self
    }
    /// Set pass threshold.
    pub fn pass_threshold(mut self, value: f64) -> Self {
        self.pass_threshold = Some(value);
        self
    }
    /// Toggle overview table.
    pub fn show_table(mut self, value: bool) -> Self {
        self.show_table = Some(value);
        self
    }
    /// Toggle JSON emission.
    pub fn results_json(mut self, value: bool) -> Self {
        self.results_json = Some(value);
        self
    }
    /// Toggle feedback emission.
    pub fn feedback(mut self, value: bool) -> Self {
        self.feedback = Some(value);
        self
    }
    /// Toggle debug output (writes results.json locally).
    pub fn debug(mut self, value: bool) -> Self {
        self.debug = Some(value);
        self
    }
    /// Enable specific SLO identifiers.
    pub fn enabled_slos(mut self, slos: Vec<String>) -> Self {
        self.enabled_slos = slos.into_iter().collect();
        self
    }

    /// Finalize the configuration.
    pub fn build(self) -> GradescopeConfig {
        let defaults = InnerGradescopeConfig::default();
        let inner = InnerGradescopeConfig {
            source_files:        self.source_files,
            test_files:          self.test_files,
            project_title:       self.project_title.unwrap_or(defaults.project_title),
            project_description: self
                .project_description
                .unwrap_or(defaults.project_description),
            pass_threshold:      self.pass_threshold.unwrap_or(defaults.pass_threshold),
            show_table:          self.show_table.unwrap_or(defaults.show_table),
            results_json:        self.results_json.unwrap_or(defaults.results_json),
            feedback:            self.feedback.unwrap_or(defaults.feedback),
            debug:               self.debug.unwrap_or(defaults.debug),
            enabled_slos:        if self.enabled_slos.is_empty() {
                defaults.enabled_slos
            } else {
                self.enabled_slos
            },
        };
        GradescopeConfig { inner }
    }
}

/// Render results using default Gradescope configuration.
pub fn show_result(results: Vec<GradeResult>) -> RuneResult<()> {
    let config = InnerGradescopeConfig::default();
    let inner_results: Vec<_> = results.into_iter().map(|r| r.into_inner()).collect();
    grade::show_result(inner_results, config).map_err(host_err)
}

/// Render results with an explicit Gradescope configuration.
pub fn show_result_with_config(
    results: Vec<GradeResult>,
    config: GradescopeConfig,
) -> RuneResult<()> {
    let inner_results: Vec<_> = results.into_iter().map(|r| r.into_inner()).collect();
    grade::show_result(inner_results, config.inner).map_err(host_err)
}

/// Alias for `show_result`.
pub fn show_results(results: Vec<GradeResult>) -> RuneResult<()> {
    show_result(results)
}

/// Alias for `show_result_with_config`.
pub fn show_results_with_config(
    results: Vec<GradeResult>,
    config: GradescopeConfig,
) -> RuneResult<()> {
    show_result_with_config(results, config)
}

/// Install the `umm::gradescope` Rune module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("umm", ["gradescope"])?;

    module.ty::<GradescopeOutputFormat>()?;
    module.ty::<GradescopeVisibility>()?;
    module.ty::<GradescopeConfig>()?;
    module.ty::<GradescopeConfigBuilder>()?;

    module.function_meta(GradescopeConfig::builder)?;
    module.associated_function("source_files", GradescopeConfigBuilder::source_files)?;
    module.associated_function("test_files", GradescopeConfigBuilder::test_files)?;
    module.associated_function("project_title", GradescopeConfigBuilder::project_title)?;
    module
        .associated_function("project_description", GradescopeConfigBuilder::project_description)?;
    module.associated_function("pass_threshold", GradescopeConfigBuilder::pass_threshold)?;
    module.associated_function("show_table", GradescopeConfigBuilder::show_table)?;
    module.associated_function("results_json", GradescopeConfigBuilder::results_json)?;
    module.associated_function("feedback", GradescopeConfigBuilder::feedback)?;
    module.associated_function("debug", GradescopeConfigBuilder::debug)?;
    module.associated_function("enabled_slos", GradescopeConfigBuilder::enabled_slos)?;
    module.associated_function("build", GradescopeConfigBuilder::build)?;

    module.function("show_result", show_result).build()?;
    module
        .function("show_result_with_config", show_result_with_config)
        .build()?;
    module.function("show_results", show_results).build()?;
    module
        .function("show_results_with_config", show_results_with_config)
        .build()?;

    module.function_meta(GradescopeOutputFormat::text)?;
    module.function_meta(GradescopeOutputFormat::html)?;
    module.function_meta(GradescopeOutputFormat::simple_format)?;
    module.function_meta(GradescopeOutputFormat::md)?;
    module.function_meta(GradescopeOutputFormat::ansi)?;

    module.function_meta(GradescopeVisibility::hidden)?;
    module.function_meta(GradescopeVisibility::after_due_date)?;
    module.function_meta(GradescopeVisibility::after_published)?;
    module.function_meta(GradescopeVisibility::visible)?;
    Ok(module)
}
