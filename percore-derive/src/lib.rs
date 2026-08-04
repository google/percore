// Copyright 2026 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemStatic, parse_macro_input};

/// Marks the variable as percore, creating an instance for each core.
///
/// This replaces the static with a `percore::derive::LinkedPerCore` of the same name and places it
/// in the `.percore` linker section. The static's symbol is the base address of the per-core variable
/// and can be used to access it from assembly.
///
/// # Example
///
/// ```
/// use percore::{ExceptionLock, derive::percore};
/// use core::cell::RefCell;
///
/// #[percore]
/// static VARIABLE: ExceptionLock<RefCell<u64>> = ExceptionLock::new(RefCell::new(1));
/// ```
#[proc_macro_attribute]
pub fn percore(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let static_item = parse_macro_input!(item as ItemStatic);

    let attrs = &static_item.attrs;
    let vis = &static_item.vis;
    let name = &static_item.ident;
    let ty = &static_item.ty;
    let expr = &static_item.expr;

    quote! {
        #[cfg_attr(target_os = "none", unsafe(link_section = ".percore"))]
        #(#attrs)*
        #vis static #name: percore::derive::LinkedPerCore<#ty> = const {
            let value = #expr;
            unsafe { percore::derive::LinkedPerCore::new(value) }
        };
    }
    .into()
}
