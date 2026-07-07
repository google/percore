// Copyright 2026 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemStatic, ItemStruct, parse_macro_input};

/// Marks the variable as percore, creating an instance for each core.
///
/// * Defines the variable called `PERCORE_BASE_[variable name]` in the percore section. This can
///   be used for retrieving the base address (the address without the core's local offset) for
///   accessing the variable from assembly.
/// * Defines a `PerCoreWrapper` with the original variable name.
#[proc_macro_attribute]
pub fn percore(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let variable = parse_macro_input!(item as ItemStatic);

    let attrs = &variable.attrs;
    let vis = &variable.vis;
    let name = &variable.ident;
    let ty = &variable.ty;
    let expr = &variable.expr;

    let percore_base_name = &format_ident!("PERCORE_BASE_{name}");

    quote! {
        #[cfg_attr(target_os = "none", unsafe(link_section = ".percore"))]
        #(#attrs)*
        #vis static mut #percore_base_name: #ty = #expr;

        #(#attrs)*
        #[doc = concat!("Per-core wrapper for [`", stringify!(#name), "`]")]
        #vis static #name: percore::derive::PerCoreWrapper<#ty> =
            percore::derive::PerCoreWrapper::new(unsafe{ &#percore_base_name });
    }
    .into()
}

/// Marks the type that implements `percore::derive::PercoreLocalOffset`.
///
/// This creates the `percore_local_offset` function used internally by `percore::derive`.
#[proc_macro_attribute]
pub fn percore_local_offset(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let variable = parse_macro_input!(item as ItemStruct);
    let ident = &variable.ident;

    quote! {
        #variable

        #[doc(hidden)]
        mod __percore {
            use super::*;

            #[doc = "percore_local_offset wrapper function."]
            #[unsafe(no_mangle)]
            #[inline(always)]
            fn percore_local_offset() -> usize {
                <#ident as percore::derive::PercoreLocalOffset>::percore_local_offset()
            }
        }
    }
    .into()
}
