#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::sync::Arc;

use anyhow::{Context, Result};
use postgrest::Postgrest;
use state::InitCell;
use tokio::runtime::Runtime;

/// Holds prompt strings that will eventually become script-configurable.
#[derive(Clone)]
pub struct Prompts {
    /// Intro portion of the primary system prompt.
    system_message_intro: String,
    /// Outro portion of the primary system prompt.
    system_message_outro: String,
    /// Full system prompt assembled from intro/outro.
    system_message: String,
    /// Intro prompt used when active retrieval is enabled.
    retrieval_message_intro: String,
    /// Outro prompt used when active retrieval is enabled.
    retrieval_message_outro: String,
    /// SLO template for Algorithmic Solutions feedback.
    algorithmic_solutions_slo: String,
    /// SLO template for Code Readability feedback.
    code_readability_slo: String,
    /// SLO template for comments feedback.
    comments_written_slo: String,
    /// SLO template for error-handling feedback.
    error_handling_slo: String,
    /// SLO template for logic feedback.
    logic_slo: String,
    /// SLO template for naming conventions feedback.
    naming_conventions_slo: String,
    /// SLO template for OOP feedback.
    object_oriented_programming_slo: String,
    /// SLO template for syntax feedback.
    syntax_slo: String,
    /// SLO template for testing feedback.
    testing_slo: String,
}

impl Prompts {
    /// Load prompt templates from disk.
    fn load() -> Self {
        let system_message_intro = include_str!("prompts/system_message_intro.md").to_string();
        let system_message_outro = include_str!("prompts/system_message_outro.md").to_string();
        let system_message = format!("{}\n{}", system_message_intro, system_message_outro);

        let retrieval_message_intro =
            include_str!("prompts/retrieval_system_message_intro.md").into();
        let retrieval_message_outro =
            include_str!("prompts/retrieval_system_message_outro.md").into();

        Self {
            system_message_intro,
            system_message_outro,
            system_message,
            retrieval_message_intro,
            retrieval_message_outro,
            algorithmic_solutions_slo: format!(
                include_str!("prompts/slos/system_message_intro.md"),
                SLO_DESCRIPTION = include_str!("prompts/slos/algorithmic_solutions_quant.md"),
            ),
            code_readability_slo: format!(
                include_str!("prompts/slos/system_message_intro.md"),
                SLO_DESCRIPTION = include_str!("prompts/slos/code_readability_written_com.md"),
            ),
            comments_written_slo: format!(
                include_str!("prompts/slos/system_message_intro.md"),
                SLO_DESCRIPTION = include_str!("prompts/slos/comments_written_com.md"),
            ),
            error_handling_slo: format!(
                include_str!("prompts/slos/system_message_intro.md"),
                SLO_DESCRIPTION = include_str!("prompts/slos/error_handling_verification.md"),
            ),
            logic_slo: format!(
                include_str!("prompts/slos/system_message_intro.md"),
                SLO_DESCRIPTION = include_str!("prompts/slos/logic_programming.md"),
            ),
            naming_conventions_slo: format!(
                include_str!("prompts/slos/system_message_intro.md"),
                SLO_DESCRIPTION = include_str!("prompts/slos/naming_written_com.md"),
            ),
            object_oriented_programming_slo: format!(
                include_str!("prompts/slos/system_message_intro.md"),
                SLO_DESCRIPTION = include_str!("prompts/slos/oop_programming.md"),
            ),
            syntax_slo: format!(
                include_str!("prompts/slos/system_message_intro.md"),
                SLO_DESCRIPTION = include_str!("prompts/slos/syntax_programming.md"),
            ),
            testing_slo: format!(
                include_str!("prompts/slos/system_message_intro.md"),
                SLO_DESCRIPTION = include_str!("prompts/slos/testing_verification.md"),
            ),
        }
    }

    /// Returns the primary system prompt delivered to ChatGPT.
    pub fn system_message(&self) -> &str {
        &self.system_message
    }

    /// Returns the intro segment of the system prompt.
    pub fn system_message_intro(&self) -> &str {
        &self.system_message_intro
    }

    /// Returns the outro segment of the system prompt.
    pub fn system_message_outro(&self) -> &str {
        &self.system_message_outro
    }

    /// Returns the retrieval system message intro.
    pub fn retrieval_message_intro(&self) -> &str {
        &self.retrieval_message_intro
    }

    /// Returns the retrieval system message outro.
    pub fn retrieval_message_outro(&self) -> &str {
        &self.retrieval_message_outro
    }

    /// Returns the algorithmic solutions SLO prompt.
    pub fn algorithmic_solutions_slo(&self) -> &str {
        &self.algorithmic_solutions_slo
    }

    /// Returns the code readability SLO prompt.
    pub fn code_readability_slo(&self) -> &str {
        &self.code_readability_slo
    }

    /// Returns the comments written SLO prompt.
    pub fn comments_written_slo(&self) -> &str {
        &self.comments_written_slo
    }

    /// Returns the error handling SLO prompt.
    pub fn error_handling_slo(&self) -> &str {
        &self.error_handling_slo
    }

    /// Returns the logic SLO prompt.
    pub fn logic_slo(&self) -> &str {
        &self.logic_slo
    }

    /// Returns the naming conventions SLO prompt.
    pub fn naming_conventions_slo(&self) -> &str {
        &self.naming_conventions_slo
    }

    /// Returns the object oriented programming SLO prompt.
    pub fn object_oriented_programming_slo(&self) -> &str {
        &self.object_oriented_programming_slo
    }

    /// Returns the syntax SLO prompt.
    pub fn syntax_slo(&self) -> &str {
        &self.syntax_slo
    }

    /// Returns the testing SLO prompt.
    pub fn testing_slo(&self) -> &str {
        &self.testing_slo
    }
}

/// Runtime and prompt configuration shared across the crate.
#[derive(Clone)]
pub struct Config {
    /// Supabase PostgREST client seeded with the API key.
    postgrest: Postgrest,
    /// Shared Tokio runtime for async helpers (downloads, Supabase calls).
    runtime:   Arc<Runtime>,
    /// Loaded prompt catalog used by graders and retrieval helpers.
    prompts:   Prompts,
    /// Course identifier exposed to Supabase-backed endpoints.
    course:    String,
    /// Academic term identifier exposed to Supabase-backed endpoints.
    term:      String,
}

impl Config {
    /// Construct a new configuration instance by reading environment and prompt
    /// assets.
    fn new() -> Result<Self> {
        let supabase_url = get_required_env("SUPABASE_URL");
        let supabase_key = get_required_env("SUPABASE_ANON_KEY");
        let rest_url = format!("{}/rest/v1", supabase_url.trim_end_matches('/'));
        let postgrest = Postgrest::new(rest_url).insert_header("apiKey", supabase_key);

        let runtime = Runtime::new().context("Failed to create shared Tokio runtime")?;
        let prompts = Prompts::load();

        let course = std::env::var("UMM_COURSE").unwrap_or_else(|_| "ITSC 2214".to_string());
        let term = std::env::var("UMM_TERM").unwrap_or_else(|_| "Fall 2022".to_string());

        Ok(Self {
            postgrest,
            runtime: Arc::new(runtime),
            prompts,
            course,
            term,
        })
    }

    /// Returns the configured PostgREST client.
    pub fn postgrest(&self) -> Postgrest {
        self.postgrest.clone()
    }

    /// Returns the shared Tokio runtime.
    pub fn runtime(&self) -> Arc<Runtime> {
        self.runtime.clone()
    }

    /// Returns the course identifier.
    pub fn course(&self) -> &str {
        &self.course
    }

    /// Returns the academic term identifier.
    pub fn term(&self) -> &str {
        &self.term
    }

    /// Returns the prompt bundle.
    pub fn prompts(&self) -> &Prompts {
        &self.prompts
    }
}

/// Lazily initialized singleton backing the configuration accessors.
static CONFIG: InitCell<Config> = InitCell::new();

/// Ensure the global configuration has been initialized and return a reference
/// to it.
pub fn ensure_initialized() -> Result<&'static Config> {
    if let Some(config) = CONFIG.try_get() {
        return Ok(config);
    }

    let config = Config::new()?;
    CONFIG.set(config);
    Ok(CONFIG.get())
}

/// Returns the active configuration, initializing it on demand.
pub fn get() -> &'static Config {
    ensure_initialized().expect("configuration initialization failed")
}

/// Returns the configured PostgREST client.
pub fn postgrest_client() -> Postgrest {
    get().postgrest()
}

/// Returns a clone of the shared Tokio runtime.
pub fn runtime() -> Arc<Runtime> {
    get().runtime()
}

/// Returns the configured course identifier.
pub fn course() -> &'static str {
    get().course()
}

/// Returns the configured term identifier.
pub fn term() -> &'static str {
    get().term()
}

/// Returns the configured prompts.
pub fn prompts() -> &'static Prompts {
    get().prompts()
}

/// Fetch an environment variable or terminate with a helpful message.
fn get_required_env(key: &str) -> String {
    match std::env::var(key) {
        Ok(v) => v,
        Err(_) => {
            eprintln!(
                "Missing required environment variable: {}.\n\nSet {} to configure Supabase and \
                 try again.",
                key, key
            );
            std::process::exit(2);
        }
    }
}
