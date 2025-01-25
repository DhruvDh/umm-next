//////////////////////////////////////////////////////////
// src/main.rs
//////////////////////////////////////////////////////////
use std::sync::Arc;

use anyhow::Result;
use rune::{
    // Basic Rune items
    Any,
    Context,
    ContextError,
    // For diagnostics
    Diagnostics,
    Module,
    Options,
    // The "public" re-exports for adding script sources:
    Source,
    Sources,
    Vm,
    // For inline compile
    prepare,
    // For hooking into run-time
    runtime::{Protocol, VmError, VmResult},
    // For color in error printing
    termcolor::{ColorChoice, StandardStream},
    // For short-circuit macros
    vm_try,
};

/// A simple external struct we expose to Rune.
#[derive(Debug, Any)]
pub struct External {
    pub value: i64,
}

impl External {
    /// A method that increments `value` by `amount`, returning an error on
    /// overflow.
    fn inc_by(&mut self,
              amount: i64)
              -> VmResult<i64> {
        let new_val = vm_try!(self.value
                                  .checked_add(amount)
                                  .ok_or_else(|| VmError::panic("Overflow in inc_by")));
        self.value = new_val;
        VmResult::Ok(new_val)
    }
}

/// Build a `Module` that exposes `External`.
fn create_module() -> Result<Module, ContextError> {
    let mut module = Module::new();

    // Register the type
    module.ty::<External>()?;

    // Provide field GET for `external.value`.
    module.field_function(Protocol::GET, "value", |s: &External| VmResult::Ok(s.value))?;

    // Provide field SET for `external.value`.
    module.field_function(Protocol::SET, "value", |s: &mut External, val: i64| {
              s.value = val;
              VmResult::Ok(())
          })?;

    // Overload `+=` on `external.value`.
    module.field_function(
                          Protocol::ADD_ASSIGN,
                          "value",
                          |s: &mut External, rhs: i64| {
                              let new_val = vm_try!(s
            .value
            .checked_add(rhs)
            .ok_or_else(|| VmError::panic("Overflow in += operator")));
                              s.value = new_val;
                              VmResult::Ok(())
                          },
    )?;

    // The `inc_by` method: external.inc_by(42).
    module.associated_function("inc_by",
                               External::inc_by as fn(&mut External, i64) -> VmResult<i64>)?;

    Ok(module)
}

fn main() -> Result<()> {
    // 1) Create a context with default modules
    let mut context = Context::with_default_modules()?;

    // 2) Build and install our custom module
    let module = create_module()?;
    context.install(&module)?;

    // 3) Inline Rune script as a &str
    //
    // Because older Rune `println` can only take *one* argument, we manually
    // build strings: "message" + number.to_string() => "message###"
    let script = r#"
        pub fn main(external) {
            let init_msg = "Initial: " + external.value.to_string();
            println(init_msg);

            external.value += 5;
            let after_msg = "After += 5 => " + external.value.to_string();
            println(after_msg);

            match external.inc_by(42) {
                Ok(v) => {
                    let s = "inc_by(42) => " + v.to_string();
                    println(s);
                },
                Err(e) => {
                    let s = "Error: " + e.to_string();
                    println(s);
                }
            }

            match external.inc_by(9_223_372_036_854_775_700) {
                Ok(v) => {
                    let s = "huge inc => " + v.to_string();
                    println(s);
                },
                Err(e) => {
                    let s = "Overflow error: " + e.to_string();
                    println(s);
                }
            }
        }
    "#;

    // 4) Insert the script into a Sources collection
    let source = Source::new("my_script", script)?; // label, content
    let mut sources = Sources::new();
    sources.insert(source)?;

    // 5) Prepare + compile
    let mut diagnostics = Diagnostics::new();
    let options = Options::default();

    let build_res = prepare(&mut sources).with_context(&context)
                                         .with_diagnostics(&mut diagnostics)
                                         .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Auto);
        diagnostics.emit(&mut writer, &sources)?;
    }

    // If compilation failed, bail out
    let unit = build_res?;

    // 6) Build a VM
    let runtime = Arc::new(context.runtime()?);
    let mut vm = Vm::new(runtime, Arc::new(unit));

    // 7) Our external instance
    let external = External { value: 100 };

    println!("=== Rune script output ===");
    // 8) Call the script's `main` function with `external`
    vm.call(["main"], (external,))?;

    Ok(())
}
