use proc_macro::TokenStream;
use quote::quote;
use syn::{punctuated::Punctuated, Token, Type};

pub(crate) fn app_register_types_impl(input: TokenStream) -> TokenStream {
    use syn::parse::Parser;
    let types = Punctuated::<Type, Token![,]>::parse_separated_nonempty.parse(input).unwrap();

    let expanded = types.iter().map(|ty| {
        quote! {
            app.register_type::<#ty>();
        }
    });

    TokenStream::from(quote! {
        #( #expanded )*
    })
}
