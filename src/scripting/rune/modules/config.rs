use rune::{ContextError, Module};

/// Enable or disable active retrieval globally.
pub fn set_active_retrieval(enabled: bool) {
    crate::config::set_active_retrieval(enabled);
}

/// Check whether active retrieval is enabled.
pub fn active_retrieval_enabled() -> bool {
    crate::config::active_retrieval_enabled()
}

/// Install the `umm::config` Rune module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("umm", ["config"])?;

    module
        .function("set_active_retrieval", set_active_retrieval)
        .build()?;
    module
        .function("active_retrieval_enabled", active_retrieval_enabled)
        .build()?;
    Ok(module)
}
