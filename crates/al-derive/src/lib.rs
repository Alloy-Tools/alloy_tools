use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive the required elements for an `Event`
#[proc_macro_derive(EventMarker)]
pub fn event_marker_derive(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);

    for param in &mut input.generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            type_param.bounds.push(syn::parse_quote!(EventRequirements));
        }
    }
    derive_event_marker(input)
}

/// Generate the implementation of EventMarker
/// type name concats module path with the name for 'path::to::module::TypeName'
fn derive_event_marker(input: DeriveInput) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = &input.generics.split_for_impl();
    quote! {impl #impl_generics EventMarker for #name #type_generics #where_clause {
        fn _type_name() -> &'static str {
            concat!(module_path!(), "::", stringify!(#name))
        }
    }}
    .into()
}

#[proc_macro_attribute]
pub fn show_streams(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("attr: \"{attr}\"");
    println!("item: \"{item}\"");
    item
}

#[proc_macro_attribute]
pub fn event(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("attr: \"{attr}\"");
    println!("item: \"{item}\"");
    item
}

/*
/// Derive the required elements for an `Event`
#[proc_macro_derive(Event)]
pub fn event_derive(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);

    eprintln!("Attributes: {:?}", input.attrs);

    // Add EventRequirements bound to all generic parameters
    for param in &mut input.generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            type_param.bounds.push(syn::parse_quote!(EventRequirements));
        }
    }
    derive_event(input)
}

fn derive_event(mut input: DeriveInput) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = &input.generics.split_for_impl();

    // Add all the required traits as a second #[derive(...)] attribute
    let required_traits: Vec<syn::Path> = vec![
        syn::parse_quote!(Clone),
        syn::parse_quote!(Default),
        syn::parse_quote!(PartialEq),
        syn::parse_quote!(Hash),
        syn::parse_quote!(Debug),
    ];
    //input.attrs.push(syn::parse_quote!(#[derive(#(#required_traits),*)]));

    quote! {impl #impl_generics EventMarker for #name #type_generics #where_clause {
        fn _type_name() -> &'static str {
            concat!(module_path!(), "::", stringify!(#name))
        }
    }}
    .into()
}*/
