//! The TypeScript types of Rust types that are written or read as JSON, read with `syn` from the
//! source (coding-standards §1). Three generators use it: the book of a load run
//! (`load_book.rs`), the wording of `bb-core` (`word_types.rs`) and the search language
//! (`search_types.rs`). xtask does not link the product crates, so the Rust source is read with
//! `syn`.
//!
//! What is understood (anything else is an error that names the type, so a new shape is never
//! mapped wrongly in silence):
//!
//! - `String` is `string`, the integers and floats are `number`, `bool` is `boolean`, a `Box<T>` is
//!   the `T` it holds;
//! - `Vec<T>` is `readonly T[]`;
//! - `Option<T>` is `T | null`; with `#[serde(default)]` the key may be missing too (`?:`), and
//!   with `skip_serializing_if` it is never written as `null`;
//! - a struct is an `interface` with `readonly` keys, `#[serde(rename = "…")]` names the key;
//! - an enum of plain variants is a union of strings, `#[serde(rename_all = "snake_case")]` and
//!   `#[serde(rename = "…")]` name the words;
//! - `&str` and `MessageKey` are `string`, `serde_json::Value` is `unknown`, a `BTreeMap` or
//!   `HashMap` is a `Readonly<Record<string, ...>>`;
//! - a non-`Option` field with `#[serde(default)]` may be missing (`?:`);
//! - an enum with `#[serde(tag = "...")]` is a union of objects that each carry the tag, and an enum
//!   with `#[serde(untagged)]` is a union of its variants' shapes (a newtype variant is the type it
//!   holds, a variant with named fields is an object);
//! - a type that the API client already declares (`EXTERNAL_TYPES`) is named from there;
//! - another type of the file is named as it is.
//!

use std::fmt::Write as _;

use anyhow::{Context as _, bail};
use syn::{Attribute, Fields, Item, Meta, Type};

use super::Generated;
use super::api::format_with_oxfmt;
use crate::repo::Repo;
use crate::source::read_text;

/// What `#[serde(…)]` says about an item, a field or a variant.
#[derive(Default)]
pub(crate) struct Serde {
    pub(crate) rename: Option<String>,
    pub(crate) rename_all: Option<String>,
    pub(crate) default: bool,
    pub(crate) skip_serializing_if: bool,
    pub(crate) tag: Option<String>,
    pub(crate) untagged: bool,
}

pub(crate) fn serde_of(attributes: &[Attribute]) -> anyhow::Result<Serde> {
    let mut found = Serde::default();
    for attribute in attributes.iter().filter(|a| a.path().is_ident("serde")) {
        attribute.parse_nested_meta(|meta| {
            let text = |meta: &syn::meta::ParseNestedMeta<'_>| -> syn::Result<String> {
                Ok(meta.value()?.parse::<syn::LitStr>()?.value())
            };
            if meta.path.is_ident("rename") {
                found.rename = Some(text(&meta)?);
            } else if meta.path.is_ident("rename_all") {
                found.rename_all = Some(text(&meta)?);
            } else if meta.path.is_ident("tag") {
                found.tag = Some(text(&meta)?);
            } else if meta.path.is_ident("untagged") {
                found.untagged = true;
            } else if meta.path.is_ident("default") {
                found.default = true;
            } else if meta.path.is_ident("skip_serializing_if") {
                text(&meta)?;
                found.skip_serializing_if = true;
            } else {
                return Err(meta.error("this serde setting is not understood by the generator"));
            }
            Ok(())
        })?;
    }
    Ok(found)
}

/// The text of the doc comment of an item, on one line.
pub(crate) fn docs_of(attributes: &[Attribute]) -> String {
    let lines: Vec<String> = attributes
        .iter()
        .filter_map(|attribute| match &attribute.meta {
            Meta::NameValue(pair) if pair.path.is_ident("doc") => match &pair.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(text),
                    ..
                }) => Some(text.value().trim().to_owned()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    lines.join(" ")
}

fn doc_comment(docs: &str, indent: &str) -> String {
    if docs.is_empty() {
        String::new()
    } else {
        format!("{indent}/** {docs} */\n")
    }
}

/// Whether the item is written or read as JSON.
pub(crate) fn is_serde_type(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .filter(|a| a.path().is_ident("derive"))
        .any(|attribute| {
            let mut found = false;
            let _ = attribute.parse_nested_meta(|meta| {
                found |= meta.path.is_ident("Serialize") || meta.path.is_ident("Deserialize");
                Ok(())
            });
            found
        })
}

/// Types that the generated API client declares already, by their Rust name, and how the browser
/// names them. A file that uses one imports `Schema` from the client.
const EXTERNAL_TYPES: [(&str, &str); 4] = [
    ("FileEvent", "Schema<\"FileEvent\">"),
    ("FieldType", "Schema<\"FieldType\">"),
    ("NotificationKind", "Schema<\"NotificationKind\">"),
    ("NoticeKind", "Schema<\"NoticeKind\">"),
];

/// A Rust type as TypeScript, and whether it was an `Option`. `known` collects the names of other
/// types that it refers to.
fn ts_type(ty: &Type, known: &mut Vec<String>) -> anyhow::Result<(String, bool)> {
    if let Type::Reference(reference) = ty {
        return match &*reference.elem {
            Type::Path(path) if path.path.is_ident("str") => Ok(("string".to_owned(), false)),
            _ => bail!("a reference to anything but `str` cannot be mapped"),
        };
    }
    let Type::Path(path) = ty else {
        bail!("a type that is not a plain name cannot be mapped");
    };
    let segment = path.path.segments.last().context("an empty type")?;
    let name = segment.ident.to_string();
    let argument = |known: &mut Vec<String>| -> anyhow::Result<(String, bool)> {
        let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
            bail!("`{name}` needs one type argument");
        };
        match arguments.args.first() {
            Some(syn::GenericArgument::Type(inner)) => ts_type(inner, known),
            _ => bail!("`{name}` needs one type argument"),
        }
    };
    Ok(match name.as_str() {
        "String" | "MessageKey" => ("string".to_owned(), false),
        "bool" => ("boolean".to_owned(), false),
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" | "f32"
        | "f64" => ("number".to_owned(), false),
        "Option" => (argument(known)?.0, true),
        "Box" => (argument(known)?.0, false),
        "Value" => ("unknown".to_owned(), false),
        "BTreeMap" | "HashMap" => {
            let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                bail!("`{name}` needs two type arguments");
            };
            let Some(syn::GenericArgument::Type(value)) = arguments.args.iter().nth(1) else {
                bail!("`{name}` needs two type arguments");
            };
            let inner = ts_type(value, known)?.0;
            (format!("Readonly<Record<string, {inner}>>"), false)
        }
        "Vec" => {
            let inner = argument(known)?.0;
            let inner = if inner.contains(' ') {
                format!("({inner})")
            } else {
                inner
            };
            (format!("readonly {inner}[]"), false)
        }
        other => {
            if let Some((_, client)) = EXTERNAL_TYPES.iter().find(|(rust, _)| *rust == other) {
                return Ok(((*client).to_owned(), false));
            }
            known.push(name.clone());
            (name, false)
        }
    })
}

/// `DraftEnd` as `draft_end`.
pub(crate) fn snake_case(name: &str) -> String {
    let mut out = String::new();
    for (at, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if at > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// The lines of the named fields of a struct or of a variant, at this indentation.
fn field_lines(
    fields: &syn::FieldsNamed,
    owner: &dyn std::fmt::Display,
    pad: &str,
    known: &mut Vec<String>,
) -> anyhow::Result<String> {
    let mut out = String::new();
    for field in &fields.named {
        let ident = field.ident.as_ref().context("a field without a name")?;
        let serde = serde_of(&field.attrs)?;
        let key = serde.rename.unwrap_or_else(|| ident.to_string());
        let (ts, option) = ts_type(&field.ty, known)
            .with_context(|| format!("the field `{ident}` of `{owner}`"))?;
        let (mark, ts) = match (option, serde.default, serde.skip_serializing_if) {
            (false, true, _) | (true, true, true) => ("?", ts),
            (false, false, _) => ("", ts),
            (true, true, false) => ("?", format!("{ts} | null")),
            (true, false, _) => ("", format!("{ts} | null")),
        };
        out.push_str(&doc_comment(&docs_of(&field.attrs), pad));
        let _ = writeln!(out, "{pad}readonly {key}{mark}: {ts};");
    }
    Ok(out)
}

fn interface(item: &syn::ItemStruct, known: &mut Vec<String>) -> anyhow::Result<String> {
    let name = &item.ident;
    if serde_of(&item.attrs)?.rename_all.is_some() {
        bail!("`{name}`: `rename_all` on a struct is not understood; rename the fields");
    }
    let Fields::Named(fields) = &item.fields else {
        bail!("`{name}` is not a struct with named fields");
    };
    let mut out = format!(
        "{}export interface {name} {{\n",
        doc_comment(&docs_of(&item.attrs), "")
    );
    out.push_str(&field_lines(fields, name, "  ", known)?);
    out.push_str("}\n\n");
    Ok(out)
}

/// The word a variant has in the JSON: its own name, or the name made by `rename_all`.
pub(crate) fn variant_word(item: &syn::ItemEnum, variant: &syn::Variant) -> anyhow::Result<String> {
    let name = &item.ident;
    let rule = serde_of(&item.attrs)?.rename_all;
    if rule
        .as_deref()
        .is_some_and(|rule| rule != "snake_case" && rule != "kebab-case")
    {
        bail!("`{name}`: only `rename_all` = snake_case and kebab-case are understood");
    }
    Ok(match (serde_of(&variant.attrs)?.rename, rule.as_deref()) {
        (Some(own), _) => own,
        (None, Some("snake_case")) => snake_case(&variant.ident.to_string()),
        (None, Some(_)) => snake_case(&variant.ident.to_string()).replace('_', "-"),
        (None, None) => variant.ident.to_string(),
    })
}

fn union(item: &syn::ItemEnum, known: &mut Vec<String>) -> anyhow::Result<String> {
    let name = &item.ident;
    let serde = serde_of(&item.attrs)?;
    let mut shapes = Vec::new();
    for variant in &item.variants {
        let word = variant_word(item, variant)?;
        let owner = format!("{name}::{}", variant.ident);
        let shape = match (&variant.fields, &serde.tag, serde.untagged) {
            (Fields::Unit, None, false) => format!("{word:?}"),
            (Fields::Unit, Some(tag), false) => format!("{{ readonly {tag}: {word:?} }}"),
            (Fields::Named(fields), Some(tag), false) => format!(
                "{{\n  readonly {tag}: {word:?};\n{}}}",
                field_lines(fields, &owner, "  ", known)?
            ),
            (Fields::Unnamed(fields), None, true) if fields.unnamed.len() == 1 => {
                ts_type(&fields.unnamed[0].ty, known)
                    .with_context(|| format!("the variant `{owner}`"))?
                    .0
            }
            (Fields::Named(fields), None, true) => {
                format!("{{\n{}}}", field_lines(fields, &owner, "  ", known)?)
            }
            _ => bail!(
                "`{owner}` holds data that is not understood: use plain variants, a tag with named fields, or untagged with one value or named fields"
            ),
        };
        shapes.push(shape);
    }
    Ok(format!(
        "{}export type {name} = {};\n\n",
        doc_comment(&docs_of(&item.attrs), ""),
        shapes.join(" | ")
    ))
}

/// The TypeScript declarations of every type of the source that is written or read as JSON, in the
/// order of the source, under a header (the generated file says which command wrote it, and what it
/// imports).
pub(crate) fn types_under(source: &str, header: &str) -> anyhow::Result<String> {
    let file = syn::parse_file(source).context("it is not Rust")?;
    let mut out = String::from(header);
    let mut declared = Vec::new();
    let mut known = Vec::new();
    for item in &file.items {
        match item {
            Item::Struct(item) if is_serde_type(&item.attrs) => {
                declared.push(item.ident.to_string());
                out.push_str(&interface(item, &mut known)?);
            }
            Item::Enum(item) if is_serde_type(&item.attrs) => {
                declared.push(item.ident.to_string());
                out.push_str(&union(item, &mut known)?);
            }
            _ => {}
        }
    }
    if declared.is_empty() {
        bail!("it declares no type that is written or read as JSON");
    }
    if let Some(missing) = known.iter().find(|name| !declared.contains(name)) {
        bail!("it names `{missing}`, which it does not declare as JSON");
    }
    Ok(out.trim_end().to_owned() + "\n")
}

/// The generated file of one Rust source: the types of `source`, formatted, in `output`.
pub(crate) fn generated_file(
    repo: &Repo,
    output: &'static str,
    source: &str,
    header: &str,
) -> anyhow::Result<Vec<Generated>> {
    let text = types_under(&read_text(repo, source)?, header)
        .with_context(|| format!("{source} cannot be written as types"))?;
    Ok(vec![Generated::new(
        output,
        format_with_oxfmt(repo, output, &text)?,
    )])
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "// test\n\n";

    fn types(source: &str) -> anyhow::Result<String> {
        types_under(source, HEADER)
    }

    fn one(fields: &str) -> String {
        types(&format!(
            "#[derive(Serialize)] pub struct Row {{ {fields} }}"
        ))
        .unwrap()
    }

    #[test]
    fn text_numbers_and_switches_become_string_number_and_boolean() {
        let text =
            one("pub a: String, pub b: i64, pub c: u32, pub d: usize, pub e: f64, pub f: bool");
        for line in [
            "readonly a: string;",
            "readonly b: number;",
            "readonly c: number;",
            "readonly d: number;",
            "readonly e: number;",
            "readonly f: boolean;",
        ] {
            assert!(text.contains(line), "{text}");
        }
        assert!(text.contains("export interface Row {"), "{text}");
    }

    #[test]
    fn a_list_becomes_a_readonly_array() {
        let text = one("pub a: Vec<String>");
        assert!(text.contains("readonly a: readonly string[];"), "{text}");
    }

    #[test]
    fn an_option_that_is_left_out_when_empty_is_an_optional_key() {
        let text = one(
            "#[serde(default, skip_serializing_if = \"Option::is_none\")] pub a: Option<String>",
        );
        assert!(text.contains("readonly a?: string;"), "{text}");
    }

    #[test]
    fn an_option_that_may_be_missing_when_read_but_is_written_as_null_says_so() {
        let text = one("#[serde(default)] pub a: Option<bool>");
        assert!(text.contains("readonly a?: boolean | null;"), "{text}");
    }

    #[test]
    fn an_option_without_serde_settings_is_a_key_that_may_be_null() {
        let text = one("pub a: Option<i64>");
        assert!(text.contains("readonly a: number | null;"), "{text}");
    }

    #[test]
    fn a_renamed_field_has_the_name_of_the_json() {
        let text = one("#[serde(rename = \"other\")] pub a: String");
        assert!(text.contains("readonly other: string;"), "{text}");
    }

    #[test]
    fn an_enum_of_plain_variants_is_a_union_of_snake_case_words() {
        let text = types(
            "#[derive(Deserialize)] #[serde(rename_all = \"snake_case\")] pub enum End { Saved, StillOpen, #[serde(rename = \"gone\")] Vanished }",
        )
        .unwrap();
        assert!(
            text.contains("export type End = \"saved\" | \"still_open\" | \"gone\";"),
            "{text}"
        );
    }

    #[test]
    fn an_enum_without_a_rule_keeps_the_names_of_its_variants() {
        let text = types("#[derive(Serialize)] pub enum End { Saved }").unwrap();
        assert!(text.contains("export type End = \"Saved\";"), "{text}");
    }

    #[test]
    fn another_type_of_the_file_is_named_and_kept_in_the_order_of_the_source() {
        let text = types(
            "#[derive(Serialize)] pub struct Inner { pub a: String }\n#[derive(Serialize)] pub struct Outer { pub inner: Inner, pub all: Vec<Inner> }",
        )
        .unwrap();
        assert!(text.contains("readonly inner: Inner;"), "{text}");
        assert!(text.contains("readonly all: readonly Inner[];"), "{text}");
        assert!(text.find("interface Inner").unwrap() < text.find("interface Outer").unwrap());
    }

    #[test]
    fn the_doc_comments_are_kept() {
        let text = types(
            "/// A row.\n#[derive(Serialize)] pub struct Row {\n /// The name.\n pub a: String }",
        )
        .unwrap();
        assert!(
            text.contains("/** A row. */\nexport interface Row"),
            "{text}"
        );
        assert!(
            text.contains("  /** The name. */\n  readonly a: string;"),
            "{text}"
        );
    }

    #[test]
    fn types_that_are_not_written_as_json_are_left_out() {
        let text = types(
            "#[derive(Serialize)] pub struct Row { pub a: String }\npub struct Other { pub a: Vec<u8> }\nimpl Row { fn x() {} }",
        )
        .unwrap();
        assert!(!text.contains("Other"), "{text}");
    }

    #[test]
    fn a_shape_that_cannot_be_mapped_is_an_error() {
        for source in [
            "#[derive(Serialize)] pub struct Row { pub a: (u8, u8) }",
            "#[derive(Serialize)] pub struct Row { pub a: Other }",
            "#[derive(Serialize)] pub struct Row(u8);",
            "#[derive(Serialize)] #[serde(rename_all = \"camelCase\")] pub struct Row { pub a: u8 }",
            "#[derive(Serialize)] #[serde(rename_all = \"camelCase\")] pub enum End { A }",
            "#[derive(Serialize)] pub enum End { A(u8) }",
            "#[derive(Serialize)] pub struct Row { #[serde(flatten)] pub a: u8 }",
            "pub struct Row { pub a: u8 }",
            "not rust",
        ] {
            assert!(types(source).is_err(), "{source}");
        }
    }

    #[test]
    fn an_enum_with_a_tag_is_a_union_of_objects_that_carry_the_tag() {
        let text = types(
            "#[derive(Serialize)] #[serde(tag = \"shown\", rename_all = \"kebab-case\")] pub enum Side { Text { text: String }, TwoWords { #[serde(default)] more: Option<String>, list: Vec<String> } }",
        )
        .unwrap();
        for expected in [
            "readonly shown: \"text\";",
            "readonly shown: \"two-words\";",
            "readonly more?: string | null;",
            "readonly list: readonly string[];",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
    }

    #[test]
    fn an_untagged_enum_is_a_union_of_what_its_variants_hold() {
        let text = types(
            "#[derive(Serialize)] #[serde(untagged)] pub enum Said { Text(String), Parts(Vec<Said>), Message { key: MessageKey, args: BTreeMap<&'static str, Said> } }",
        )
        .unwrap();
        assert!(
            text.contains("export type Said = string | readonly Said[] | {"),
            "{text}"
        );
        assert!(text.contains("readonly key: string;"), "{text}");
        assert!(
            text.contains("readonly args: Readonly<Record<string, Said>>;"),
            "{text}"
        );
    }

    #[test]
    fn a_field_with_a_default_may_be_missing_and_a_known_client_type_is_named_from_there() {
        let text = one("#[serde(default)] pub a: Vec<String>, pub kind: FieldType, pub v: Value");
        assert!(text.contains("readonly a?: readonly string[];"), "{text}");
        assert!(
            text.contains("readonly kind: Schema<\"FieldType\">;"),
            "{text}"
        );
        assert!(text.contains("readonly v: unknown;"), "{text}");
    }
}
