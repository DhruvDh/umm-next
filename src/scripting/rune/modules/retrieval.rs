use rune::{Any, ContextError, Module};

use crate::retrieval::HeuristicConfig as InnerHeuristicConfig;

/// Wrapper around retrieval `HeuristicConfig` for Rune scripts.
#[derive(Any, Clone, Copy)]
#[rune(item = ::umm::retrieval)]
pub struct HeuristicConfig {
    /// Wrapped Rust heuristic configuration.
    inner: InnerHeuristicConfig,
}

impl HeuristicConfig {
    #[rune::function(path = HeuristicConfig::default)]
    /// Start from the host default heuristic configuration.
    pub fn default() -> Self {
        Self {
            inner: crate::config::heuristic_defaults(),
        }
    }

    /// Set snippet start offset.
    pub fn start_offset(mut self, value: usize) -> Self {
        self.inner.set_start_offset(value);
        self
    }

    /// Set number of lines to capture.
    pub fn num_lines(mut self, value: usize) -> Self {
        self.inner.set_num_lines(value);
        self
    }

    /// Set maximum line references.
    pub fn max_line_refs(mut self, value: usize) -> Self {
        self.inner.set_max_line_refs(value);
        self
    }

    /// Set full-file ratio threshold.
    pub fn full_file_ratio(mut self, value: f32) -> Self {
        self.inner.set_full_file_ratio(value);
        self
    }

    /// Apply the configuration globally.
    pub fn apply(self) {
        crate::config::set_heuristic_defaults(self.inner);
    }
}

/// Install the `umm::retrieval` Rune module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("umm", ["retrieval"])?;

    module.ty::<HeuristicConfig>()?;

    module.function_meta(HeuristicConfig::default)?;
    module.associated_function("start_offset", HeuristicConfig::start_offset)?;
    module.associated_function("num_lines", HeuristicConfig::num_lines)?;
    module.associated_function("max_line_refs", HeuristicConfig::max_line_refs)?;
    module.associated_function("full_file_ratio", HeuristicConfig::full_file_ratio)?;
    module.associated_function("apply", HeuristicConfig::apply)?;
    Ok(module)
}
