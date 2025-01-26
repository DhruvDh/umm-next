# Rune: Macros

Rune has support for macros. These are functions which expand into code, and can be used by library writers to "extend the compiler".
For now, the following type of macros are support:
• Function-like macros expanding to items (functions, type declarations, ..).
• Function-like macros expanding to expression (statements, blocks, async blocks, ..).
• Attribute macros expanding around a function.
Macros can currently only be defined natively. This is to get around the rather tricky issue that the code of a macro has to be runnable during compilation. Native modules have an edge here, because they have to be defined at a time when they are definitely available to the compiler.*Don't worry though, we will be playing around with `macro fn` as well, but at a later stage 😉 (See [issue #27](https://github.com/rune-rs/rune/issues/27)).*
Native modules also means we can re-use all the existing compiler infrastructure for Rune as a library for macro authors. Which is really nice!
**Writing a native macro**
The following is the definition of the `stringy_math!` macro. Which is a macro that can be invoked on expressions.
This relies heavily on a Rune-specific [`quote!` macro](https://docs.rs/rune/0/rune/macro.quote.html). Which is inspired by its [famed counterpart in the Rust world](https://docs.rs/quote/1/quote/). A major difference with Rune `quote!` is that we need to pass in the `MacroContext` when invoking it. This is a detail which will be covered in one of the advanced sections.

`use crate as rune;
use crate::ast;
use crate::compile;
use crate::macros::{quote, MacroContext, TokenStream};
use crate::parse::Parser;

/// Implementation of the `stringy_math!` macro.
#[rune::macro_]
pub fn stringy_math(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream, cx.input_span());

    let mut output = quote!(0);

    while !parser.is_eof()? {
        let op = parser.parse::<ast::Ident>()?;
        let arg = parser.parse::<ast::Expr>()?;

        output = match cx.resolve(op)? {
            "add" => quote!((#output) + #arg),
            "sub" => quote!((#output) - #arg),
            "div" => quote!((#output) / #arg),
            "mul" => quote!((#output) * #arg),
            _ => return Err(compile::Error::msg(op, "unsupported operation")),
        }
    }

    parser.eof()?;
    Ok(output.into_token_stream(cx)?)
}`
A macro is added to a [`Module`](https://docs.rs/rune/0/rune/module/struct.Module.html) using the [`Module::macro_`](https://docs.rs/rune/0/rune/module/struct.Module.html#method.macro_) function.

`pub fn module() -> Result<rune::Module, rune::ContextError> {
    let mut module = rune::Module::new(["test", "macros"]);
    module.macro_meta(stringy_math)?;
    Ok(module)
}`
With this module installed, we can now take `stringy_math!` for a spin.

`use ::test::macros::stringy_math;

pub fn main() {
    stringy_math!(add 10 sub 2 div 3 mul 100)
}`

Running this would return `200`.

- • Function-like macros expanding to items (functions, type declarations, ..).
- • Function-like macros expanding to expression (statements, blocks, async blocks, ..).
- • Attribute macros expanding around a function.