//! The expansion of `#[derive(Entity)]`.
//!
//! The macro knows no words: a category or basis is looked up in `bb_core` by the code that is
//! evaluated at compile time, so the words are defined once, in `bb_core::personal`.

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Attribute, Data, DeriveInput, Field, Fields, Ident, LitStr};

/// What the author wrote on one field.
enum Mark {
    Personal { category: LitStr, basis: LitStr },
    NotPersonal,
}

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            input.ident.span(),
            "`Entity` describes a stored record, so it can only be derived for a struct (ADR-0015)",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new(
            input.ident.span(),
            "an entity needs named fields, so every field can carry its personal-data mark",
        ));
    };
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new(
            input.generics.span(),
            "an entity cannot have generic parameters; the personal-data map needs one fixed shape",
        ));
    }
    let entity = &input.ident;
    let name = entity.to_string();
    let mut marks = Vec::new();
    let mut assertions = Vec::new();
    let mut errors: Option<syn::Error> = None;
    for field in &fields.named {
        match field_mark(entity, field) {
            Ok((mark, assertion)) => {
                marks.push(mark);
                assertions.extend(assertion);
            }
            Err(error) => match &mut errors {
                Some(all) => all.combine(error),
                None => errors = Some(error),
            },
        }
    }
    if let Some(error) = errors {
        return Err(error);
    }
    Ok(quote! {
        impl ::bb_core::personal::Entity for #entity {
            const MARKS: ::bb_core::personal::EntityMarks =
                ::bb_core::personal::EntityMarks::new(#name, &[#(#marks),*]);
        }

        // A field marked `#[not_personal]` must have a type that cannot hold personal data.
        const _: fn() = || {
            fn not_personal<T: ?Sized + ::bb_core::personal::NotPersonal>() {}
            #(#assertions)*
        };
    })
}

/// The mark of one field, and for `#[not_personal]` the proof that its type may carry it.
fn field_mark(entity: &Ident, field: &Field) -> syn::Result<(TokenStream, Option<TokenStream>)> {
    let Some(ident) = &field.ident else {
        return Err(syn::Error::new(
            field.span(),
            "a field of an entity needs a name",
        ));
    };
    let name = ident.to_string();
    Ok(match read_mark(entity, ident, &field.attrs)? {
        Mark::NotPersonal => {
            let ty = &field.ty;
            (
                quote! { ::bb_core::personal::FieldMark::not_personal(#name) },
                Some(quote_spanned! { ty.span() => not_personal::<#ty>(); }),
            )
        }
        Mark::Personal { category, basis } => {
            let category = word(&category, "Category");
            let basis = word(&basis, "Basis");
            (
                quote! { ::bb_core::personal::FieldMark::personal(#name, #category, #basis) },
                None,
            )
        }
    })
}

fn read_mark(entity: &Ident, field: &Ident, attrs: &[Attribute]) -> syn::Result<Mark> {
    let mut found: Option<(Mark, Span)> = None;
    for attr in attrs {
        let mark = if attr.path().is_ident("personal") {
            read_personal(attr)?
        } else if attr.path().is_ident("not_personal") {
            attr.meta.require_path_only()?;
            Mark::NotPersonal
        } else {
            continue;
        };
        if found.is_some() {
            return Err(syn::Error::new(
                attr.span(),
                format!(
                    "field `{field}` of `{entity}` has more than one personal-data mark; keep exactly one"
                ),
            ));
        }
        found = Some((mark, attr.span()));
    }
    found.map(|(mark, _)| mark).ok_or_else(|| {
        syn::Error::new(
            field.span(),
            format!(
                "field `{field}` of `{entity}` has no personal-data mark; add \
                 `#[personal(category = \"...\", basis = \"...\")]` or `#[not_personal]` (ADR-0015)"
            ),
        )
    })
}

fn read_personal(attr: &Attribute) -> syn::Result<Mark> {
    let mut category = None;
    let mut basis = None;
    attr.parse_nested_meta(|meta| {
        let target = if meta.path.is_ident("category") {
            &mut category
        } else if meta.path.is_ident("basis") {
            &mut basis
        } else {
            return Err(meta.error("expected `category = \"...\"` or `basis = \"...\"`"));
        };
        if target.is_some() {
            return Err(meta.error("this part is given twice"));
        }
        *target = Some(meta.value()?.parse::<LitStr>()?);
        Ok(())
    })?;
    match (category, basis) {
        (Some(category), Some(basis)) => Ok(Mark::Personal { category, basis }),
        _ => Err(syn::Error::new(
            attr.span(),
            "`#[personal(...)]` needs both `category = \"...\"` and `basis = \"...\"`",
        )),
    }
}

/// The word as a value of its enum. An unknown word stops the build at the word, in the constant
/// evaluation of `bb_core`.
fn word(text: &LitStr, enum_name: &str) -> TokenStream {
    let enum_ident = Ident::new(enum_name, text.span());
    quote_spanned! { text.span() => ::bb_core::personal::#enum_ident::from_code_const(#text) }
}
