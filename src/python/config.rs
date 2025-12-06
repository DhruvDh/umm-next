#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Python-specific configuration helpers.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::retrieval::HeuristicConfig;

/// Default timeout for Python execution in seconds.
const DEFAULT_PYTHON_TIMEOUT_SECS: u64 = 60;

/// Prompt templates for Python grading and feedback generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonPrompts {
    /// System message for code review.
    system_message:            String,
    /// Context for retrieval-based feedback.
    retrieval_context_message: String,
    /// Prompt for analyzing input requirements.
    input_analysis_prompt:     String,
    /// Code review template.
    code_review_template:      String,
}

impl Default for PythonPrompts {
    fn default() -> Self {
        Self {
            system_message:            include_str!("prompts/system.md").to_string(),
            retrieval_context_message: include_str!("prompts/retrieval_context.md").to_string(),
            input_analysis_prompt:     include_str!("prompts/input_analysis.md").to_string(),
            code_review_template:      include_str!("prompts/code_review.md").to_string(),
        }
    }
}

impl PythonPrompts {
    /// Returns the system message prompt.
    pub fn system_message(&self) -> &str {
        &self.system_message
    }

    /// Returns the retrieval context message.
    pub fn retrieval_context_message(&self) -> &str {
        &self.retrieval_context_message
    }

    /// Returns the input analysis prompt.
    pub fn input_analysis_prompt(&self) -> &str {
        &self.input_analysis_prompt
    }

    /// Returns the code review template.
    pub fn code_review_template(&self) -> &str {
        &self.code_review_template
    }
}

/// Python-specific configuration bundle.
#[derive(Debug, Clone)]
pub struct PythonConfig {
    /// Prompt templates for grading.
    prompts:          PythonPrompts,
    /// Heuristic configuration for retrieval.
    heuristic_config: HeuristicConfig,
    /// Timeout for Python execution.
    python_timeout:   Duration,
    /// Timeout for linting operations.
    lint_timeout:     Duration,
    /// Timeout for test execution.
    test_timeout:     Duration,
}

impl Default for PythonConfig {
    fn default() -> Self {
        Self {
            prompts:          PythonPrompts::default(),
            heuristic_config: HeuristicConfig::default(),
            python_timeout:   Duration::from_secs(DEFAULT_PYTHON_TIMEOUT_SECS),
            lint_timeout:     Duration::from_secs(30),
            test_timeout:     Duration::from_secs(120),
        }
    }
}

impl PythonConfig {
    /// Returns the prompt templates.
    pub fn prompts(&self) -> &PythonPrompts {
        &self.prompts
    }

    /// Returns the heuristic configuration.
    pub fn heuristic_config(&self) -> &HeuristicConfig {
        &self.heuristic_config
    }

    /// Returns the Python execution timeout.
    pub fn python_timeout(&self) -> Duration {
        self.python_timeout
    }

    /// Returns the lint timeout.
    pub fn lint_timeout(&self) -> Duration {
        self.lint_timeout
    }

    /// Returns the test timeout.
    pub fn test_timeout(&self) -> Duration {
        self.test_timeout
    }

    /// Returns a new config with a custom Python timeout.
    pub fn with_python_timeout(mut self, timeout: Duration) -> Self {
        self.python_timeout = timeout;
        self
    }

    /// Returns a new config with a custom test timeout.
    pub fn with_test_timeout(mut self, timeout: Duration) -> Self {
        self.test_timeout = timeout;
        self
    }
}
