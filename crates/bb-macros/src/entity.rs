//! The expansion of `#[derive(Entity)]`.

use proc_macro2::{Span, TokenStream};
use quote::quote;
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
    let mut errors: Option<syn::Error> = None;
    for field in &fields.named {
        match field_mark(entity, field) {
            Ok(tokens) => marks.push(tokens),
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
    })
}

fn field_mark(entity: &Ident, field: &Field) -> syn::Result<TokenStream> {
    let Some(ident) = &field.ident else {
        return Err(syn::Error::new(
            field.span(),
            "a field of an entity needs a name",
        ));
    };
    let name = ident.to_string();
    let mark = read_mark(entity, ident, &field.attrs)?;
    Ok(match mark {
        Mark::NotPersonal => quote! { ::bb_core::personal::FieldMark::not_personal(#name) },
        Mark::Personal { category, basis } => {
            let category = variant(&category, "Category")?;
            let basis = variant(&basis, "Basis")?;
            quote! { ::bb_core::personal::FieldMark::personal(#name, #category, #basis) }
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

/// `employee_time` becomes `Category::EmployeeTime`. The variant gets the span of the text the
/// author wrote, so an unknown word is reported on that word.
fn variant(text: &LitStr, enum_name: &str) -> syn::Result<TokenStream> {
    let value = text.value();
    let plain = value.starts_with(|c: char| c.is_ascii_lowercase())
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !value.contains("__")
        && !value.ends_with('_');
    if !plain {
        return Err(syn::Error::new(
            text.span(),
            "use lower-case words joined by underscores, like \"employee_time\"",
        ));
    }
    let camel: String = value
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect();
    let enum_ident = Ident::new(enum_name, text.span());
    let variant = Ident::new(&camel, text.span());
    Ok(quote! { ::bb_core::personal::#enum_ident::#variant })
}
