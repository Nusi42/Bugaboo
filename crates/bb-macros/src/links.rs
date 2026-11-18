//! The links of a person in `#[derive(Entity)]` (ADR-0044, decision 1): `#[person(author)]` on a
//! field that names a person in every row, `#[person_for(kind = "person", link = "subject")]` on a
//! reference to a record that is a person in the rows about that kind of record.
//!
//! Like the marks, the macro knows no words: a link or a kind is looked up in `bb_core` at compile
//! time. A field of a type that holds the ID of a person and has no link is found by DATA-MARKS,
//! which reads the list of those types from `bb_core::personal`.

use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::spanned::Spanned as _;
use syn::{Attribute, Field, LitStr};

/// The links written on one field, and for `#[person(…)]` the proof that its type names a person.
pub(crate) struct FieldLinks {
    pub(crate) links: Vec<TokenStream>,
    pub(crate) assertion: Option<TokenStream>,
}

pub(crate) fn read(field: &Field, name: &str) -> syn::Result<FieldLinks> {
    let mut links = Vec::new();
    let mut assertion = None;
    let mut seen_kinds: Vec<String> = Vec::new();
    for attr in &field.attrs {
        if attr.path().is_ident("person") {
            if assertion.is_some() {
                return Err(syn::Error::new(
                    attr.span(),
                    format!(
                        "field `{name}` has more than one `#[person(...)]`; keep exactly one link"
                    ),
                ));
            }
            let word = read_word(attr)?;
            let ty = &field.ty;
            let link = link_word(&word);
            links.push(quote_spanned! { word.span() =>
                ::bb_core::personal::PersonLink::new(#name, #link)
            });
            assertion = Some(quote_spanned! { ty.span() => names_a_person::<#ty>(); });
        } else if attr.path().is_ident("person_for") {
            let (kind, word) = read_for(attr)?;
            if seen_kinds.contains(&kind.value()) {
                return Err(syn::Error::new(
                    kind.span(),
                    format!(
                        "the kind `{}` is given twice in `#[person_for(...)]`; keep one link for it",
                        kind.value()
                    ),
                ));
            }
            seen_kinds.push(kind.value());
            let link = link_word(&word);
            links.push(quote_spanned! { kind.span() =>
                ::bb_core::personal::PersonLink::for_kind(
                    #name,
                    #link,
                    ::bb_core::event::RecordKind::from_code_const(#kind),
                )
            });
        }
    }
    if assertion.is_some() && !seen_kinds.is_empty() {
        return Err(syn::Error::new(
            field.span(),
            format!(
                "field `{name}` has `#[person(...)]` and `#[person_for(...)]`; a field names a \
                 person in every row or in the rows of some kinds, not both"
            ),
        ));
    }
    Ok(FieldLinks { links, assertion })
}

/// The one word of `#[person(author)]`.
fn read_word(attr: &Attribute) -> syn::Result<LitStr> {
    let ident: syn::Ident = attr.parse_args().map_err(|_| {
        syn::Error::new(
            attr.span(),
            "`#[person(...)]` takes one link: author, recipient, subject, actor or holder",
        )
    })?;
    Ok(LitStr::new(&ident.to_string(), ident.span()))
}

/// The kind and the link of `#[person_for(kind = "…", link = "…")]`.
fn read_for(attr: &Attribute) -> syn::Result<(LitStr, LitStr)> {
    let mut kind = None;
    let mut link = None;
    attr.parse_nested_meta(|meta| {
        let target = if meta.path.is_ident("kind") {
            &mut kind
        } else if meta.path.is_ident("link") {
            &mut link
        } else {
            return Err(meta.error("expected `kind = \"...\"` or `link = \"...\"`"));
        };
        if target.is_some() {
            return Err(meta.error("this part is given twice"));
        }
        *target = Some(meta.value()?.parse::<LitStr>()?);
        Ok(())
    })?;
    match (kind, link) {
        (Some(kind), Some(link)) => Ok((kind, link)),
        _ => Err(syn::Error::new(
            attr.span(),
            "`#[person_for(...)]` needs both `kind = \"...\"` and `link = \"...\"`",
        )),
    }
}

/// The word as a value of `Link`; an unknown word stops the build at the word.
fn link_word(text: &LitStr) -> TokenStream {
    quote_spanned! { text.span() => ::bb_core::personal::Link::from_code_const(#text) }
}
