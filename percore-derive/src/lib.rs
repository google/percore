// Copyright 2026 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemFn, ItemStatic, parse_macro_input};

/// Marks the variable as percore, creating an instance for each core.
///
/// * Defines the variable in the percore section
/// * Creates a wrapper type that provides the get() function.
/// * Defines an instance of the wrapper.
#[proc_macro_attribute]
pub fn percore(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let variable = parse_macro_input!(item as ItemStatic);

    let attrs = &variable.attrs;
    let vis = &variable.vis;
    let name = &variable.ident;
    let ty = &variable.ty;
    let expr = &variable.expr;

    let percore_name = &format_ident!("PERCORE_BASE_{name}");
    let wrapper_ty = &format_ident!("PERCORE_WRAPPER_{name}");

    quote! {
        #[cfg_attr(target_os = "none", unsafe(link_section = ".percore"))]
        #(#attrs)*
        #vis static mut #percore_name: #ty = #expr;

        #[doc = concat!("Per-core wrapper for [`", stringify!(#name), "`]")]
        #vis struct #wrapper_ty;

        impl #wrapper_ty {
            #[doc = "Returns a shared reference to the value of the current CPU."]
            #[inline(always)]
            pub fn get(&self) -> &#ty {
                let offset = unsafe{ percore::derive::percore_local_offset() };
                unsafe { core::ptr::NonNull::from_ref(&#percore_name).byte_add(offset).as_ref() }
            }
        }

        #(#attrs)*
        #vis static #name: #wrapper_ty = #wrapper_ty;
    }
    .into()
}

/// Marks the function that returns the offset of the local cores percore area in bytes from the
/// beginning of the percore section.
/// It creates a `percore_local_offset` wrapper function that is the interface towards
/// `percore::derive`.
#[proc_macro_attribute]
pub fn percore_local_offset(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let ident = &func.sig.ident;

    quote! {
        #func

        #[doc = "percore_local_offset wrapper function."]
        #[unsafe(no_mangle)]
        #[inline(always)]
        pub extern "Rust" fn percore_local_offset() -> usize {
            #ident()
        }
    }
    .into()
}
