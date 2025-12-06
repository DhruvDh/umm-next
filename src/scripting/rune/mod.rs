//! Rune embedding helpers and module installation.

use rune::{Context, compile::ContextError};

/// Collection of `umm` Rune submodules (java, gradescope, config, retrieval).
pub mod modules;

/// Install all `umm` Rune modules into the provided context.
pub fn install_all_modules(context: &mut Context) -> Result<(), ContextError> {
    context.install(modules::java::module()?)?;
    context.install(modules::python::module()?)?;
    context.install(modules::gradescope::module()?)?;
    context.install(modules::config::module()?)?;
    context.install(modules::retrieval::module()?)?;
    Ok(())
}
