//! `cargo xtask gen privacy`: the personal-data map as a page, from the marks in the code
//! (ADR-0015, PRIV-02).
//!
//! The marks live in the entity definitions (`#[derive(Entity)]`, `#[personal(…)]`,
//! `#[not_personal]`). This module reads the Rust source, so xtask does not depend on the product
//! crates. DATA-MARKS uses the same reader to check the marks before the compiler does.

use std::fmt::Write as _;

use anyhow::{Context as _, bail};
use syn::{Attribute, Expr, Fields, Item, Lit, Meta};

use super::{Generated, write_all};
use crate::repo::Repo;
use crate::rust_source::{Source, read_product_sources};

/// The generated page, relative to the repository root.
pub(crate) const OUTPUT: &str = "docs/privacy/personal-data.md";

/// Where the words of the marks are defined (`Category` and `Basis`).
pub(crate) const VOCABULARY_PATH: &str = "crates/bb-core/src/personal.rs";

/// Where all entities are listed (`entities! { … }`).
pub(crate) const LIST_PATH: &str = "crates/bb-core/src/entities.rs";

/// What a field says about itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    Personal { category: String, basis: String },
    NotPersonal,
}

#[derive(Clone, Debug)]
pub(crate) struct Field {
    pub(crate) name: String,
    /// The mark, or what is wrong with the marks written on the field.
    pub(crate) mark: Result<Mark, String>,
    pub(crate) doc: String,
}

#[derive(Clone, Debug)]
pub(crate) struct EntityDef {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) doc: String,
    pub(crate) fields: Vec<Field>,
}

/// A word that may be used in a mark, with its explanation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Word {
    pub(crate) code: String,
    pub(crate) doc: String,
}

/// The words `#[personal(category = …, basis = …)]` accepts.
#[derive(Clone, Debug, Default)]
pub(crate) struct Vocabulary {
    pub(crate) categories: Vec<Word>,
    pub(crate) bases: Vec<Word>,
}

/// Everything read from the source.
pub(crate) struct Model {
    pub(crate) entities: Vec<EntityDef>,
    /// The names in the `entities!` list.
    pub(crate) listed: Vec<String>,
    pub(crate) vocabulary: Vocabulary,
}

pub(crate) fn run(repo: &Repo) -> anyhow::Result<Vec<String>> {
    write_all(repo, &generate(repo)?)
}

pub(crate) fn generate(repo: &Repo) -> anyhow::Result<Vec<Generated>> {
    let sources = read_product_sources(repo)?;
    let model = read_model(&sources)?;
    require_vocabulary(&model)?;
    Ok(vec![Generated::new(OUTPUT, render(&model))])
}

pub(crate) fn read_model(sources: &[Source]) -> anyhow::Result<Model> {
    let mut entities = Vec::new();
    let mut listed = Vec::new();
    let mut vocabulary = Vocabulary::default();
    for (path, text) in sources {
        let file = syn::parse_file(text).with_context(|| format!("{path} is not valid Rust"))?;
        visit(&file.items, path, &mut entities);
        if path == VOCABULARY_PATH {
            vocabulary = read_vocabulary(&file.items);
        }
        if path == LIST_PATH {
            listed = read_list(&file.items);
        }
    }
    entities.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.path.cmp(&b.path)));
    Ok(Model {
        entities,
        listed,
        vocabulary,
    })
}

fn visit(items: &[Item], path: &str, entities: &mut Vec<EntityDef>) {
    for item in items {
        match item {
            Item::Struct(item) if derives_entity(&item.attrs) => {
                let fields = match &item.fields {
                    Fields::Named(named) => named.named.iter().map(read_field).collect(),
                    Fields::Unnamed(_) | Fields::Unit => Vec::new(),
                };
                entities.push(EntityDef {
                    name: item.ident.to_string(),
                    path: path.to_owned(),
                    doc: doc(&item.attrs),
                    fields,
                });
            }
            Item::Mod(module) if !is_test_only(&module.attrs) => {
                if let Some((_, inner)) = &module.content {
                    visit(inner, path, entities);
                }
            }
            _ => {}
        }
    }
}

fn derives_entity(attrs: &[Attribute]) -> bool {
    let mut found = false;
    for attr in attrs.iter().filter(|a| a.path().is_ident("derive")) {
        let _ = attr.parse_nested_meta(|meta| {
            found |= meta
                .path
                .segments
                .last()
                .is_some_and(|s| s.ident == "Entity");
            Ok(())
        });
    }
    found
}

fn is_test_only(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .parse_args::<syn::Ident>()
                .is_ok_and(|ident| ident == "test")
    })
}

fn read_field(field: &syn::Field) -> Field {
    let name = field
        .ident
        .as_ref()
        .map_or_else(String::new, ToString::to_string);
    let mut marks = Vec::new();
    for attr in &field.attrs {
        if attr.path().is_ident("not_personal") {
            marks.push(Ok(Mark::NotPersonal));
        } else if attr.path().is_ident("personal") {
            marks.push(read_personal(attr));
        }
    }
    let mark = match marks.len() {
        0 => Err(
            "has no mark; add #[personal(category = \"…\", basis = \"…\")] or #[not_personal]"
                .to_owned(),
        ),
        1 => marks.remove(0),
        _ => Err("has more than one mark; keep exactly one".to_owned()),
    };
    Field {
        name,
        mark,
        doc: doc(&field.attrs),
    }
}

fn read_personal(attr: &Attribute) -> Result<Mark, String> {
    let mut category = None;
    let mut basis = None;
    attr.parse_nested_meta(|meta| {
        let value: syn::LitStr = meta.value()?.parse()?;
        if meta.path.is_ident("category") {
            category = Some(value.value());
        } else if meta.path.is_ident("basis") {
            basis = Some(value.value());
        }
        Ok(())
    })
    .map_err(|error| format!("has a mark that is not understood: {error}"))?;
    match (category, basis) {
        (Some(category), Some(basis)) => Ok(Mark::Personal { category, basis }),
        _ => Err("needs both category = \"…\" and basis = \"…\" in #[personal(…)]".to_owned()),
    }
}

/// The doc comment of an item as one line.
fn doc(attrs: &[Attribute]) -> String {
    let lines: Vec<String> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| match &attr.meta {
            Meta::NameValue(pair) => match &pair.value {
                Expr::Lit(literal) => match &literal.lit {
                    Lit::Str(text) => Some(text.value().trim().to_owned()),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        })
        .collect();
    lines.join(" ")
}

fn read_vocabulary(items: &[Item]) -> Vocabulary {
    let mut vocabulary = Vocabulary::default();
    for item in items {
        if let Item::Enum(item) = item {
            let words: Vec<Word> = item
                .variants
                .iter()
                .map(|variant| Word {
                    code: snake_case(&variant.ident.to_string()),
                    doc: doc(&variant.attrs),
                })
                .collect();
            match item.ident.to_string().as_str() {
                "Category" => vocabulary.categories = words,
                "Basis" => vocabulary.bases = words,
                _ => {}
            }
        }
    }
    vocabulary
}

/// The entity names in `entities! { crate::event::DomainEvent, … }`: the last part of each path.
fn read_list(items: &[Item]) -> Vec<String> {
    let mut names = Vec::new();
    for item in items {
        let Item::Macro(item) = item else { continue };
        if item
            .mac
            .path
            .segments
            .last()
            .is_none_or(|s| s.ident != "entities")
        {
            continue;
        }
        let paths = item.mac.parse_body_with(
            syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
        );
        if let Ok(paths) = paths {
            names.extend(
                paths
                    .iter()
                    .filter_map(|path| path.segments.last())
                    .map(|segment| segment.ident.to_string()),
            );
        }
    }
    names
}

/// `EmployeeTime` becomes `employee_time`, the way the derive macro reads the words.
fn snake_case(name: &str) -> String {
    let mut out = String::new();
    for (index, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if index > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// The page. Tables are padded so the Markdown formatter leaves them alone.
pub(crate) fn render(model: &Model) -> String {
    let fields: usize = model.entities.iter().map(|e| e.fields.len()).sum();
    let personal: usize = model
        .entities
        .iter()
        .flat_map(|e| &e.fields)
        .filter(|f| matches!(f.mark, Ok(Mark::Personal { .. })))
        .count();
    let mut out = String::new();
    out.push_str("# Personal data in Bugaboo\n\n");
    out.push_str("GENERATED by `cargo xtask gen privacy` from the personal-data marks in the code (ADR-0015). Do not edit this page by hand.\n\n");
    out.push_str("Every stored field says whether it holds personal data. This page lists the answers. The same marks drive *Download my data*, *Remove a person*, the keep-and-delete rules and the record of processing (GDPR Art. 30).\n\n");
    let _ = writeln!(
        out,
        "**{} entities, {fields} fields: {personal} personal, {} not personal.**\n",
        model.entities.len(),
        fields - personal
    );
    out.push_str("## Categories\n\n");
    out.push_str(&table(
        &["Category", "What it covers"],
        &model
            .vocabulary
            .categories
            .iter()
            .map(word_row)
            .collect::<Vec<_>>(),
    ));
    out.push_str("\n## Legal bases\n\n");
    out.push_str(&table(
        &["Legal basis", "What it means"],
        &model
            .vocabulary
            .bases
            .iter()
            .map(word_row)
            .collect::<Vec<_>>(),
    ));
    out.push_str("\n## Entities\n");
    for entity in &model.entities {
        let _ = write!(out, "\n### {}\n\n", entity.name);
        if !entity.doc.is_empty() {
            let _ = writeln!(out, "{}\n", entity.doc);
        }
        let _ = writeln!(out, "Defined in `{}`.\n", entity.path);
        let rows: Vec<Vec<String>> = entity.fields.iter().map(field_row).collect();
        out.push_str(&table(
            &[
                "Field",
                "Personal data",
                "Category",
                "Legal basis",
                "What it holds",
            ],
            &rows,
        ));
    }
    out
}

fn word_row(word: &Word) -> Vec<String> {
    vec![format!("`{}`", word.code), word.doc.clone()]
}

fn field_row(field: &Field) -> Vec<String> {
    let (personal, category, basis) = match &field.mark {
        Ok(Mark::Personal { category, basis }) => (
            "Yes".to_owned(),
            format!("`{category}`"),
            format!("`{basis}`"),
        ),
        Ok(Mark::NotPersonal) => ("No".to_owned(), "–".to_owned(), "–".to_owned()),
        Err(problem) => ("?".to_owned(), "–".to_owned(), problem.clone()),
    };
    vec![
        format!("`{}`", field.name),
        personal,
        category,
        basis,
        field.doc.clone(),
    ]
}

/// A Markdown table with padded columns.
fn table(header: &[&str], rows: &[Vec<String>]) -> String {
    let escape = |cell: &str| cell.replace('|', "\\|");
    let mut all: Vec<Vec<String>> = vec![header.iter().map(|h| (*h).to_owned()).collect()];
    all.extend(
        rows.iter()
            .map(|row| row.iter().map(|c| escape(c)).collect()),
    );
    let widths: Vec<usize> = (0..header.len())
        .map(|column| {
            all.iter()
                .map(|row| row[column].chars().count())
                .max()
                .unwrap_or(3)
                .max(3)
        })
        .collect();
    let line = |cells: &[String]| {
        let padded: Vec<String> = cells
            .iter()
            .zip(&widths)
            .map(|(cell, width)| format!("{cell:<width$}"))
            .collect();
        format!("| {} |\n", padded.join(" | "))
    };
    let mut out = line(&all[0]);
    let dashes: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    out.push_str(&line(&dashes));
    for row in &all[1..] {
        out.push_str(&line(row));
    }
    out
}

/// Fails with a plain message when the vocabulary could not be read.
pub(crate) fn require_vocabulary(model: &Model) -> anyhow::Result<()> {
    if model.vocabulary.categories.is_empty() || model.vocabulary.bases.is_empty() {
        bail!("could not read the categories and legal bases from {VOCABULARY_PATH}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VOCABULARY: &str = "pub enum Category { /// Who.\n Identity, EmployeeTime }\npub enum Basis { /// A contract.\n Contract }\n";

    fn model(entity: &str) -> Model {
        read_model(&[
            (VOCABULARY_PATH.to_owned(), VOCABULARY.to_owned()),
            ("crates/bb-core/src/person.rs".to_owned(), entity.to_owned()),
        ])
        .unwrap()
    }

    #[test]
    fn marks_and_docs_are_read_from_the_source() {
        let model = model(
            "/// A person.\n#[derive(Clone, Entity)]\npub struct Person {\n    /// The ID.\n    #[not_personal]\n    pub id: u32,\n    /// The address.\n    #[personal(category = \"identity\", basis = \"contract\")]\n    pub email: String,\n}\n",
        );
        assert_eq!(model.entities.len(), 1);
        let person = &model.entities[0];
        assert_eq!(person.doc, "A person.");
        assert_eq!(person.fields[0].mark, Ok(Mark::NotPersonal));
        assert_eq!(
            person.fields[1].mark,
            Ok(Mark::Personal {
                category: "identity".into(),
                basis: "contract".into()
            })
        );
        assert_eq!(model.vocabulary.categories[1].code, "employee_time");
        assert_eq!(model.vocabulary.bases[0].doc, "A contract.");
    }

    #[test]
    fn unmarked_and_doubly_marked_fields_are_reported_not_hidden() {
        let model = model(
            "#[derive(Entity)]\npub struct P {\n    pub a: u32,\n    #[not_personal]\n    #[personal(category = \"identity\", basis = \"contract\")]\n    pub b: u32,\n}\n",
        );
        assert!(model.entities[0].fields[0].mark.is_err());
        assert!(model.entities[0].fields[1].mark.is_err());
    }

    #[test]
    fn structs_without_the_derive_and_test_modules_are_not_entities() {
        let model = model(
            "pub struct Plain { pub a: u32 }\n#[cfg(test)]\nmod tests {\n    #[derive(Entity)]\n    struct T { #[not_personal] a: u32 }\n}\n",
        );
        assert!(model.entities.is_empty());
    }

    #[test]
    fn the_list_names_the_last_part_of_each_path() {
        let model = read_model(&[(
            LIST_PATH.to_owned(),
            "crate::entities! { crate::event::DomainEvent, crate::person::Person }".to_owned(),
        )])
        .unwrap();
        assert_eq!(model.listed, ["DomainEvent", "Person"]);
    }

    #[test]
    fn the_page_counts_and_lists_every_field() {
        let page = render(&model(
            "#[derive(Entity)]\npub struct P {\n    #[not_personal]\n    pub a: u32,\n    #[personal(category = \"identity\", basis = \"contract\")]\n    pub b: u32,\n}\n",
        ));
        assert!(
            page.contains("**1 entities, 2 fields: 1 personal, 1 not personal.**"),
            "{page}"
        );
        assert!(page.contains("| `b`"), "{page}");
        assert!(page.contains("`identity`"), "{page}");
    }

    #[test]
    fn the_generated_page_is_up_to_date() {
        let repo = Repo::discover().unwrap();
        for output in generate(&repo).unwrap() {
            let on_disk = std::fs::read_to_string(repo.path(output.path)).unwrap();
            assert_eq!(
                on_disk.replace("\r\n", "\n"),
                output.content,
                "{} is stale; run `cargo xtask gen privacy`",
                output.path
            );
        }
    }

    #[test]
    fn camel_case_words_become_snake_case() {
        assert_eq!(snake_case("EmployeeTime"), "employee_time");
        assert_eq!(snake_case("Identity"), "identity");
    }
}
