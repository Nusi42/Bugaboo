//! The expansion of `#[derive(Masked)]` (SEC-04, M2-F21).
//!
//! A view that leaves the server is masked by the fields of the catalogue it holds, and a view
//! inside it is masked by its own fields. The derive makes both by construction: every member of
//! the view has to say what it is (`field`, `nested` or `open`), so a member that holds another
//! view cannot be forgotten, and a hidden field of a nested view never leaves.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
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
    /// The member says it holds nothing that needs a mask.
    open: bool,
    /// What else the member is for, if anything.
    special: Special,
}

/// A member with a job beyond holding a field or a nested view (CF-06).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Special {
    /// Nothing more.
    None,
    /// The member holds the values of own fields, by the key of the field: the mask drops the
    /// ones whose field the person may not see in any issue type.
    OwnFields,
    /// The member holds the issue type of the item (an object with an `id`): the view is masked
    /// with the mask of that type.
    ItemType,
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
        let (field_word, nested, open, special) = read_member(ident, &field.attrs)?;
        members.push(Member {
            name: ident.to_string(),
            ty: &field.ty,
            field: field_word,
            nested,
            open,
            special,
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
    let own = members
        .iter()
        .filter(|member| member.special == Special::OwnFields)
        .map(|member| {
            let name = &member.name;
            quote! { ::bb_policy::drop_hidden_own_fields(value, #name, mask); }
        });
    let item_type = members
        .iter()
        .find(|member| member.special == Special::ItemType)
        .map(|member| member.name.clone());
    let pick_mask = item_type.as_ref().map(|name| {
        quote! {
            let item_mask = ::bb_policy::mask_of_item(value, #name, mask);
            let mask: &::bb_policy::FieldMask = &item_mask;
        }
    });
    // A view that does not say its issue type is masked with the lowest level its fields have in
    // any type, so a field that is hidden for some type never leaves with it (fail closed). The
    // views inside it still get the whole mask, to pick the mask of their own type.
    let (floor, own_mask) = if item_type.is_some() {
        (quote! {}, quote! { mask })
    } else {
        (
            quote! { let floor = mask.for_type(None); },
            quote! { &floor },
        )
    };
    // An open member is a plain value: a type that is itself a view with a mask would leak.
    let plain = members.iter().filter(|member| member.open).map(|member| {
        let ty = member.ty;
        quote_spanned! { ty.span() => plain::<#ty>(); }
    });
    Ok(quote! {
        const _: fn() = || {
            fn plain<T: ?Sized + ::bb_core::plain::Plain>() {}
            #(#plain)*
        };

        impl ::bb_policy::Masked for #view {
            const FIELDS: &'static [(&'static str, ::bb_core::fields::Field)] = &[#(#entries),*];

            fn mask(value: &mut ::bb_policy::MaskedValue, mask: &::bb_policy::FieldMask) {
                #pick_mask
                #floor
                ::bb_policy::drop_hidden(value, <Self as ::bb_policy::Masked>::FIELDS, #own_mask);
                #(#nested)*
                #(#own)*
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

type MemberKind = (Option<LitStr>, bool, bool, Special);

fn read_member(ident: &Ident, attrs: &[Attribute]) -> syn::Result<MemberKind> {
    let mut said = false;
    let (mut field, mut nested, mut open, mut special) = (None, false, false, Special::None);
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("masked")) {
        said = true;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("field") {
                field = Some(meta.value()?.parse::<LitStr>()?);
            } else if meta.path.is_ident("nested") {
                nested = true;
            } else if meta.path.is_ident("open") {
                open = true;
            } else if meta.path.is_ident("own_fields") {
                special = Special::OwnFields;
            } else if meta.path.is_ident("item_type") {
                special = Special::ItemType;
            } else {
                return Err(meta.error(
                    "expected `field = \"...\"`, `nested`, `open`, `own_fields` or `item_type` (SEC-04, M2-F21, CF-06)",
                ));
            }
            Ok(())
        })?;
    }
    if open && (field.is_some() || nested || special != Special::None) {
        return Err(syn::Error::new(
            ident.span(),
            format!("member `{ident}` is `open`, so it is neither a `field` nor `nested`"),
        ));
    }
    if said {
        Ok((field, nested, open, special))
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
