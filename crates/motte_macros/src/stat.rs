use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{DeriveInput, Fields};

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
    let generics = &ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let value_field = find_stat_value_field(&ast);

    let gen = quote! {
        impl #impl_generics Default for #name #ty_generics #where_clause {
            fn default() -> Self {
                Self { #value_field: 0.0, ..Default::default() }
            }
        }

        impl #impl_generics #crate_ident::Stat for #name #ty_generics #where_clause {
            fn new(value: f32) -> Self {
                Self { #value_field: value, ..Default::default() }
            }

            fn value(&self) -> f32 {
                self.#value_field
            }

            fn value_mut(&mut self) -> &mut f32 {
                &mut self.#value_field
            }
        }

        impl #impl_generics Into<#name #ty_generics> for f32 #where_clause {
            fn into(self) -> #name #ty_generics {
                #name::new(self)
            }
        }

        impl #impl_generics std::ops::Deref for #name #ty_generics #where_clause {
            type Target = f32;
            fn deref(&self) -> &f32 {
                &self.#value_field
            }
        }
    };
    gen.into()
}

fn find_stat_value_field(ast: &DeriveInput) -> proc_macro2::TokenStream {
    match &ast.data {
        syn::Data::Struct(data) => {
            match &data.fields {
                // Handle tuple structs: Assumes single f32 field
                Fields::Unnamed(fields_unnamed) if fields_unnamed.unnamed.len() == 1 => {
                    let index = syn::Index::from(0);
                    quote!(#index)
                }
                // Handle named fields that might be annotated or using single named field
                Fields::Named(fields_named) => fields_named
                    .named
                    .iter()
                    .find_map(|f| {
                        if f.attrs.iter().any(|a| a.path().is_ident("stat")) {
                            f.ident.as_ref().map(|ident| quote!(#ident))
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| {
                        panic!("No field marked with #[stat(value)] and structure is not a simple tuple struct")
                    }),
                _ => panic!("Stat can only be derived for structs with exactly one field"),
            }
        }
        _ => panic!("Stat can only be derived for structs"),
    }
}
