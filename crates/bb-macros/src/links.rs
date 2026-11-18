//! The links of a person in `#[derive(Entity)]` (ADR-0044, decision 1):
//!
//! - `#[person(author)]` on a field that names a person in every row;
//! - `#[person_for(record, link = "subject")]` on a reference to a record (a kind and an ID) that
//!   names a person when the record is one or belongs to one, as `bb_core::event::whose` says;
//! - `#[person_for(field_type = "person", key = "field", link = "subject")]` on a reference that
//!   names a person in the rows whose own field (`key`) is of that type, once for each type;
//! - `#[person_email(recipient)]` on the text of an e-mail address;
//! - `#[person_in_changes(subject)]` on the changes of a record, whose values can name a person;
//! - `#[names_no_person]` on a reference to a record that never names a person.
//!
//! Like the marks, the macro knows no words: a link or a field type is looked up in `bb_core` at
//! compile time. A field of a type that can name a person and has no link is found by DATA-MARKS,
//! which reads the lists of those types from `bb_core::personal`.

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Attribute, Field, LitStr};

/// The links written on one field, and the proofs that its type may carry them.
pub(crate) struct FieldLinks {
    pub(crate) links: Vec<TokenStream>,
    pub(crate) assertions: Vec<TokenStream>,
}

/// How a field names a person, as one attribute says it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Way {
    Always,
    Email,
    Changes,
    Rows,
    Never,
}

/// One attribute of the field: the way, the link word (none for `Never`), the scope and, for
/// `#[person_for(field_type = …)]`, the type that tells two of them apart.
struct Written {
    way: Way,
    span: Span,
    link: Option<LitStr>,
    scope: TokenStream,
    field_type: Option<String>,
}

pub(crate) fn read(field: &Field, name: &str) -> syn::Result<FieldLinks> {
    let mut written = Vec::new();
    for attr in &field.attrs {
        let path = attr.path();
        let one = if path.is_ident("person") {
            word(attr, "person", Way::Always, quote! { Every })?
        } else if path.is_ident("person_email") {
            word(attr, "person_email", Way::Email, quote! { Email })?
        } else if path.is_ident("person_in_changes") {
            word(attr, "person_in_changes", Way::Changes, quote! { Changes })?
        } else if path.is_ident("person_for") {
            read_for(attr)?
        } else if path.is_ident("names_no_person") {
            attr.meta.require_path_only()?;
            Written {
                way: Way::Never,
                span: attr.span(),
                link: None,
                scope: TokenStream::new(),
                field_type: None,
            }
        } else {
            continue;
        };
        written.push(one);
    }
    check_combination(&written, name)?;
    let ty = &field.ty;
    let mut links = Vec::new();
    let mut assertions = Vec::new();
    for one in &written {
        assertions.push(match one.way {
            Way::Always => quote_spanned! { ty.span() => names_a_person::<#ty>(); },
            Way::Email => quote_spanned! { ty.span() => holds_an_address::<#ty>(); },
            Way::Changes => quote_spanned! { ty.span() => carries_changes::<#ty>(); },
            Way::Rows | Way::Never => {
                quote_spanned! { ty.span() => refers_to_a_record::<#ty>(); }
            }
        });
        if let Some(word) = &one.link {
            let scope = &one.scope;
            links.push(quote_spanned! { word.span() =>
                ::bb_core::personal::PersonLink::new(
                    #name,
                    ::bb_core::personal::Link::from_code_const(#word),
                    ::bb_core::personal::LinkScope::#scope,
                )
            });
        }
    }
    Ok(FieldLinks { links, assertions })
}

/// A field names a person in one way, said once; only `#[person_for(field_type = …)]` may be
/// given more than once, once for each type.
fn check_combination(written: &[Written], name: &str) -> syn::Result<()> {
    let Some(first) = written.first() else {
        return Ok(());
    };
    let mut types: Vec<&str> = Vec::new();
    for (at, one) in written.iter().enumerate() {
        if one.way != first.way {
            return Err(syn::Error::new(
                one.span,
                format!(
                    "field `{name}` names a person in more than one way; a field names a person \
                     always (`person`), as an address (`person_email`), in its changes \
                     (`person_in_changes`), in some rows (`person_for`) or never \
                     (`names_no_person`)"
                ),
            ));
        }
        match &one.field_type {
            Some(field_type) if types.contains(&field_type.as_str()) => {
                return Err(syn::Error::new(
                    one.span,
                    format!(
                        "the field type `{field_type}` is given twice in `#[person_for(...)]` on \
                         `{name}`; keep one link for it"
                    ),
                ));
            }
            Some(field_type) => types.push(field_type),
            None if at > 0 => {
                return Err(syn::Error::new(
                    one.span,
                    format!("field `{name}` says more than once how it names a person; keep one"),
                ));
            }
            None => {}
        }
    }
    Ok(())
}

/// `#[person(author)]` and its kin: one word of `bb_core::personal::Link`.
fn word(attr: &Attribute, what: &str, way: Way, scope: TokenStream) -> syn::Result<Written> {
    let ident: syn::Ident = attr.parse_args().map_err(|_| {
        syn::Error::new(
            attr.span(),
            format!("`#[{what}(...)]` takes one link, a word of `bb_core::personal::Link`"),
        )
    })?;
    Ok(Written {
        way,
        span: attr.span(),
        link: Some(LitStr::new(&ident.to_string(), ident.span())),
        scope,
        field_type: None,
    })
}

/// `#[person_for(record, link = "…")]` or `#[person_for(field_type = "…", key = "…", link = "…")]`.
fn read_for(attr: &Attribute) -> syn::Result<Written> {
    let mut record = false;
    let mut kind = None;
    let mut link = None;
    let mut field_type = None;
    let mut key = None;
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("record") {
            record = true;
            return Ok(());
        }
        let target = if meta.path.is_ident("link") {
            &mut link
        } else if meta.path.is_ident("kind") {
            &mut kind
        } else if meta.path.is_ident("field_type") {
            &mut field_type
        } else if meta.path.is_ident("key") {
            &mut key
        } else {
            return Err(
                meta.error("expected `record`, or `link`, `kind`, `field_type` or `key` = \"...\"")
            );
        };
        if target.is_some() {
            return Err(meta.error("this part is given twice"));
        }
        *target = Some(meta.value()?.parse::<LitStr>()?);
        Ok(())
    })?;
    let Some(link) = link else {
        return Err(syn::Error::new(
            attr.span(),
            "`#[person_for(...)]` needs `link = \"...\"`: what the person is to the row",
        ));
    };
    let (scope, field_type) = match (record, field_type, key, kind) {
        (true, None, None, None) => (quote! { Record { kind: None } }, None),
        (true, None, None, Some(kind)) => (quote! { Record { kind: Some(#kind) } }, None),
        (false, Some(field_type), Some(key), None) => (
            quote_spanned! { field_type.span() => FieldType {
                key: #key,
                field_type: ::bb_core::own_fields::FieldType::from_code_const(#field_type),
            } },
            Some(field_type.value()),
        ),
        _ => {
            return Err(syn::Error::new(
                attr.span(),
                "`#[person_for(...)]` takes `record` (and maybe `kind`), or `field_type` and `key`, besides `link`",
            ));
        }
    };
    Ok(Written {
        way: Way::Rows,
        span: attr.span(),
        link: Some(link),
        scope,
        field_type,
    })
}
