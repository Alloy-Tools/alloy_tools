use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, token::Comma, DeriveInput, Meta};

/// Debugging attribute macro to print the input tokens
#[proc_macro_attribute]
pub fn show_attribute(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("attr: \"{attr}\"");
    println!("item: \"{item}\"");
    item
}

/// Debugging attribute macro to print only the item tokens
#[proc_macro_attribute]
pub fn show_item(_: TokenStream, item: TokenStream) -> TokenStream {
    println!("item: \"{item}\"");
    item
}

/// Derive the required elements for an `Event`
/// Adds EventRequirements bound to all generic parameters
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
    let generics_str = quote! {#type_generics}.to_string().replace(" ", "");
    quote! {impl #impl_generics EventMarker for #name #type_generics #where_clause {
        fn _type_name() -> &'static str {
            concat!(module_path!(), "::", stringify!(#name), #generics_str)
        }
        fn _module_path() -> &'static str {
            module_path!()
        }
    }}
    .into()
}

/// Attribute macro to mark a struct as an event, automatically implementing `EventMarker` and required traits.
///
/// Will cause conflicting implementations if placed after any `#derive(...)]` attributes that implement any super traits of `EventRequirements`.
#[proc_macro_attribute]
pub fn event(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(item as DeriveInput);

    let mut required_traits: Vec<syn::Path> = vec![
        syn::parse_quote!(Clone),
        syn::parse_quote!(Default),
        syn::parse_quote!(PartialEq),
        syn::parse_quote!(Hash),
        syn::parse_quote!(Debug),
        syn::parse_quote!(al_derive::EventMarker),
    ];

    #[cfg(feature = "serde")]
    let mut serde_traits: Vec<syn::Path> = vec![
        syn::parse_quote!(serde::Serialize),
        syn::parse_quote!(serde::Deserialize),
    ];

    // find any existing #[derive(...)] attributes and remove any duplicates from required_traits
    let _ = &item
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derive"))
        .filter_map(|attr| {
            attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated)
                .ok()
        })
        .flatten()
        .for_each(|meta| {
            if let Meta::Path(path) = meta {
                if let Some(pos) = required_traits.iter().position(|t| t == &path) {
                    required_traits.remove(pos);
                }
                #[cfg(feature = "serde")]
                if let Some(pos) = serde_traits.iter().position(|t| t == &path) {
                    serde_traits.remove(pos);
                }
            }
        });

    // Add all the missing required traits as a second #[derive(...)] attribute
    if !required_traits.is_empty() {
        item.attrs
            .push(syn::parse_quote!(#[derive(#(#required_traits),*)]));
    }

    #[cfg(feature = "serde")]
    // Add serde traits
    if !serde_traits.is_empty() {
        item.attrs
            .push(syn::parse_quote!(#[derive(#(#serde_traits),*)]));
    }

    quote! {
        #item
    }
    .into()
}
