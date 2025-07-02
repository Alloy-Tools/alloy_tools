use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(EventMarker)]
pub fn event_marker_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_event_marker(input)
}

fn derive_event_marker(input: DeriveInput) -> TokenStream {
    let name = &input.ident;
    let generics = &input.generics;
    quote! {impl EventMarker for #name {
        fn _type_name() -> &'static str {
            concat!(module_path!(), "::", stringify!(#name))
        }
    }}
    .into()
}
