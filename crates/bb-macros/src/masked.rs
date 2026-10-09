//! The expansion of `#[derive(Masked)]` (SEC-04, M2-F21).
//!
//! A view that leaves the server is masked by the fields of the catalogue it holds, and a view
//! inside it is masked by its own fields. The derive makes both by construction: every member of
//! the view has to say what it is (`field`, `nested` or `open`), so a member that holds another
//! view cannot be forgotten, and a hidden field of a nested view never leaves.

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned as _;
use syn::{Attribute, Data, DeriveInput, Fields, Ident, LitStr};

/// What the author said about one member of the view.
struct Member<'a> {
    /// The name of the member in the JSON: the name of the Rust field.
    name: String,
    ty: &'a syn::Type,
    /// The field of the catalogue the member holds, as its word.
    field: Option<LitStr>,
    /// The member is a view of its own, or a list of them, that masks itself.
    nested: bool,
}

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            input.ident.span(),
            "`Masked` describes a view, so it can only be derived for a struct",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new(
            input.ident.span(),
            "a view needs named fields: the names are the members of its JSON",
        ));
    };
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new(
            input.generics.span(),
            "a masked view cannot have generic parameters; write `Masked` by hand for those",
        ));
    }
    refuse_serde(&input.attrs)?;
    let mut members = Vec::new();
    for field in &fields.named {
        refuse_serde(&field.attrs)?;
        let Some(ident) = &field.ident else {
            return Err(syn::Error::new(field.span(), "a member needs a name"));
        };
        let (field_word, nested) = read_member(ident, &field.attrs)?;
        members.push(Member {
            name: ident.to_string(),
            ty: &field.ty,
            field: field_word,
            nested,
        });
    }
    let view = &input.ident;
    let entries = members.iter().filter_map(|member| {
        member.field.as_ref().map(|word| {
            let name = &member.name;
            quote! { (#name, ::bb_core::fields::Field::from_code_const(#word)) }
        })
    });
    let nested = members.iter().filter(|member| member.nested).map(|member| {
        let (name, ty) = (&member.name, &member.ty);
        quote! { ::bb_policy::mask_member::<#ty>(value, #name, mask); }
    });
    Ok(quote! {
        impl ::bb_policy::Masked for #view {
            const FIELDS: &'static [(&'static str, ::bb_core::fields::Field)] = &[#(#entries),*];

            fn mask(value: &mut ::bb_policy::MaskedValue, mask: &::bb_policy::FieldMask) {
                ::bb_policy::drop_hidden(value, <Self as ::bb_policy::Masked>::FIELDS, mask);
                #(#nested)*
            }
        }
    })
}

/// The derive reads the names of the members from the struct, so a rename would make it mask the
/// wrong member. The one `serde` attribute that does not change a name is allowed: a member that
/// is left out when it has no value (`skip_serializing_if`).
fn refuse_serde(attrs: &[Attribute]) -> syn::Result<()> {
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("serde")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip_serializing_if") {
                meta.value()?.parse::<LitStr>().map(drop)
            } else {
                Err(meta.error(
                    "`Masked` uses the names of the fields as the members of the JSON, so the                      view cannot rename or move them with `#[serde(...)]`; name the field as the                      member (only `skip_serializing_if` is allowed)",
                ))
            }
        })?;
    }
    Ok(())
}

fn read_member(ident: &Ident, attrs: &[Attribute]) -> syn::Result<(Option<LitStr>, bool)> {
    let mut said = false;
    let (mut field, mut nested) = (None, false);
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("masked")) {
        said = true;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("field") {
                field = Some(meta.value()?.parse::<LitStr>()?);
            } else if meta.path.is_ident("nested") {
                nested = true;
            } else if !meta.path.is_ident("open") {
                return Err(
                    meta.error("expected `field = \"...\"`, `nested` or `open` (SEC-04, M2-F21)")
                );
            }
            Ok(())
        })?;
    }
    if said {
        Ok((field, nested))
    } else {
        Err(syn::Error::new(
            ident.span(),
            format!(
                "member `{ident}` does not say how it is masked; add `#[masked(field = \"...\")]` \
                 for a field of the catalogue, `#[masked(nested)]` for a view inside the view, or \
                 `#[masked(open)]` for a member everybody who sees the view may see (SEC-04)"
            ),
        ))
    }
}
