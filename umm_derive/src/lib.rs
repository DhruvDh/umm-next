//! # umm_derive
//!
//! Defines some proc macros to make exporting functions to rhai easier.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use quote::{ToTokens, format_ident, quote};
use syn::{FnArg, Token, parse_macro_input, punctuated::Punctuated};

#[proc_macro_error]
#[proc_macro_attribute]
/// Generates a version of a fallible function (that uses anyhow Result) that
/// returns an EvalAltResult instead.
///
/// * `input`: a token stream for a function that returns an anyhow::Result
pub fn generate_rhai_variant(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = attr.to_string();
    let mut is_impl_fn = attr.contains("Impl");
    let is_fallible_fn = attr.contains("Fallible");
    let to_mut_self_fn = attr.contains("Mut");

    let input = parse_macro_input!(input as syn::ItemFn);
    let og_fn = input.to_token_stream();
    let fn_name = input.sig.ident;
    let mut new_fn_name = format_ident!("{}_script", fn_name);

    let sig_args = input.sig.inputs;
    let mut is_impl_self_fn = false;

    let mut args = Punctuated::<_, Token![,]>::new();
    for arg in sig_args.clone().into_iter() {
        let arg = match arg {
            FnArg::Receiver(_) => {
                is_impl_self_fn = true;
                is_impl_fn = true;
                continue;
            }
            FnArg::Typed(a) => a.pat,
        };
        args.push(arg);
    }

    let sig_args = if to_mut_self_fn {
        let mut res = Punctuated::<_, Token![,]>::new();
        for arg in sig_args.into_iter() {
            let arg = match arg {
                FnArg::Receiver(_) => quote! {&mut self},
                FnArg::Typed(a) => quote! {#a},
            };
            res.push(quote! {#arg});
        }
        new_fn_name = format_ident!("{}_mut_script", fn_name);

        res
    } else {
        let mut res = Punctuated::<_, Token![,]>::new();
        for arg in sig_args.into_iter() {
            let arg = match arg {
                FnArg::Receiver(a) => quote! {#a},
                FnArg::Typed(a) => quote! {#a},
            };
            res.push(quote! {#arg});
        }
        res
    };

    let output = if is_fallible_fn {
        let output = input.sig.output.into_token_stream().to_string();

        let output = output.replace("-> ", "").replace(' ', "");

        if &output == "Result<()>" {
            quote!(-> Result<(), Box<EvalAltResult>>)
        } else if output.starts_with("Result<") {
            if output.replace("Result<", "").starts_with("Vec<") {
                let inner_type = if output.contains(',') {
                    let o = output
                        .replace("Result<", "")
                        .replace("Vec<", "")
                        .replace('>', "");
                    let o = o.split_once(',').unwrap().0;
                    format_ident!("{o}",)
                } else {
                    format_ident!(
                        "{}",
                        output
                            .replace("Result<", "")
                            .replace("Vec<", "")
                            .replace('>', "")
                    )
                };

                quote! {-> Result<Vec<#inner_type>, Box<EvalAltResult>>}
            } else {
                let inner_type = if output.contains(',') {
                    let o = output
                        .replace("Result<", "")
                        .replace("Vec<", "")
                        .replace('>', "");
                    let o = o.split_once(',').unwrap().0;
                    format_ident!("{o}",)
                } else {
                    format_ident!("{}", output.replace("Result<", "").replace('>', ""))
                };

                quote! {-> Result<#inner_type, Box<EvalAltResult>>}
            }
        } else {
            quote! {}
        }
    } else {
        input.sig.output.into_token_stream()
    };

    let match_expr = if is_impl_self_fn {
        quote! { self.#fn_name(#args) }
    } else if is_impl_fn {
        quote! { Self::#fn_name(#args) }
    } else {
        quote! { #fn_name(#args) }
    };

    // Build the output, possibly using quasi-quotation
    let expanded = if is_fallible_fn {
        quote! {
            #og_fn

            /// Macro generated variant of #fn_name that returns EvalAltResult.
            /// This allows the function to be used in scripts.
            pub fn #new_fn_name(#sig_args) #output {
                match #match_expr {
                    Ok(res) => Ok(res),
                    Err(e) => Err(format!("{}", e).into()),
                }
            }
        }
    } else {
        quote! {
            #og_fn

            /// Macro generated variant of #fn_name that returns EvalAltResult.
            /// This allows the function to be used in scripts.
            pub fn #new_fn_name(#sig_args) #output {
                #match_expr
            }
        }
    };

    // Hand the output tokens back to the compiler
    expanded.into()
}
