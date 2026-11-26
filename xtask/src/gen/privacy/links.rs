//! The links of a person, read from the source like the marks (ADR-0044, decision 1): which
//! fields name a person, in which rows, and what the person is to the row (`#[person(…)]`,
//! `#[person_for(…)]`, `#[person_email(…)]`, `#[names_no_person]`), and which types hold the ID of
//! a person or refer to a record (`names_a_person!` and `refers_to_a_record!` in
//! `bb_core::personal`, and the aliases and newtypes of them in `bb-core`).

use syn::{Attribute, Fields, GenericArgument, Item, PathArguments, Type};

/// In which rows a link says the field names a person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    /// Every row (`#[person(…)]`).
    Every,
    /// Some rows (`#[person_for(…)]`), as the page words them.
    Rows(String),
    /// The rows that are a part of a record of the person (`#[person_for(part_of, …)]`): the
    /// field holds the ID of the record, whose type the derive checks.
    Part,
    /// The rows that hold the person's e-mail address (`#[person_email(…)]`).
    Email,
    /// The rows whose changes name the person (`#[person_in_changes]`); each member says what the
    /// person is to the event.
    Changes,
    /// The rows whose JSON holds the person's ID (`#[person_in_json(…)]`).
    Json,
    /// None: the reference never names a person (`#[names_no_person]`).
    Never,
}

/// One link written on a field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkMark {
    /// The word: `author`, `recipient`, `subject`, `actor` or `holder`; empty for `Never`.
    pub(crate) word: String,
    /// In which rows.
    pub(crate) scope: Scope,
}

/// The links written on a field, each one or what is wrong with it.
pub(crate) fn read_links(field: &syn::Field) -> Vec<Result<LinkMark, String>> {
    field
        .attrs
        .iter()
        .filter_map(|attr| {
            let path = attr.path();
            if path.is_ident("person") {
                Some(read_word(attr, "person", Scope::Every))
            } else if path.is_ident("person_email") {
                Some(read_word(attr, "person_email", Scope::Email))
            } else if path.is_ident("person_in_changes") {
                Some(Ok(LinkMark {
                    word: String::new(),
                    scope: Scope::Changes,
                }))
            } else if path.is_ident("person_in_json") {
                Some(read_word(attr, "person_in_json", Scope::Json))
            } else if path.is_ident("person_for") {
                Some(read_person_for(attr))
            } else if path.is_ident("names_no_person") {
                Some(Ok(LinkMark {
                    word: String::new(),
                    scope: Scope::Never,
                }))
            } else {
                None
            }
        })
        .collect()
}

fn read_word(attr: &Attribute, what: &str, scope: Scope) -> Result<LinkMark, String> {
    attr.parse_args::<syn::Ident>()
        .map(|word| LinkMark {
            word: word.to_string(),
            scope,
        })
        .map_err(|_| format!("has a #[{what}(…)] that does not name one link"))
}

fn read_person_for(attr: &Attribute) -> Result<LinkMark, String> {
    let mut parts: Vec<(String, String)> = Vec::new();
    attr.parse_nested_meta(|meta| {
        let name = meta
            .path
            .get_ident()
            .map_or_else(String::new, ToString::to_string);
        // `record` and `part_of` stand alone; every other part has a value.
        let value = if name == "record" || name == "part_of" {
            String::new()
        } else {
            meta.value()?.parse::<syn::LitStr>()?.value()
        };
        parts.push((name, value));
        Ok(())
    })
    .map_err(|error| format!("has a #[person_for(…)] that is not understood: {error}"))?;
    let part = |name: &str| {
        parts
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    };
    let Some(word) = part("link") else {
        return Err("needs link = \"…\" in #[person_for(…)]".to_owned());
    };
    if part("part_of").is_some() {
        return if parts.len() == 2 {
            Ok(LinkMark {
                word,
                scope: Scope::Part,
            })
        } else {
            Err("takes only link = \"…\" beside part_of in #[person_for(…)]".to_owned())
        };
    }
    let rows = match (
        part("record"),
        part("kind"),
        part("field_type"),
        part("key"),
    ) {
        (Some(_), None, None, None) => {
            "the rows about a record that is the person or belongs to them (`whose`)".to_owned()
        }
        (Some(_), Some(kind), None, None) => format!(
            "the rows about a record that is the person or belongs to them (`whose`), of the kind in `{kind}`"
        ),
        (None, None, Some(field_type), Some(key)) => {
            format!("the rows whose own field `{key}` is of the type `{field_type}`")
        }
        _ => {
            return Err(
                "needs record (and maybe kind = \"…\"), or field_type = \"…\" and key = \"…\" in #[person_for(…)]"
                    .to_owned(),
            );
        }
    };
    Ok(LinkMark {
        word,
        scope: Scope::Rows(rows),
    })
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
        Type::Tuple(tuple) => {
            names.push(TUPLE.to_owned());
            tuple.elems.iter().for_each(|elem| collect(elem, names));
        }
        Type::Group(group) => collect(&group.elem, names),
        Type::Paren(paren) => collect(&paren.elem, names),
        _ => {}
    }
}

/// The name `type_names` gives a tuple, which no link can be put on.
pub(crate) const TUPLE: &str = "(tuple)";

/// The containers a link sees through, as `bb_core::personal::NamesAPerson` does: a person type
/// inside anything else (a tuple, a container of its own) cannot carry a link.
pub(crate) const CONTAINERS: &[&str] = &["Option", "Vec", "Box", "BTreeSet", "BTreeMap"];

/// The types in `names_a_person!(…)`, `refers_to_a_record!(…)`, `carries_changes!(…)` or
/// `holds_json!(…)`: the last part of each path.
pub(crate) fn read_listed_types(items: &[Item], list: &str) -> Vec<String> {
    let mut names = Vec::new();
    for item in items {
        let Item::Macro(item) = item else { continue };
        if item
            .mac
            .path
            .segments
            .last()
            .is_none_or(|s| s.ident != list)
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

/// The aliases (`type Owner = UserId;`) and newtypes (`struct Owner(UserId);`) of the types in
/// `known` declared in `items`: a field of one of them names a person (or a record) as much.
pub(crate) fn aliases_of(items: &[Item], known: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for item in items {
        let (name, inner): (String, Vec<&Type>) = match item {
            Item::Type(alias) => (alias.ident.to_string(), vec![&*alias.ty]),
            Item::Struct(strukt) => match &strukt.fields {
                Fields::Unnamed(fields) => (
                    strukt.ident.to_string(),
                    fields.unnamed.iter().map(|field| &field.ty).collect(),
                ),
                _ => continue,
            },
            Item::Mod(module) => {
                if let Some((_, inner)) = &module.content {
                    found.extend(aliases_of(inner, known));
                }
                continue;
            }
            _ => continue,
        };
        if inner
            .iter()
            .any(|ty| type_names(ty).iter().any(|name| known.contains(name)))
        {
            found.push(name);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(source: &str) -> syn::Field {
        let item: syn::ItemStruct = syn::parse_str(&format!("struct S {{ {source} }}")).unwrap();
        item.fields.into_iter().next().unwrap()
    }

    #[test]
    fn links_are_read_with_their_scopes() {
        let links = read_links(&field(
            "#[person_for(record, kind = \"entity_kind\", link = \"subject\")] #[person(author)] #[person_email(recipient)] #[names_no_person] a: UserId",
        ));
        assert_eq!(links.len(), 4);
        assert_eq!(
            links[0].as_ref().unwrap().scope,
            Scope::Rows("the rows about a record that is the person or belongs to them (`whose`), of the kind in `entity_kind`".into())
        );
        assert_eq!(links[1].as_ref().unwrap().scope, Scope::Every);
        assert_eq!(links[2].as_ref().unwrap().scope, Scope::Email);
        assert_eq!(links[3].as_ref().unwrap().scope, Scope::Never);
        let part = read_links(&field(
            "#[person_for(part_of, link = \"author\")] a: DashboardId",
        ));
        assert_eq!(part[0].as_ref().unwrap().scope, Scope::Part);
        assert!(
            read_links(&field(
                "#[person_for(part_of, record, link = \"author\")] a: DashboardId"
            ))[0]
                .is_err()
        );
        assert!(read_links(&field("#[person(a, b)] a: UserId"))[0].is_err());
        assert!(read_links(&field("#[person_for(kind = \"x\")] a: UserId"))[0].is_err());
        assert!(
            read_links(&field(
                "#[person_for(field_type = \"person\", link = \"subject\")] a: EntityId"
            ))[0]
                .is_err()
        );
    }

    #[test]
    fn the_type_names_include_the_types_inside() {
        let names = type_names(&field("a: Option<Vec<crate::id::UserId>>").ty);
        assert_eq!(names, ["Option", "Vec", "UserId"]);
        assert_eq!(type_names(&field("a: (UserId, u8)").ty)[0], TUPLE);
    }

    #[test]
    fn the_listed_types_are_the_last_part_of_each_path() {
        let file: syn::File =
            syn::parse_str("names_a_person!(crate::id::UserId, crate::event::Actor);").unwrap();
        assert_eq!(
            read_listed_types(&file.items, "names_a_person"),
            ["UserId", "Actor"]
        );
    }

    #[test]
    fn aliases_and_newtypes_of_a_person_type_are_found() {
        let file: syn::File = syn::parse_str(
            "type Owner = Option<UserId>; pub struct Keeper(pub UserId); struct Other(u8); mod inner { type Deep = UserId; }",
        )
        .unwrap();
        assert_eq!(
            aliases_of(&file.items, &["UserId".to_owned()]),
            ["Owner", "Keeper", "Deep"]
        );
    }
}
