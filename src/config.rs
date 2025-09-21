#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::sync::Arc;

use anyhow::{Context, Result};
use async_openai::types::ReasoningEffort;
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

/// Supabase credentials loaded from the environment, if available.
#[derive(Clone)]
struct SupabaseEnv {
    /// Fully qualified PostgREST endpoint.
    rest_endpoint: String,
    /// API key used for PostgREST requests.
    api_key:       String,
}

impl SupabaseEnv {
    /// Builds a Supabase credential bundle from environment-provided values.
    fn new(url: String, key: String) -> Self {
        let rest_endpoint = format!("{}/rest/v1", url.trim_end_matches('/'));
        Self {
            rest_endpoint,
            api_key: key,
        }
    }
}

/// Parses the optional reasoning-effort environment value into the OpenAI enum,
/// defaulting to `ReasoningEffort::Medium` when unset or unrecognised.
fn parse_reasoning_effort(val: Option<String>) -> ReasoningEffort {
    match val
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
        .unwrap_or("medium")
    {
        "low" => ReasoningEffort::Low,
        "high" => ReasoningEffort::High,
        _ => ReasoningEffort::Medium,
    }
}

/// OpenAI credentials and optional tuning parameters sourced from the
/// environment.
pub struct OpenAiEnv {
    /// Base URL for the OpenAI-compatible API endpoint.
    api_base:         String,
    /// API key used to authenticate OpenAI requests.
    api_key:          String,
    /// Default model identifier for chat completions.
    model:            String,
    /// Optional temperature override, if provided.
    temperature:      Option<f32>,
    /// Optional top-p override, if provided.
    top_p:            Option<f32>,
    /// Reasoning effort hint to send with requests.
    reasoning_effort: ReasoningEffort,
}

impl OpenAiEnv {
    /// Construct an `OpenAiEnv` from environment variables; returns `None` if
    /// any required field is missing.
    fn from_env() -> Option<Self> {
        let api_base = std::env::var("OPENAI_ENDPOINT").ok()?.trim().to_owned();
        let api_key = std::env::var("OPENAI_API_KEY_SLO").ok()?.trim().to_owned();
        let model = std::env::var("OPENAI_MODEL").ok()?.trim().to_owned();

        if api_base.is_empty() || api_key.is_empty() || model.is_empty() {
            return None;
        }

        let temperature = std::env::var("OPENAI_TEMPERATURE")
            .ok()
            .and_then(|s| s.parse::<f32>().ok());
        let top_p = std::env::var("OPENAI_TOP_P")
            .ok()
            .and_then(|s| s.parse::<f32>().ok());
        let reasoning_effort =
            parse_reasoning_effort(std::env::var("OPENAI_REASONING_EFFORT").ok());

        Some(Self {
            api_base,
            api_key,
            model,
            temperature,
            top_p,
            reasoning_effort,
        })
    }

    /// Returns the API base URL used for OpenAI requests.
    pub fn api_base(&self) -> &str {
        &self.api_base
    }

    /// Returns the API key used for OpenAI requests.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Returns the default model identifier.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Returns the configured temperature, if any.
    pub fn temperature(&self) -> Option<f32> {
        self.temperature
    }

    /// Returns the configured top_p, if any.
    pub fn top_p(&self) -> Option<f32> {
        self.top_p
    }

    /// Returns the reasoning effort level (defaults to Medium when
    /// unspecified).
    pub fn reasoning_effort(&self) -> ReasoningEffort {
        self.reasoning_effort.clone()
    }
}

impl Clone for OpenAiEnv {
    fn clone(&self) -> Self {
        #[allow(clippy::needless_match)]
        let reasoning_effort = match self.reasoning_effort {
            ReasoningEffort::Low => ReasoningEffort::Low,
            ReasoningEffort::Medium => ReasoningEffort::Medium,
            ReasoningEffort::High => ReasoningEffort::High,
            ReasoningEffort::Minimal => ReasoningEffort::Minimal,
        };

        Self {
            api_base: self.api_base.clone(),
            api_key: self.api_key.clone(),
            model: self.model.clone(),
            temperature: self.temperature,
            top_p: self.top_p,
            reasoning_effort,
        }
    }
}

/// Runtime and prompt configuration shared across the crate.
pub struct Config {
    /// Supabase credentials, if configured.
    supabase:  Option<SupabaseEnv>,
    /// Lazily constructed Supabase PostgREST client.
    postgrest: InitCell<Postgrest>,
    /// Shared Tokio runtime for async helpers (downloads, Supabase calls).
    runtime:   Arc<Runtime>,
    /// Loaded prompt catalog used by graders and retrieval helpers.
    prompts:   Prompts,
    /// Course identifier exposed to Supabase-backed endpoints.
    course:    String,
    /// Academic term identifier exposed to Supabase-backed endpoints.
    term:      String,
    /// Cached OpenAI configuration, if available.
    openai:    Option<OpenAiEnv>,
}

impl Config {
    /// Construct a new configuration instance by reading environment and prompt
    /// assets.
    fn new() -> Result<Self> {
        let supabase =
            match (std::env::var("SUPABASE_URL").ok(), std::env::var("SUPABASE_ANON_KEY").ok()) {
                (Some(url), Some(key)) if !url.trim().is_empty() && !key.trim().is_empty() => {
                    Some(SupabaseEnv::new(url, key))
                }
                _ => None,
            };

        let runtime = Runtime::new().context("Failed to create shared Tokio runtime")?;
        let prompts = Prompts::load();

        let course = std::env::var("UMM_COURSE").unwrap_or_else(|_| "ITSC 2214".to_string());
        let term = std::env::var("UMM_TERM").unwrap_or_else(|_| "Fall 2022".to_string());

        Ok(Self {
            supabase,
            postgrest: InitCell::new(),
            runtime: Arc::new(runtime),
            prompts,
            course,
            term,
            openai: OpenAiEnv::from_env(),
        })
    }

    /// Returns the configured PostgREST client if credentials are available.
    pub fn postgrest(&self) -> Option<Postgrest> {
        if let Some(client) = self.postgrest.try_get() {
            return Some(client.clone());
        }

        let creds = self.supabase.as_ref()?;
        let client = Postgrest::new(creds.rest_endpoint.clone())
            .insert_header("apiKey", creds.api_key.clone());
        self.postgrest.set(client);
        Some(self.postgrest.get().clone())
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

    /// Returns the OpenAI configuration, if all required environment variables
    /// are present.
    pub fn openai(&self) -> Option<&OpenAiEnv> {
        self.openai.as_ref()
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

/// Returns the configured PostgREST client, if Supabase has been configured.
pub fn postgrest_client() -> Option<Postgrest> {
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

/// Returns the configured OpenAI environment, if set.
pub fn openai_config() -> Option<&'static OpenAiEnv> {
    get().openai()
}
