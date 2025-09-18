use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive the required elements for an `Event`
#[proc_macro_derive(EventMarker)]
pub fn event_marker_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_event_marker(input)
}

/// Generate the implementation of EventMarker
/// type name concats module path with the name for 'path::to::module::TypeName'
fn derive_event_marker(input: DeriveInput) -> TokenStream {
    let name = &input.ident;
    /*TODO: add generics support
    let generics = &input.generics;*/
    quote! {impl EventMarker for #name {
        fn _type_name() -> &'static str {
            concat!(module_path!(), "::", stringify!(#name))
        }
    }}
    .into()
}