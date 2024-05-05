#![feature(concat_idents)]

mod bevy_macros;
mod stat;

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro]
pub fn app_register_types(input: TokenStream) -> TokenStream {
    crate::bevy_macros::app_register_types_impl(input)
}

/// Derive macro generating an impl of the trait `Stat`.
#[proc_macro_error]
#[proc_macro_derive(Stat, attributes(stat))]
pub fn stat_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    crate::stat::impl_stat_derive(&ast)
}
