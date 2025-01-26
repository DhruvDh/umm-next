# Rune: External Types, Methods, Operator Overloads, and Fallible Functions

This guide covers how to register *external* Rust types in Rune, expose fields and methods, leverage operator overloading, and handle failure cases (Result-returning functions) seamlessly within Rune scripts.

**1. What is an External Type?**

An **external type** is any Rust type *not* defined in Rune code but exposed to Rune via a native module. These types can be:

•	**Opaque** (Rune only sees a “handle”),

•	**Struct-like** (with fields that Rune can read or modify),

•	**Enums** (which can be pattern matched),

•	**Special-purpose** references or wrappers.

Rune determines what is accessible based on the module registration you do in Rust. Without explicit registration, the type is “opaque,” meaning scripts can hold values of it but cannot inspect or mutate them.

**1.1 Declaring an External Type**

1.	Annotate your type with #[derive(Any)].

2.	Add module.ty::<YourType>()? to your Module.

**Example**:

```
use rune::Any;
use rune::Module;
use rune::ContextError;

#[derive(Debug, Any)]
struct External {
    // Possibly annotated fields ...
}

pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new();

    // Make `External` known to Rune
    module.ty::<External>()?;

    // ... additional registrations, if needed ...

    Ok(module)
}
```

By default, External is now recognized by Rune but is opaque. Scripts can store it in a variable or pass it around, but can’t see fields or call methods unless you add those in the module.

**2. Field Access on External Types**

Rune provides an attribute-based system for making struct fields available:

•	#[rune(get)] => Makes the field readable in scripts (external.field).

•	#[rune(set)] => Makes the field writable in scripts (external.field = foo).

These are collectively known as *field functions* in Rune.

**2.1 Basic Getters/Setters**

**Rust side**:

```
#[derive(Debug, Any)]
struct External {
    #[rune(get, set)]
    value: u32,
}
```

**Rune script**:

```
pub fn main(external) {
    println!("{}", external.value); // read
    external.value = external.value + 1; // write
}
```

If you only want read-only access:

```
#[derive(Debug, Any)]
struct External {
    #[rune(get)]
    value: u32,
}
```

**2.2 Operator Overloads**

Rune calls these operator “protocols” or “field functions,” e.g. ADD_ASSIGN for +=. You attach them with attributes:

•	#[rune(add_assign)] => +=

•	#[rune(sub_assign)] => -=

•	#[rune(mul_assign)] => *=

•	#[rune(div_assign)] => /=

•	… etc.

**Example**:

```
#[derive(Debug, Any)]
struct External {
    #[rune(get, set, add_assign)]
    number: i64,
}
```

Now external.number += 10 in Rune calls the generated “field function.”

**2.2.1 Custom Operator Functions**

Instead of auto-generating from the type, you can define your own operator handler:

```
#[derive(Debug, Any)]
struct External {
    #[rune(get)]
    number: i64,
}

impl External {
    // Ties into the `+=` operator for field "number".
    // `#[rune(add_assign = "checked_add_assign")]` means:
    //   the protocol's function is custom_add_assign below.
    #[rune(add_assign = "custom_add_assign")]
    fn custom_add_assign(&mut self, rhs: i64) -> rune::Result<()> {
        self.number = self.number.checked_add(rhs)
            .ok_or_else(|| rune::Error::msg("Overflow!"))?;
        Ok(())
    }
}
```

Hence, external.number += x uses custom_add_assign. If overflow is detected, we return an error which becomes a Rune runtime error.

**3. External Methods (Associated Functions)**

Besides field-based protocols, you can register *arbitrary methods* on your struct. These show up as calls like external.some_method(...) in Rune.

1.	Write a normal Rust method on your type.

2.	Call module.associated_function("method_name", MyType::method_name)? to register it.

**Example**:

```
use rune::{ContextError, Module};
use rune::Any;

#[derive(Debug, Any)]
struct External {
    number: i64,
}

impl External {
    fn increment_by(&mut self, amt: i64) -> i64 {
        self.number += amt;
        self.number
    }
}

pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new();
    module.ty::<External>()?;

    // Register as a Rune instance method:
    module.associated_function("increment_by", External::increment_by)?;

    Ok(module)
}
```

**Rune script**:

```
fn main(external) {
    let new_value = external.increment_by(42);
    println!("New value: {}", new_value);
}
```

**3.1 Opaque if You Skip Registration**

If you skip registering increment_by, Rune scripts have no knowledge it exists. The struct is effectively “opaque” for that method call.

**4. External Enums and Pattern Matching**

Rune can fully match on external enums if:

1.	#[derive(Any)] is used on the enum.

2.	You mark any fields with #[rune(get)] if you want Rune to destructure them.

3.	Use #[rune(constructor)] if you also want to build the variant from Rune.

**Example**:

```
#[derive(Debug, Any)]
enum External {
    #[rune(constructor)]
    First(#[rune(get)] i64, #[rune(get)] i64),
    #[rune(constructor)]
    Second(#[rune(get)] i64),
    Third {
        a: i64,
        b: i64,
        #[rune(get)]
        c: i64,
    },
}
```

**Rune side**:

```
use External;

fn main(value) {
    match value {
        External::First(x, y) => { /* do something */ }
        External::Second(n)   => { /* handle second variant */ }
        External::Third { c } => { /* can bind the c field */ }
    }
}

// Construct a variant if it has a #[rune(constructor)] annotation:
fn maker() {
    let val = External::First(10, 20);
    /* ... */
}
```

**5. Fallible External Functions**

If an external function or method returns Result<T, E> (or the Rune-specific rune::Result type), an Err(...) becomes a Rune runtime error. Scripts can handle them via ? or a match on Ok/Err.

**5.1 Example: Overflowing add**

```
#[derive(Debug, Any)]
struct CheckedInt {
    #[rune(get)]
    value: i64,
}

impl CheckedInt {
    #[rune(add_assign)]
    fn add_checked(&mut self, rhs: i64) -> rune::Result<()> {
        self.value = self.value.checked_add(rhs)
            .ok_or_else(|| rune::Error::msg("Overflow happened!"))?;
        Ok(())
    }
}
```

Then, from Rune:

```
fn main(ci) {
    ci.value = 42;
    ci.value += 9_223_372_036_854_775_807; // will overflow => Error
}
```

If you have a normal function returning a rune::Result<T>, you can handle it in Rune via:

```
fn main(ci) {
    match ci.do_something_fallible() {
        Ok(v) => println!("Success: {}", v),
        Err(e) => println!("Err: {}", e),
    }
}
```

**6. Putting It All Together**

Below is a mini “blueprint” of how you might define a module with:

•	An external struct

•	Field get/set, operator overloading

•	A “fallible” method

Then inside a Rune script, you can exercise them.

```
//////////////////////////////////////////////////////////
// src/main.rs
//////////////////////////////////////////////////////////
use std::sync::Arc;
use anyhow::Result;
use rune::{
    Any, Context, ContextError, Diagnostics, Module, Options,
    Source, Sources, Vm,
    prepare,
    runtime::{Protocol, VmError, VmResult},
    termcolor::{ColorChoice, StandardStream},
};

#[derive(Debug, Any)]
pub struct External {
    pub value: i64,
}

impl External {
    // A method that increments `value` by `amt`, returning an overflow error if needed.
    fn inc_by(&mut self, amt: i64) -> VmResult<i64> {
        let new_val = self
            .value
            .checked_add(amt)
            .ok_or_else(|| VmError::panic("Overflow in inc_by"))?;
        self.value = new_val;
        VmResult::Ok(self.value)
    }
}

fn create_module() -> Result<Module, ContextError> {
    let mut module = Module::new();

    // 1) Register the type
    module.ty::<External>()?;

    // 2) Provide a field GET and SET for `value`
    module.field_function(Protocol::GET, "value", |s: &External| {
        VmResult::Ok(s.value)
    })?;
    module.field_function(Protocol::SET, "value", |s: &mut External, val: i64| {
        s.value = val;
        VmResult::Ok(())
    })?;

    // 3) Overload `+=` on `external.value`
    module.field_function(Protocol::ADD_ASSIGN, "value", |s: &mut External, rhs: i64| {
        let new_val = s.value.checked_add(rhs)
            .ok_or_else(|| VmError::panic("Overflow in += operator"))?;
        s.value = new_val;
        VmResult::Ok(())
    })?;

    // 4) Register inc_by method for external.inc_by(amt)
    module.associated_function("inc_by", External::inc_by)?;

    Ok(module)
}

fn main() -> Result<()> {
    let mut context = Context::with_default_modules()?;
    context.install(&create_module()?)?;

    let script = r#"
        pub fn main(external) {
            println("initial: " + external.value.to_string());
            external.value += 5;
            println("after += 5 => " + external.value.to_string());

            match external.inc_by(42) {
                Ok(v) => println("inc_by(42) => " + v.to_string()),
                Err(e) => println("Error: " + e.to_string()),
            }
        }
    "#;

    let mut sources = Sources::new();
    sources.insert(Source::new("my_script", script)?)?;

    let mut diagnostics = Diagnostics::new();
    let options = Options::default();
    let unit = prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()?;

    let runtime = Arc::new(context.runtime()?);
    let mut vm = Vm::new(runtime, Arc::new(unit));

    let external = External { value: 100 };
    vm.call(&["main"], (external,))?;

    Ok(())
}
```

**Rune script** snippet:

```
fn main(external) {
    println("initial: " + external.value.to_string());
    external.value += 5;
    // ...
    match external.inc_by(42) {
        Ok(val) => println("Ok => " + val.to_string()),
        Err(e)  => println("Error => " + e.to_string()),
    }
}
```

**7. Additional Observations**

1.	**Visibility**: If you do not register a field or method, it does not exist from Rune’s perspective.

2.	**Performance**: Each external call adds dynamic dispatch overhead. Keep that minimal if performance is crucial.

3.	**Thread Safety**: External types are reference-counted by default. If you need them to be Send or Sync, ensure your Rust type is safe to share across threads.

4.	**Asynchronous**: If your external methods return impl Future + Send, you can call them in Rune .await blocks.

5.	**Type-level Operator Overloads**: Everything in this guide uses *field-based operator overloading*. If you want to define a “global” + or - directly on the type (like a + b rather than a.field += b), you can do so via module.inst_fn(Protocol::ADD, MyType::add_impl)?. That is more advanced but allows mytype1 + mytype2 syntax in Rune.

Using these techniques, you can seamlessly integrate your Rust domain logic into Rune scripts—exposing only the fields and methods needed, customizing operator behavior, and surfacing errors in a friendly way.