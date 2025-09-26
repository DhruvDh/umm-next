use anyhow::{Context as AnyhowContext, Result};
use async_openai::types::ChatCompletionRequestMessage;

use crate::java::grade::LineRef;

/// Mode describing how we want to assemble context snippets for the LLM.
#[derive(Debug, Clone)]
pub enum RetrievalMode {
    /// Send the entire codebase (language-specific formatting).
    FullCodebase,
    /// Send heuristically selected snippets around diagnostics.
    Heuristic(HeuristicConfig),
    /// Invoke the active-retrieval service and optionally fall back.
    Active {
        /// Combined stdout/stderr from the grader run.
        grader_output: String,
        /// Mode to use when the retrieval service fails.
        fallback:      Box<RetrievalMode>,
    },
}

impl RetrievalMode {
    /// Returns the fallback mode for active retrieval failures.
    pub fn fallback_or(self, default: RetrievalMode) -> RetrievalMode {
        match self {
            RetrievalMode::Active { fallback, .. } => *fallback,
            _ => default,
        }
    }
}

/// Parameters controlling heuristic snippet selection.
#[derive(Debug, Clone, Copy)]
pub struct HeuristicConfig {
    /// Number of lines to include before the diagnostic line.
    pub start_offset:  usize,
    /// Number of lines to include after the diagnostic line.
    pub num_lines:     usize,
    /// Maximum number of merged line references to include.
    pub max_line_refs: usize,
}

impl Default for HeuristicConfig {
    fn default() -> Self {
        Self {
            start_offset:  3,
            num_lines:     6,
            max_line_refs: 6,
        }
    }
}

impl HeuristicConfig {
    /// Returns the number of lines included before the diagnostic line.
    pub fn start_offset(&self) -> usize {
        self.start_offset
    }

    /// Updates the number of lines included before the diagnostic line.
    pub fn set_start_offset(&mut self, value: usize) {
        self.start_offset = value;
    }

    /// Returns the number of lines included after the diagnostic line.
    pub fn num_lines(&self) -> usize {
        self.num_lines
    }

    /// Updates the number of lines included after the diagnostic line.
    pub fn set_num_lines(&mut self, value: usize) {
        self.num_lines = value;
    }

    /// Returns the maximum number of merged line references included.
    pub fn max_line_refs(&self) -> usize {
        self.max_line_refs
    }

    /// Updates the maximum number of merged line references included.
    pub fn set_max_line_refs(&mut self, value: usize) {
        self.max_line_refs = value;
    }
}

/// Trait implemented by language-specific project types to format retrieval
/// context.
pub trait RetrievalFormatter {
    /// Returns a short language identifier ("java", "python", etc.).
    fn language(&self) -> &'static str;

    /// Serializes the entire codebase using language-specific markup.
    fn full_codebase(&self) -> Result<Vec<ChatCompletionRequestMessage>>;

    /// Returns the default heuristic configuration (may be overridden per
    /// language).
    fn heuristic_defaults(&self) -> HeuristicConfig {
        crate::config::heuristic_defaults()
    }

    /// Generates snippet-based context using the diagnostic line references.
    fn heuristic_context(
        &self,
        line_refs: Vec<LineRef>,
        cfg: HeuristicConfig,
    ) -> Result<ChatCompletionRequestMessage>;

    /// Resolves active retrieval using the provided grader output.
    fn active_retrieval(&self, grader_output: String) -> Result<ChatCompletionRequestMessage>;
}

/// Assembles retrieval messages based on the requested mode.
pub fn build_messages<F: RetrievalFormatter>(
    formatter: &F,
    mode: RetrievalMode,
    line_refs: Vec<LineRef>,
) -> Result<Vec<ChatCompletionRequestMessage>> {
    match mode {
        RetrievalMode::FullCodebase => formatter.full_codebase(),
        RetrievalMode::Heuristic(cfg) => {
            let message = formatter.heuristic_context(line_refs, cfg)?;
            Ok(vec![message])
        }
        RetrievalMode::Active {
            grader_output,
            fallback,
        } => match formatter.active_retrieval(grader_output) {
            Ok(message) => Ok(vec![message]),
            Err(err) => {
                eprintln!("Active retrieval failed: {err:?}. Falling back to heuristic context.");
                build_messages(formatter, *fallback, line_refs)
            }
        },
    }
}

/// Convenience helper that returns the first message produced by the retrieval
/// pipeline or errors if none are generated.
pub fn build_single_message<F, T>(
    formatter: &F,
    mode: RetrievalMode,
    diags: Vec<T>,
) -> Result<ChatCompletionRequestMessage>
where
    F: RetrievalFormatter,
    T: Into<LineRef>,
{
    let line_refs = diags.into_iter().map(Into::into).collect();
    let messages = build_messages(formatter, mode, line_refs)?;
    messages
        .into_iter()
        .next()
        .with_context(|| format!("{} retrieval produced no messages", formatter.language()))
}

/// Builds a single context message using the language defaults and the global
/// config.
pub fn build_context_message<F, T>(
    formatter: &F,
    grader_output: Option<String>,
    diags: Vec<T>,
) -> Result<ChatCompletionRequestMessage>
where
    F: RetrievalFormatter,
    T: Into<LineRef>,
{
    let cfg = formatter.heuristic_defaults();
    let mode = match grader_output {
        Some(output) if crate::config::active_retrieval_enabled() => RetrievalMode::Active {
            grader_output: output,
            fallback:      Box::new(RetrievalMode::Heuristic(cfg)),
        },
        Some(_) | None => RetrievalMode::Heuristic(cfg),
    };

    build_single_message(formatter, mode, diags)
}
