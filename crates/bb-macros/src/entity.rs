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
    let entity_doc = doc_of(&input.attrs);
    let mut marks = Vec::new();
    let mut links = Vec::new();
    let mut assertions = Vec::new();
    let mut person_assertions = Vec::new();
    let mut errors: Option<syn::Error> = None;
    for field in &fields.named {
        let read = field_mark(entity, field).and_then(|marked| {
            let name = field
                .ident
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            crate::links::read(field, &name).map(|linked| (marked, linked))
        });
        match read {
            Ok(((mark, assertion), linked)) => {
                marks.push(mark);
                assertions.extend(assertion);
                links.extend(linked.links);
                person_assertions.extend(linked.assertions);
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
    // A kind of record that names a person is shown to people (*Download my data*, *Remove a
    // person*) by a plain name of its own; `derived_key!` stops the build when it is missing.
    let title = if links.is_empty() {
        quote! {}
    } else {
        let word = kebab(&name);
        quote! { .with_title(::bb_core::derived_key!("personal-kind.", #word)) }
    };
    Ok(quote! {
        impl ::bb_core::personal::Entity for #entity {
            const MARKS: ::bb_core::personal::EntityMarks =
                ::bb_core::personal::EntityMarks::new(#name, &[#(#marks),*])
                    .with_links(&[#(#links),*])
                    .with_doc(#entity_doc)
                    #title;
        }

        // A field marked `#[not_personal]` must have a type that cannot hold personal data.
        const _: fn() = || {
            fn not_personal<T: ?Sized + ::bb_core::personal::NotPersonal>() {}
            #(#assertions)*
        };

        // A link must fit the type of its field: `#[person(...)]` the ID of a person,
        // `#[person_for(...)]` and `#[names_no_person]` a reference to a record, `#[person_email(...)]`
        // the text of an address.
        const _: fn() = || {
            fn names_a_person<T: ?Sized + ::bb_core::personal::NamesAPerson>() {}
            fn refers_to_a_record<T: ?Sized + ::bb_core::personal::RefersToARecord>() {}
            fn holds_an_address<T: ?Sized + ::bb_core::personal::HoldsAnAddress>() {}
            fn carries_changes<T: ?Sized + ::bb_core::personal::CarriesChanges>() {}
            fn holds_json<T: ?Sized + ::bb_core::personal::HoldsJson>() {}
            fn can_name_a_person<T: ?Sized + ::bb_core::personal::CanNameAPerson>() {}
            #(#person_assertions)*
        };
    })
}

/// The word of an entity in a message key: `OAuthGrant` is `oauth-grant`, `TwoStep` is
/// `two-step`.
fn kebab(name: &str) -> String {
    let mut word = String::with_capacity(name.len() + 4);
    let mut after_small = false;
    for letter in name.chars() {
        if letter.is_ascii_uppercase() && after_small {
            word.push('-');
        }
        after_small = letter.is_ascii_lowercase() || letter.is_ascii_digit();
        word.push(letter.to_ascii_lowercase());
    }
    word
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
    let kinds = read_kinds(&field.attrs)?;
    let with_kinds = if kinds.is_empty() {
        quote! {}
    } else {
        quote! { .with_kinds(&[#(#kinds),*]) }
    };
    let doc = doc_of(&field.attrs);
    let moment = if holds_moment(&field.ty) {
        quote! { .moment() }
    } else {
        quote! {}
    };
    let with_kinds = quote! { #with_kinds #moment .with_doc(#doc) };
    Ok(match read_mark(entity, ident, &field.attrs)? {
        Mark::NotPersonal => {
            let ty = &field.ty;
            (
                quote! { ::bb_core::personal::FieldMark::not_personal(#name) #with_kinds },
                Some(quote_spanned! { ty.span() => not_personal::<#ty>(); }),
            )
        }
        Mark::Personal { category, basis } => {
            let category = word(&category, "Category");
            let basis = word(&basis, "Basis");
            (
                quote! {
                    ::bb_core::personal::FieldMark::personal(#name, #category, #basis) #with_kinds
                },
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

/// The marks of the field in the rows of some kinds of record (`#[personal_for(…)]`).
fn read_kinds(attrs: &[Attribute]) -> syn::Result<Vec<TokenStream>> {
    let mut kinds = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for attr in attrs
        .iter()
        .filter(|attr| attr.path().is_ident("personal_for"))
    {
        let mut kind = None;
        let mut category = None;
        let mut basis = None;
        attr.parse_nested_meta(|meta| {
            let target = if meta.path.is_ident("kind") {
                &mut kind
            } else if meta.path.is_ident("category") {
                &mut category
            } else if meta.path.is_ident("basis") {
                &mut basis
            } else {
                return Err(meta.error(
                    "expected `kind = \"...\"`, `category = \"...\"` or `basis = \"...\"`",
                ));
            };
            if target.is_some() {
                return Err(meta.error("this part is given twice"));
            }
            *target = Some(meta.value()?.parse::<LitStr>()?);
            Ok(())
        })?;
        let (Some(kind), Some(category), Some(basis)) = (kind, category, basis) else {
            return Err(syn::Error::new(
                attr.span(),
                "`#[personal_for(...)]` needs `kind`, `category` and `basis`",
            ));
        };
        // One kind has one mark: a second would leave it open which of the two counts.
        if seen.contains(&kind.value()) {
            return Err(syn::Error::new(
                kind.span(),
                format!(
                    "the kind `{}` is given twice in `#[personal_for(...)]`; keep one mark for it",
                    kind.value()
                ),
            ));
        }
        seen.push(kind.value());
        let category = word(&category, "Category");
        let basis = word(&basis, "Basis");
        kinds.push(quote_spanned! { kind.span() =>
            ::bb_core::personal::KindMark::new(
                ::bb_core::event::RecordKind::from_code_const(#kind),
                #category,
                #basis,
            )
        });
    }
    Ok(kinds)
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

/// The doc comment as one line, for the readable pages and the `README` of *Download my data*.
fn doc_of(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| match &attr.meta {
            syn::Meta::NameValue(pair) => match &pair.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(text),
                    ..
                }) => Some(text.value().trim().to_owned()),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether the type is a moment (`Timestamp`, or an `Option` of one), which the store keeps as
/// microseconds since 1970: the walk of a person's data writes it as a moment in UTC.
fn holds_moment(ty: &syn::Type) -> bool {
    let syn::Type::Path(path) = ty else {
        return false;
    };
    path.path.segments.last().is_some_and(|last| {
        last.ident == "Timestamp"
            || (last.ident == "Option"
                && matches!(&last.arguments, syn::PathArguments::AngleBracketed(arguments)
                    if arguments.args.iter().any(|argument| matches!(argument,
                        syn::GenericArgument::Type(inner) if holds_moment(inner)))))
    })
}

#[cfg(test)]
mod tests {
    use super::kebab;

    #[test]
    fn the_word_of_an_entity_joins_its_words_with_dashes() {
        assert_eq!(kebab("Issue"), "issue");
        assert_eq!(kebab("TwoStep"), "two-step");
        assert_eq!(kebab("OAuthGrant"), "oauth-grant");
        assert_eq!(kebab("SecurityLogEntry"), "security-log-entry");
    }
}
