use std::time::Duration;

use crate::retrieval::HeuristicConfig;

/// Prompt assets used by the Java graders and retrieval pipeline.
#[derive(Clone)]
pub struct JavaPrompts {
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

impl JavaPrompts {
    /// Load prompt templates embedded in the binary.
    pub fn load() -> Self {
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

    /// Returns the full system prompt.
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

/// Java-specific configuration derived from environment and embedded assets.
#[derive(Clone)]
pub struct JavaConfig {
    /// Loaded prompt catalog used by graders and retrieval helpers.
    prompts:            JavaPrompts,
    /// Default heuristic window for snippet-based retrieval.
    retrieval_defaults: HeuristicConfig,
    /// Maximum time allowed for javac invocations.
    javac_timeout:      Duration,
    /// Maximum time allowed for java/JUnit invocations.
    java_timeout:       Duration,
}

impl JavaConfig {
    /// Constructs a new Java configuration bundle.
    pub fn new(
        prompts: JavaPrompts,
        retrieval_defaults: HeuristicConfig,
        javac_timeout: Duration,
        java_timeout: Duration,
    ) -> Self {
        Self {
            prompts,
            retrieval_defaults,
            javac_timeout,
            java_timeout,
        }
    }

    /// Returns the prompt catalog.
    pub fn prompts(&self) -> &JavaPrompts {
        &self.prompts
    }

    /// Returns the default heuristic configuration for retrieval.
    pub fn retrieval_defaults(&self) -> HeuristicConfig {
        self.retrieval_defaults
    }

    /// Returns the configured javac timeout.
    pub fn javac_timeout(&self) -> Duration {
        self.javac_timeout
    }

    /// Returns the configured java/JUnit timeout.
    pub fn java_timeout(&self) -> Duration {
        self.java_timeout
    }
}
