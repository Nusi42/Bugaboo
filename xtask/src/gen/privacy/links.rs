//! The links of a person, read from the source like the marks (ADR-0044, decision 1): which
//! fields name a person and what the person is to the row (`#[person(author)]`,
//! `#[person_for(kind = "person", link = "subject")]`), and which types hold the ID of a person
//! (`names_a_person!` in `bb_core::personal`).

use syn::{Attribute, GenericArgument, Item, PathArguments, Type};

/// One link written on a field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkMark {
    /// The word: `author`, `recipient`, `subject`, `actor` or `holder`.
    pub(crate) word: String,
    /// The kind of record whose rows the link is for (`#[person_for(…)]`); none for every row.
    pub(crate) kind: Option<String>,
}

/// The links written on a field, each one or what is wrong with it.
pub(crate) fn read_links(field: &syn::Field) -> Vec<Result<LinkMark, String>> {
    field
        .attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("person") {
                Some(read_person(attr))
            } else if attr.path().is_ident("person_for") {
                Some(read_person_for(attr))
            } else {
                None
            }
        })
        .collect()
}

fn read_person(attr: &Attribute) -> Result<LinkMark, String> {
    attr.parse_args::<syn::Ident>()
        .map(|word| LinkMark {
            word: word.to_string(),
            kind: None,
        })
        .map_err(|_| "has a #[person(…)] that does not name one link".to_owned())
}

fn read_person_for(attr: &Attribute) -> Result<LinkMark, String> {
    let mut kind = None;
    let mut word = None;
    attr.parse_nested_meta(|meta| {
        let value: syn::LitStr = meta.value()?.parse()?;
        if meta.path.is_ident("kind") {
            kind = Some(value.value());
        } else if meta.path.is_ident("link") {
            word = Some(value.value());
        }
        Ok(())
    })
    .map_err(|error| format!("has a #[person_for(…)] that is not understood: {error}"))?;
    match (kind, word) {
        (Some(kind), Some(word)) => Ok(LinkMark {
            word,
            kind: Some(kind),
        }),
        _ => Err("needs both kind = \"…\" and link = \"…\" in #[person_for(…)]".to_owned()),
    }
}

/// The names of every type a field's type is made of (`Option<Vec<UserId>>` is `Option`, `Vec`
/// and `UserId`), the last part of each path.
pub(crate) fn type_names(ty: &Type) -> Vec<String> {
    let mut names = Vec::new();
    collect(ty, &mut names);
    names
}

fn collect(ty: &Type, names: &mut Vec<String>) {
    match ty {
        Type::Path(path) => {
            if let Some(last) = path.path.segments.last() {
                names.push(last.ident.to_string());
                if let PathArguments::AngleBracketed(arguments) = &last.arguments {
                    for argument in &arguments.args {
                        if let GenericArgument::Type(inner) = argument {
                            collect(inner, names);
                        }
                    }
                }
            }
        }
        Type::Reference(reference) => collect(&reference.elem, names),
        Type::Slice(slice) => collect(&slice.elem, names),
        Type::Array(array) => collect(&array.elem, names),
        Type::Tuple(tuple) => tuple.elems.iter().for_each(|elem| collect(elem, names)),
        Type::Group(group) => collect(&group.elem, names),
        Type::Paren(paren) => collect(&paren.elem, names),
        _ => {}
    }
}

/// The types in `names_a_person!(crate::id::UserId, crate::event::Actor)`: the last part of each
/// path.
pub(crate) fn read_person_types(items: &[Item]) -> Vec<String> {
    let mut names = Vec::new();
    for item in items {
        let Item::Macro(item) = item else { continue };
        if item
            .mac
            .path
            .segments
            .last()
            .is_none_or(|s| s.ident != "names_a_person")
        {
            continue;
        }
        let types = item
            .mac
            .parse_body_with(syn::punctuated::Punctuated::<Type, syn::Token![,]>::parse_terminated);
        if let Ok(types) = types {
            names.extend(
                types
                    .iter()
                    .filter_map(|ty| type_names(ty).into_iter().next()),
            );
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(source: &str) -> syn::Field {
        let item: syn::ItemStruct = syn::parse_str(&format!("struct S {{ {source} }}")).unwrap();
        item.fields.into_iter().next().unwrap()
    }

    #[test]
    fn links_are_read_with_their_kinds() {
        let links = read_links(&field(
            "#[person_for(kind = \"person\", link = \"subject\")] #[person(author)] a: UserId",
        ));
        assert_eq!(
            links,
            [
                Ok(LinkMark {
                    word: "subject".into(),
                    kind: Some("person".into())
                }),
                Ok(LinkMark {
                    word: "author".into(),
                    kind: None
                }),
            ]
        );
        assert!(read_links(&field("#[person(a, b)] a: UserId"))[0].is_err());
        assert!(read_links(&field("#[person_for(kind = \"x\")] a: UserId"))[0].is_err());
    }

    #[test]
    fn the_type_names_include_the_types_inside() {
        let names = type_names(&field("a: Option<Vec<crate::id::UserId>>").ty);
        assert_eq!(names, ["Option", "Vec", "UserId"]);
    }

    #[test]
    fn the_person_types_are_the_last_part_of_each_path() {
        let file: syn::File =
            syn::parse_str("names_a_person!(crate::id::UserId, crate::event::Actor);").unwrap();
        assert_eq!(read_person_types(&file.items), ["UserId", "Actor"]);
    }
}
