#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use async_openai::types::chat::ReasoningEffort;
use postgrest::Postgrest;
use reqwest::Client;
use state::InitCell;

use crate::{
    java::config::{JavaConfig, JavaPrompts},
    python::config::PythonPrompts,
    retrieval::HeuristicConfig,
};

/// Prompt truncation length for generated feedback payloads.
pub const PROMPT_TRUNCATE: usize = 60_000;

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
    pub endpoint:     String,
    /// API key used to authenticate OpenAI requests.
    pub api_key:      String,
    /// Default model identifier for chat completions.
    pub model:        String,
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
        let endpoint = std::env::var("OPENAI_ENDPOINT").ok()?.trim().to_owned();
        let api_key = std::env::var("OPENAI_API_KEY_SLO").ok()?.trim().to_owned();
        let model = std::env::var("OPENAI_MODEL").ok()?.trim().to_owned();

        if endpoint.is_empty() || api_key.is_empty() || model.is_empty() {
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
            endpoint,
            api_key,
            model,
            temperature,
            top_p,
            reasoning_effort,
        })
    }

    /// Returns the API base URL used for OpenAI requests.
    pub fn api_base(&self) -> &str {
        &self.endpoint
    }

    /// Returns the API key used for OpenAI requests.
    pub fn get_api_key(&self) -> &str {
        &self.api_key
    }

    /// Returns the default model identifier.
    pub fn get_model(&self) -> &str {
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
            ReasoningEffort::None => ReasoningEffort::None,
        };

        Self {
            endpoint: self.endpoint.clone(),
            api_key: self.api_key.clone(),
            model: self.model.clone(),
            temperature: self.temperature,
            top_p: self.top_p,
            reasoning_effort,
        }
    }
}

/// Runtime and prompt configuration shared across the crate.
pub struct ConfigState {
    /// Supabase credentials, if configured.
    supabase:            Option<SupabaseEnv>,
    /// Lazily constructed Supabase PostgREST client.
    postgrest:           InitCell<Postgrest>,
    /// Shared reqwest HTTP client reused across network helpers.
    http_client:         Client,
    /// Java-specific configuration bundle.
    java_config:         JavaConfig,
    /// Course identifier exposed to Supabase-backed endpoints.
    course:              String,
    /// Academic term identifier exposed to Supabase-backed endpoints.
    term:                String,
    /// Cached OpenAI configuration, if available.
    openai:              Option<OpenAiEnv>,
    /// Flag indicating whether active retrieval is enabled.
    active_retrieval:    AtomicBool,
    /// Default heuristic window for snippet-based retrieval.
    retrieval_heuristic: Mutex<HeuristicConfig>,
    /// Endpoint used for active-retrieval service calls.
    retrieval_endpoint:  String,
}

impl ConfigState {
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

        let http_client = Client::builder()
            // Avoid macOS dynamic store lookups that fail in sandboxed environments.
            .no_proxy()
            .build()
            .context("Failed to construct shared HTTP client")?;
        let prompts = JavaPrompts::load();
        let java_config = JavaConfig::new(
            prompts,
            HeuristicConfig::default(),
            read_timeout_secs("UMM_JAVAC_TIMEOUT_SECS", 30),
            read_timeout_secs("UMM_JAVA_TIMEOUT_SECS", 60),
        );

        let course = std::env::var("UMM_COURSE").unwrap_or_else(|_| "ITSC 2214".to_string());
        let term = std::env::var("UMM_TERM").unwrap_or_else(|_| "Fall 2022".to_string());

        let retrieval_endpoint = std::env::var("UMM_RETRIEVAL_ENDPOINT")
            .map(|value| value.trim().to_owned())
            .unwrap_or_else(|_| "https://umm-feedback-openai-func.deno.dev/".to_string());

        let retrieval_heuristic = Mutex::new(java_config.retrieval_defaults());

        Ok(Self {
            supabase,
            postgrest: InitCell::new(),
            http_client,
            java_config,
            course,
            term,
            openai: OpenAiEnv::from_env(),
            active_retrieval: AtomicBool::new(false),
            retrieval_heuristic,
            retrieval_endpoint,
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

    /// Returns a clone of the shared reqwest HTTP client.
    pub fn http_client(&self) -> Client {
        self.http_client.clone()
    }

    /// Returns the configured retrieval endpoint.
    pub fn retrieval_endpoint(&self) -> &str {
        &self.retrieval_endpoint
    }

    /// Returns the course identifier.
    pub fn course(&self) -> &str {
        &self.course
    }

    /// Returns the academic term identifier.
    pub fn term(&self) -> &str {
        &self.term
    }

    /// Returns the Java prompt bundle.
    pub fn java_prompts(&self) -> &JavaPrompts {
        self.java_config.prompts()
    }

    /// Returns the OpenAI configuration, if all required environment variables
    /// are present.
    pub fn openai(&self) -> Option<&OpenAiEnv> {
        self.openai.as_ref()
    }

    /// Updates the active retrieval toggle.
    pub fn set_active_retrieval(&self, enabled: bool) {
        self.active_retrieval.store(enabled, Ordering::Relaxed);
    }

    /// Returns whether active retrieval is enabled.
    pub fn active_retrieval_enabled(&self) -> bool {
        self.active_retrieval.load(Ordering::Relaxed)
    }

    /// Returns the default heuristic configuration for snippet retrieval.
    pub fn heuristic_defaults(&self) -> HeuristicConfig {
        *self
            .retrieval_heuristic
            .lock()
            .map_err(|e| anyhow!("retrieval heuristic lock poisoned: {e}"))
            .expect("retrieval heuristic lock poisoned")
    }

    /// Updates the default heuristic configuration for snippet retrieval.
    pub fn set_heuristic_defaults(&self, cfg: HeuristicConfig) {
        *self
            .retrieval_heuristic
            .lock()
            .map_err(|e| anyhow!("retrieval heuristic lock poisoned: {e}"))
            .expect("retrieval heuristic lock poisoned") = cfg;
    }

    /// Returns the configured start offset for heuristic retrieval.
    pub fn heuristic_start_offset(&self) -> usize {
        self.retrieval_heuristic
            .lock()
            .map_err(|e| anyhow!("retrieval heuristic lock poisoned: {e}"))
            .expect("retrieval heuristic lock poisoned")
            .start_offset()
    }

    /// Sets the configured start offset for heuristic retrieval.
    pub fn set_heuristic_start_offset(&self, value: usize) {
        self.retrieval_heuristic
            .lock()
            .map_err(|e| anyhow!("retrieval heuristic lock poisoned: {e}"))
            .expect("retrieval heuristic lock poisoned")
            .set_start_offset(value);
    }

    /// Returns the configured number of lines captured after diagnostics.
    pub fn heuristic_num_lines(&self) -> usize {
        self.retrieval_heuristic
            .lock()
            .map_err(|e| anyhow!("retrieval heuristic lock poisoned: {e}"))
            .expect("retrieval heuristic lock poisoned")
            .num_lines()
    }

    /// Sets the configured number of lines captured after diagnostics.
    pub fn set_heuristic_num_lines(&self, value: usize) {
        self.retrieval_heuristic
            .lock()
            .map_err(|e| anyhow!("retrieval heuristic lock poisoned: {e}"))
            .expect("retrieval heuristic lock poisoned")
            .set_num_lines(value);
    }

    /// Returns the configured maximum number of merged diagnostic ranges.
    pub fn heuristic_max_line_refs(&self) -> usize {
        self.retrieval_heuristic
            .lock()
            .map_err(|e| anyhow!("retrieval heuristic lock poisoned: {e}"))
            .expect("retrieval heuristic lock poisoned")
            .max_line_refs()
    }

    /// Sets the configured maximum number of merged diagnostic ranges.
    pub fn set_heuristic_max_line_refs(&self, value: usize) {
        self.retrieval_heuristic
            .lock()
            .map_err(|e| anyhow!("retrieval heuristic lock poisoned: {e}"))
            .expect("retrieval heuristic lock poisoned")
            .set_max_line_refs(value);
    }

    /// Returns the ratio used to trigger full-file snippet rendering.
    pub fn heuristic_full_file_ratio(&self) -> f32 {
        self.retrieval_heuristic
            .lock()
            .map_err(|e| anyhow!("retrieval heuristic lock poisoned: {e}"))
            .expect("retrieval heuristic lock poisoned")
            .full_file_ratio()
    }

    /// Sets the ratio used to trigger full-file snippet rendering.
    pub fn set_heuristic_full_file_ratio(&self, value: f32) {
        self.retrieval_heuristic
            .lock()
            .map_err(|e| anyhow!("retrieval heuristic lock poisoned: {e}"))
            .expect("retrieval heuristic lock poisoned")
            .set_full_file_ratio(value);
    }

    /// Returns the configured javac timeout duration.
    pub fn javac_timeout(&self) -> Duration {
        self.java_config.javac_timeout()
    }

    /// Returns the configured java/JUnit timeout duration.
    pub fn java_timeout(&self) -> Duration {
        self.java_config.java_timeout()
    }

    /// Returns the Java configuration bundle.
    pub fn java_config(&self) -> &JavaConfig {
        &self.java_config
    }
}

/// Borrowed view of the Java prompt catalog that keeps the underlying
/// configuration alive.
pub struct JavaPromptsRef(ConfigHandle);

impl std::ops::Deref for JavaPromptsRef {
    type Target = JavaPrompts;

    fn deref(&self) -> &Self::Target {
        self.0.java_prompts()
    }
}

/// Borrowed view of the Java configuration bundle.
pub struct JavaConfigRef(ConfigHandle);

impl std::ops::Deref for JavaConfigRef {
    type Target = JavaConfig;

    fn deref(&self) -> &Self::Target {
        self.0.java_config()
    }
}

/// Borrowed view of the OpenAI configuration tied to the global config.
pub struct OpenAiRef(ConfigHandle);

impl std::ops::Deref for OpenAiRef {
    type Target = OpenAiEnv;

    fn deref(&self) -> &Self::Target {
        // SAFETY: OpenAiRef is only constructed when openai is Some (see
        // `openai_config`). If that invariant is broken, return a hard error
        // instead of panicking to aid debugging.
        self.0
            .openai
            .as_ref()
            .unwrap_or_else(|| unreachable!("OpenAiRef constructed without config"))
    }
}

/// Shared configuration handle used throughout the crate.
#[derive(Clone)]
pub struct ConfigHandle(Arc<ConfigState>);

impl std::ops::Deref for ConfigHandle {
    type Target = ConfigState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Global storage for the lazily constructed configuration state.
static CONFIG_SLOT: OnceLock<Mutex<Option<Arc<ConfigState>>>> = OnceLock::new();

/// Returns the mutex guarding the global configuration slot.
fn slot() -> &'static Mutex<Option<Arc<ConfigState>>> {
    CONFIG_SLOT.get_or_init(|| Mutex::new(None))
}

/// Builds a fresh configuration instance and wraps it in an `Arc`.
fn build_default() -> Result<Arc<ConfigState>> {
    ConfigState::new().map(Arc::new)
}

/// Ensure the global configuration has been initialized and return a handle.
pub fn ensure_initialized() -> Result<ConfigHandle> {
    let slot = slot();
    let mut guard = slot
        .lock()
        .map_err(|_| anyhow!("configuration mutex poisoned; restart recommended"))?;

    if let Some(cfg) = guard.as_ref() {
        return Ok(ConfigHandle(Arc::clone(cfg)));
    }

    let cfg = build_default()?;
    *guard = Some(Arc::clone(&cfg));
    Ok(ConfigHandle(cfg))
}

/// Returns the active configuration, initializing it on demand.
pub fn get() -> ConfigHandle {
    match ensure_initialized() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to initialize configuration: {e:?}");
            std::process::exit(1);
        }
    }
}

/// Returns the configured PostgREST client, if Supabase has been configured.
pub fn postgrest_client() -> Option<Postgrest> {
    get().postgrest()
}

/// Returns a clone of the shared reqwest HTTP client.
pub fn http_client() -> Client {
    get().http_client()
}

/// Returns the configured retrieval endpoint.
pub fn retrieval_endpoint() -> String {
    get().retrieval_endpoint().to_string()
}

/// Returns the configured course identifier.
pub fn course() -> String {
    get().course.clone()
}

/// Returns the configured term identifier.
pub fn term() -> String {
    get().term.clone()
}

/// Returns the configured Java prompts.
pub fn java_prompts() -> JavaPromptsRef {
    JavaPromptsRef(get())
}

/// Returns the configured Java configuration bundle.
pub fn java_config() -> JavaConfigRef {
    JavaConfigRef(get())
}

/// Returns the configured OpenAI environment, if set.
pub fn openai_config() -> Option<OpenAiRef> {
    let handle = get();
    if handle.openai.is_some() {
        Some(OpenAiRef(handle))
    } else {
        None
    }
}

/// Returns the default heuristic retrieval configuration.
pub fn heuristic_defaults() -> HeuristicConfig {
    get().heuristic_defaults()
}

/// Overrides the default heuristic retrieval configuration.
pub fn set_heuristic_defaults(cfg: HeuristicConfig) {
    get().set_heuristic_defaults(cfg);
}

/// Returns the configured heuristic start offset.
pub fn heuristic_start_offset() -> usize {
    get().heuristic_start_offset()
}

/// Sets the configured heuristic start offset.
pub fn set_heuristic_start_offset(value: usize) {
    get().set_heuristic_start_offset(value);
}

/// Returns the configured number of lines captured after diagnostics.
pub fn heuristic_num_lines() -> usize {
    get().heuristic_num_lines()
}

/// Sets the configured number of lines captured after diagnostics.
pub fn set_heuristic_num_lines(value: usize) {
    get().set_heuristic_num_lines(value);
}

/// Returns the configured maximum number of merged diagnostic ranges.
pub fn heuristic_max_line_refs() -> usize {
    get().heuristic_max_line_refs()
}

/// Sets the configured maximum number of merged diagnostic ranges.
pub fn set_heuristic_max_line_refs(value: usize) {
    get().set_heuristic_max_line_refs(value);
}

/// Returns the configured full-file snippet ratio.
pub fn heuristic_full_file_ratio() -> f32 {
    get().heuristic_full_file_ratio()
}

/// Sets the configured full-file snippet ratio.
pub fn set_heuristic_full_file_ratio(value: f32) {
    get().set_heuristic_full_file_ratio(value);
}

/// Enables or disables active retrieval for context-building helpers.
pub fn set_active_retrieval(enabled: bool) {
    get().set_active_retrieval(enabled);
}

/// Returns whether active retrieval is currently enabled.
pub fn active_retrieval_enabled() -> bool {
    get().active_retrieval_enabled()
}

/// Returns the configured javac timeout duration.
pub fn javac_timeout() -> Duration {
    get().javac_timeout()
}

/// Returns the configured java/JUnit timeout duration.
pub fn java_timeout() -> Duration {
    get().java_timeout()
}

/// Returns the configured Python timeout duration.
pub fn python_timeout() -> Duration {
    read_timeout_secs("UMM_PYTHON_TIMEOUT_SECS", 60)
}

/// Returns the Python prompts bundle.
pub fn python_prompts() -> PythonPrompts {
    PythonPrompts::default()
}

/// Returns the OpenAI environment configuration, if available.
pub fn openai_env() -> Option<OpenAiEnv> {
    get().openai.clone()
}

/// Parses an environment variable into a `Duration`, falling back to
/// `default_secs` when parsing fails or the variable is missing.
fn read_timeout_secs(env: &str, default_secs: u64) -> Duration {
    std::env::var(env)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(default_secs))
}
