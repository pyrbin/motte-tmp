use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{DeriveInput, Ident};

const CRATE_IDENT: &str = "motte_lib";

pub(super) fn impl_stat_derive(ast: &DeriveInput) -> TokenStream {
    let crate_ident = match crate_name(CRATE_IDENT)
        .unwrap_or_else(|_| panic!("expected {CRATE_IDENT:?} is present in `Cargo.toml`"))
    {
        FoundCrate::Itself => quote!(crate::stats::stat),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident::stats::stat )
        }
    };

    let name = &ast.ident;

    let data = match &ast.data {
        syn::Data::Struct(data_struct) => data_struct,
        _ => panic!("Stat can only be derived for structs"),
    };

    let fields = match &data.fields {
        syn::Fields::Unnamed(fields_unnamed) => &fields_unnamed.unnamed,
        _ => panic!("Stat can only be derived for tuple structs with a single f32 field"),
    };

    if fields.len() != 1 || !fields.iter().all(|f| matches!(f.ty, syn::Type::Path(ref p) if p.path.is_ident("f32"))) {
        panic!("Stat can only be derived for tuple structs with a single f32 field");
    }

    let gen = quote! {
        impl Default for #name {
            fn default() -> Self {
                Self(0.0)
            }
        }

        impl #crate_ident::Stat for #name {
            fn new(value: f32) -> Self {
                Self(value)
            }

            fn value(&self) -> f32 {
                self.0
            }

            fn value_mut(&mut self) -> &mut f32 {
                &mut self.0
            }
        }

        impl Into<#name> for f32 {
            fn into(self) -> #name {
                #name(self)
            }
        }

        impl std::ops::Deref for #name {
            type Target = f32;
            fn deref(&self) -> &f32 {
                &self.0
            }
        }
    };
    gen.into()
}
