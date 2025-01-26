# Rune: Compiling and Running

Below is a guide on **compiling** and **running** Rune scripts from within a Rust application while taking advantage of the various **external types** and **external functions** you define. It demonstrates a typical “embedding” workflow in Rust:

•	**Set up a** Context (the global set of types and functions visible to Rune).

•	**Install your custom module** that defines external types/functions.

•	**Load or create your script**.

•	**Compile** it into a Rune Unit.

•	**Create a** Vm from the Unit and Context.

•	**Call** Rune functions with custom Rust arguments.

We will assume you have already defined external types and external methods as shown in previous guides. Below, we construct a minimal but complete example that shows how to embed Rune scripts using all those definitions.

**1. Example: External Types, External Methods, and a Rune Script**

Imagine we have these external types and methods in a my_module Rust module:

```
// my_module.rs

use rune::{Any, ContextError, Module};
use rune::runtime::{Protocol, VmError, VmResult};

#[derive(Debug, Any)]
pub struct External {
    pub value: i64,
}

impl External {
    // Normal method that increments `value` by the specified amount.
    fn inc_by(&mut self, amt: i64) -> VmResult<i64> {
        let new_val = self
            .value
            .checked_add(amt)
            .ok_or_else(|| VmError::panic("Overflow in inc_by"))?;
        self.value = new_val;
        VmResult::Ok(self.value)
    }
}

pub fn create_module() -> Result<Module, ContextError> {
    let mut module = Module::new();

    // Register the External type
    module.ty::<External>()?;

    // Add get/set for External.value
    module.field_function(Protocol::GET, "value", |s: &External| {
        VmResult::Ok(s.value)
    })?;
    module.field_function(Protocol::SET, "value", |s: &mut External, val: i64| {
        s.value = val;
        VmResult::Ok(())
    })?;

    // Overload += on `external.value`.
    module.field_function(
        Protocol::ADD_ASSIGN,
        "value",
        |this: &mut External, rhs: i64| {
            let new_val = this.value.checked_add(rhs)
                .ok_or_else(|| VmError::panic("Overflow in += operator"))?;
            this.value = new_val;
            VmResult::Ok(())
        },
    )?;

    // Associated method: external.inc_by(...)
    module.associated_function("inc_by", External::inc_by)?;

    Ok(module)
}
```

We’ll embed this module into a Rune scripting environment, compile a script that uses it, and then call a function.

**2. Setting up a Rune Context and Installing the Module**

Your main Rust entry point might look like this:

```
// main.rs

use std::sync::Arc;
use anyhow::Result;
use rune::{
    Any, Context, ContextError, Diagnostics, Module, Options,
    Source, Sources, Vm, prepare,
};
use rune::termcolor::{ColorChoice, StandardStream};
use crate::my_module::create_module;

fn main() -> Result<()> {
    // 1) Create a `Context` that includes all default modules (like `std`).
    let mut context = Context::with_default_modules()?;

    // 2) Install our custom module that defines `External`
    let module = create_module()?;
    context.install(&module)?;

    // 3) (Optional) we can now compile Rune scripts that use `External`.
    // For example, we can compile from an inline string or load from a file.

    // ...
    Ok(())
}
```

**2.1 Handling Errors**

We are using anyhow::Result for convenience. For more specialized error handling, you can use the built-in rune::Result.

**3. Creating or Loading Rune Scripts**

There are two main ways to feed your script source into Rune:

1.	**Inline script**: A string literal in your Rust code.

2.	**External file**: Load from Path or File.

**Inline example**:

```
let script = r#"
    pub fn main(external) {
        println(`initial value: ${external.value}`);
        external.value += 5;
        println(`after += 5 => ${external.value}`);

        let inc = external.inc_by(42);
        match inc {
            Ok(v) => println(`inc_by(42) => ${v}`),
            Err(e) => println(`Error => ${e}`),
        }
    }
"#;
```

**File-based example**:

```
// load from a file named "script.rn"
use std::fs;

let script = fs::read_to_string("script.rn")?;
```

**4. Creating Sources and Compiling**

Rune uses a two-step compile process:

1.	Create a Sources list with your script(s).

2.	prepare them with the context and produce a Unit.

```
use rune::{Sources, Source, Diagnostics, Options, prepare};

// ...
let mut sources = Sources::new();
sources.insert(Source::new("my_script", script)?);

// If you read from a file, use the file path as label for debug info
// sources.insert(Source::new("script.rn", fs::read_to_string("script.rn")?)?);

let mut diagnostics = Diagnostics::new();
let options = Options::default(); // e.g. optimizations, etc.

let build_res = prepare(&mut sources)
    .with_context(&context)
    .with_diagnostics(&mut diagnostics)
    .build();

// If there were any parse/compile warnings or errors, print them:
if !diagnostics.is_empty() {
    let mut writer = StandardStream::stderr(ColorChoice::Auto);
    diagnostics.emit(&mut writer, &sources)?;
}

let unit = build_res?; // if compilation fails, we bail
```

unit is the compiled bytecode “executable.”

**5. Running the Script (Vm Creation and Calls)**

With a compiled Unit and a Context, you can create a Vm.

You can call any public function (like main) with arguments, including your custom external types:

```
use std::sync::Arc;
use rune::{Vm};
use rune::runtime::Protocol;
use crate::my_module::External;

// ...
let runtime = Arc::new(context.runtime()?);
let mut vm = Vm::new(runtime.clone(), Arc::new(unit));

// Construct an External instance in Rust
let ext = External { value: 100 };

// Call "main" with that External instance
let result = vm.call(&["main"], (ext,))?;
```

Rune will run your script, passing ext as the argument to the main(external) function. Inside the script, it can do external.value, external.inc_by(42), etc.

If main returns a value, vm.call(...) returns that value as a Value. You can decode it to a Rust type if it matches (e.g., decode to i64 or String).

**6. Handling External Method Results**

If inc_by or add_checked returns a Result<T, E>, in Rune scripts that is seen as Ok(...) or Err(...). Any Err(...) that is *unhandled* by the script triggers a runtime exception. If you do not want your embedded usage to panic, you can do:

```
let result = vm.call(&["some_fallible_fn"], (some_args,));
match result {
    Ok(value) => { /* it completed successfully */ }
    Err(vm_err) => {
        eprintln!("Rune VM error: {}", vm_err);
        // handle the error or convert it to your own
    }
}
```

**7. Passing Additional Rust Values into Scripts**

You can pass arbitrary arguments to script functions as long as they implement Any or are “primitive” types recognized by Rune. E.g.:

```
fn main() -> Result<()> {
    // ...
    let mut vm = Vm::new(runtime.clone(), Arc::new(unit));

    // Pass multiple arguments to `some_func(arg1, arg2, ...)`
    // as a tuple: (arg1, arg2, arg3, ...)
    let user_str = "Hello from Rust";
    let ext_obj = External { value: 123 };

    let out = vm.call(&["some_func"], (user_str, ext_obj, 42i64))?;
    println!("Received back: {:?}", out);
    // Possibly out.cast_into::<i64>() if you expect an integer, etc.

    Ok(())
}
```

Rune automatically tries to convert recognized types (like strings, i64, bool) or uses reflection for types implementing Any.

**8. Hot-Reloading or Multiple Scripts**

If you want to compile many scripts (or do “hot reloading”), you can share the same Context and compile each script into its own Unit. Creating a Vm from each Unit is cheap. You can store them in a map keyed by script name and run them as needed.

**9. Full Example (Consolidated)**

Below is a final consolidated example that should demonstrate a typical compile-and-run workflow in Rust with Rune.

```
//////////////////////////////////////////////////////////
// main.rs
//////////////////////////////////////////////////////////

mod my_module; // contains `create_module()` and `External` definitions
use my_module::External;

use std::sync::Arc;
use std::fs;
use anyhow::Result;
use rune::{
    // Core engine
    Context, ContextError, Diagnostics, Options, Sources, Source,
    // VM
    Vm,
    // For color diagnostics
    termcolor::{ColorChoice, StandardStream},
    // compile
    prepare
};

fn main() -> Result<()> {
    // 1) Build a context with default modules from Rune
    let mut context = Context::with_default_modules()?;

    // 2) Install our external module containing `External`
    let module = my_module::create_module()?;
    context.install(&module)?;

    // 3) Load or define a script. We'll show a file-based approach.
    let script_path = "script.rn";
    let script = fs::read_to_string(script_path)?;

    // 4) Setup sources
    let mut sources = Sources::new();
    sources.insert(Source::new(script_path, script)?)?;

    // 5) Compile the script to a `Unit`
    let mut diagnostics = Diagnostics::new();
    let options = Options::default();

    let build_res = prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Auto);
        diagnostics.emit(&mut writer, &sources)?;
    }

    let unit = build_res?; // fails if invalid script

    // 6) Build the VM
    let runtime = Arc::new(context.runtime()?);
    let mut vm = Vm::new(runtime, Arc::new(unit));

    // 7) Create our external instance
    let external = External { value: 100 };

    // 8) Call the script's entrypoint function
    //    Suppose script has a `pub fn main(external)`.
    let ret = vm.call(&["main"], (external,))?;

    // If main returns a value, we can do something with `ret`.
    println!("Script returned: {:?}", ret);

    Ok(())
}
```

script.rn might look like:

```
pub fn main(ext) {
    println(`Hello from Rune. ext.value = ${ext.value}`);

    ext.value += 5;
    println(`After += 5 => ${ext.value}`);

    match ext.inc_by(42) {
        Ok(v) => println(`inc_by(42) => ${v}`),
        Err(e) => println(`Overflow or error => ${e}`),
    }
}
```

Compile and run:

```
# typical cargo usage
$ cargo run
Hello from Rune. ext.value = 100
After += 5 => 105
inc_by(42) => 147
Script returned: ()
```

**10. Wrap-Up Tips**

1.	**Error Printing**

Always check Diagnostics for compile-time warnings/errors and handle runtime errors from vm.call.

2.	**Multiple External Modules**

You can define multiple modules (e.g. moduleA, moduleB) and context.install(...) all of them before building the Context::runtime(). Your scripts can then import the items from each.

3.	**Subsequent Runs**

Once compiled into a Unit, running the same script multiple times is cheap. Just re-invoke vm.call(...). If you want different arguments, you can do so with a fresh VM or by resetting the call frame.

4.	**Performance**

•	For CPU-heavy tasks, consider whether the overhead of repeated external calls is acceptable.

•	If you want to run the same main function repeatedly, you can store a function pointer Function from vm.function(["main"]) and call that repeatedly with new arguments.

5.	**Async**

If your external methods are async or your script uses async/.await, you can run them with .execute() or spawn them in an async runtime. In that case, you’d store a future from the VM and .await it in your Rust code.

With this, you have a solid blueprint for **compiling** and **running** Rune scripts directly from Rust, using **all the external types and methods** you define. It provides the best of both worlds: your Rust logic, safety, and performance, plus the flexibility of dynamic scripting in Rune.