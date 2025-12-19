use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, token::Comma, DeriveInput,
    GenericParam, Ident, ItemFn, Meta, Path,
};

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
        if let GenericParam::Type(type_param) = param {
            type_param
                .bounds
                .push(parse_quote!(al_core::EventRequirements));
        }
    }
    derive_event_marker(input)
}

/// Generate the implementation of EventMarker
/// type name concats module path with the name for 'path::to::module::TypeName'
fn derive_event_marker(input: DeriveInput) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = &input.generics.split_for_impl();
    quote! {impl #impl_generics al_core::EventMarker for #name #type_generics #where_clause {
        fn module_path() -> &'static str {
            module_path!()
        }
    }}
    .into()
}

/// Attributte macro to add the required traits for an `Event`
#[proc_macro_attribute]
pub fn event_requirements(attrs: TokenStream, item: TokenStream) -> TokenStream {
    add_event_traits(
        parse_macro_input!(item as DeriveInput),
        parse_macro_input!(attrs with Punctuated<Meta, Comma>::parse_terminated),
    )
}

/// Helper function to add required `Event` traits to a DeriveInput
fn add_event_traits(mut item: DeriveInput, attrs: Punctuated<Meta, Comma>) -> TokenStream {
    let mut required_traits: Vec<Path> = vec![
        parse_quote!(Clone),
        parse_quote!(Default),
        parse_quote!(PartialEq),
        parse_quote!(Hash),
        parse_quote!(Debug),
    ];

    #[cfg(feature = "serde")]
    required_traits.extend(vec![
        parse_quote!(serde::Serialize),
        parse_quote!(serde::Deserialize),
    ]);

    // Remove any traits specified in the attribute arguments
    for arg in attrs {
        match arg {
            Meta::Path(path) => required_traits.retain(|t| t != &path),
            _ => panic!("Only trait paths like `Clone` or `serde::Serialize` are supported."),
        }
    }

    // Find any existing #[derive(...)] attributes and remove any duplicates from required_traits
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
            }
        });

    // Add all the missing required traits as a second #[derive(...)] attribute
    if !required_traits.is_empty() {
        item.attrs
            .push(parse_quote!(#[derive(#(#required_traits),*)]));
    }

    quote! {
        #item
    }
    .into()
}

/// Attribute macro to mark a struct as an event, automatically implementing `EventMarker` and required traits.
///
/// Will cause conflicting implementations if placed after any `#derive(...)]` attributes that implement any super traits of `EventRequirements`.
#[proc_macro_attribute]
pub fn event(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(item as DeriveInput);

    // Add the `EventMarker` derive if not already present
    if !item.attrs.iter().any(|attr| {
        if attr.path().is_ident("derive") {
            if let Ok(meta) = attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated) {
                return meta.iter().any(|m| match m {
                    Meta::Path(path) => path.is_ident("DeriveEventMarker"),
                    _ => false,
                });
            }
        }
        false
    }) {
        item.attrs
            .push(parse_quote!(#[derive(al_core::DeriveEventMarker)]));
    }

    // Use the `event_requirements` macro
    add_event_traits(
        item,
        parse_macro_input!(attrs with Punctuated<Meta, Comma>::parse_terminated),
    )
}

/// Helper attribute macro to add specific common bounds to functions to have a single place to edit the trait bounds
#[proc_macro_attribute]
pub fn with_bounds(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated<Ident, Comma>::parse_terminated);
    let mut input_fn = parse_macro_input!(item as ItemFn);

    // Check args to see which bounds to add
    let add_f = args.iter().any(|ident| ident == "F");
    let add_c = args.iter().any(|ident| ident == "C");

    if add_f || add_c {
        // get generics for processing and init where clause if missing
        let mut generics = &mut input_fn.sig.generics;
        {
            let _ = generics
                .where_clause
                .get_or_insert_with(|| syn::WhereClause {
                    where_token: Default::default(),
                    predicates: Punctuated::new(),
                });
        }

        // Add to generics and where clause
        if add_f {
            add_generic(&mut generics, "F");
            add_generic(&mut generics, "Fut");
            if let Some(where_clause) = &mut generics.where_clause {
                add_f_bound(where_clause);
            }
        }
        if add_c {
            add_generic(&mut generics, "C");
            add_generic(&mut generics, "FutC");
            if let Some(where_clause) = &mut generics.where_clause {
                add_c_bound(where_clause);
            }
        }
    }

    quote! { #input_fn }.into()
}

fn add_generic(generics: &mut syn::Generics, ident: &str) {
    generics.params.push(GenericParam::Type(syn::TypeParam {
        attrs: Vec::new(),
        ident: Ident::new(ident, proc_macro2::Span::call_site()),
        colon_token: None,
        bounds: Punctuated::new(),
        eq_token: None,
        default: None,
    }));
}

fn add_f_bound(where_clause: &mut syn::WhereClause) {
    where_clause.predicates.push(parse_quote! {
        F: FnMut(usize, &Arc<RwLock<S>>) -> Fut + Send + Sync + 'static
    });
    where_clause.predicates.push(parse_quote! {
        Fut: Future<Output = Result<T, E>> + Send + Sync + 'static
    });
}

fn add_c_bound(where_clause: &mut syn::WhereClause) {
    where_clause.predicates.push(parse_quote! {
        C: FnMut(&Arc<RwLock<S>>) -> FutC + Send + Sync + 'static
    });
    where_clause.predicates.push(parse_quote! {
        FutC: Future<Output = bool> + Send + Sync + 'static
    });
}
