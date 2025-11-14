//! Procedural macros shared across the Lexum workspace.
//!
//! Exposes `tokio_test`, a drop-in replacement for `tokio::test` that adds a
//! configurable timeout (10 seconds by default).

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    ItemFn, Lit, LitInt, Meta, parse_macro_input, parse_quote, punctuated::Punctuated, token::Comma,
};

/// Wrapper around `tokio::test` that enforces a timeout.
#[proc_macro_attribute]
pub fn tokio_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);
    let metas: Punctuated<Meta, Comma> = if attr.is_empty() {
        Punctuated::new()
    } else {
        parse_macro_input!(attr with Punctuated<Meta, Comma>::parse_terminated)
    };

    let mut timeout_secs = 10u64;
    let mut forwarded: Vec<Meta> = Vec::new();

    for meta in metas.into_iter() {
        match &meta {
            Meta::NameValue(name_value) if name_value.path.is_ident("timeout") => {
                if let syn::Expr::Lit(expr_lit) = &name_value.value {
                    if let Lit::Int(lit_int) = &expr_lit.lit {
                        timeout_secs = lit_int
                            .base10_parse::<u64>()
                            .expect("timeout must be a positive integer");
                        continue;
                    }
                }
                panic!("timeout must be specified as an integer literal, e.g. timeout = 10");
            }
            _ => forwarded.push(meta),
        }
    }

    let block = func.block.clone();
    let timeout_lit = LitInt::new(&timeout_secs.to_string(), proc_macro2::Span::call_site());
    *func.block = parse_quote!({
        let fut = async move #block;
        let duration = std::time::Duration::from_secs(#timeout_lit);
        tokio::time::timeout(duration, fut)
            .await
            .expect(concat!("test exceeded ", #timeout_lit, "s timeout"))
    });

    let tokio_attr = if forwarded.is_empty() {
        quote! { #[tokio::test] }
    } else {
        quote! { #[tokio::test( #(#forwarded),* )] }
    };

    TokenStream::from(quote! {
        #tokio_attr
        #func
    })
}
