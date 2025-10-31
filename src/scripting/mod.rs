use std::{result::Result as StdResult, sync::Arc};

use anyhow::{Context as AnyhowContext, Result, bail};
use rune::{
    Context, Diagnostics, FromValue, Source, Sources, Vm,
    termcolor::{ColorChoice, StandardStream},
};

/// Builds the Rune context with the default standard library.
pub fn build_context() -> Result<Context> {
    let context = Context::with_default_modules()
        .context("Failed to create Rune context with default modules")?;

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

    let prepared = rune::prepare(&mut sources)
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

    let value = vm
        .async_call(["main"], ())
        .await
        .context("Failed to execute `main` in Rune script")?;

    let outcome: StdResult<(), String> = <StdResult<(), String> as FromValue>::from_value(value)
        .context("Rune script returned a value that could not be decoded")?;

    match outcome {
        Ok(()) => Ok(()),
        Err(message) => {
            bail!("{message}");
        }
    }
}
