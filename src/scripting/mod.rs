#![warn(missing_docs)]
#![deny(missing_docs)]

use std::{result::Result as StdResult, sync::Arc};

use ::rune::{
    Context, Diagnostics, FromValue, Source, Sources, Vm, prepare,
    termcolor::{ColorChoice, StandardStream},
};
use anyhow::{Context as AnyhowContext, Result};

pub mod rune;

/// Builds the Rune context with the default standard library.
pub fn build_context() -> Result<Context> {
    let mut context = Context::with_default_modules()
        .context("Failed to create Rune context with default modules")?;

    crate::scripting::rune::install_all_modules(&mut context)
        .context("Failed to install umm Rune modules")?;

    Ok(context)
}

/// Executes the Rune script located at `path`, invoking its top-level
/// `main` function asynchronously.
pub async fn run_file(path: &str) -> Result<()> {
    let mut sources = Sources::new();
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Rune script: {path}"))?;
    let _ = sources.insert(Source::new(path, source)?);

    let context = build_context()?;
    let runtime = Arc::new(context.runtime()?);

    let mut diagnostics = Diagnostics::new();

    let prepared = prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Auto);
        diagnostics
            .emit(&mut writer, &sources)
            .context("Failed to emit Rune diagnostics")?;
    }

    let unit = prepared.context("Failed to compile Rune script")?;

    let mut vm = Vm::new(runtime, Arc::new(unit));

    let mut exec = vm
        .execute(["main"], ())
        .context("Failed to execute `main` in Rune script")?;

    // `async_complete` returns a `VmResult<Value>`; convert it to a plain `Value`
    // so host-side error reporting stays in `anyhow`.
    let value = exec
        .async_complete()
        .await
        .into_result()
        .context("Rune script failed during async execution")?;

    let outcome: StdResult<(), ::rune::support::Error> =
        <StdResult<(), ::rune::support::Error> as FromValue>::from_value(value)
            .context("Rune script returned a value that could not be decoded")?;

    outcome.map_err(|e| anyhow::anyhow!("{e}"))
}
