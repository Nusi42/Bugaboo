//! DATA-MARKS: every stored field is marked as personal data or not (ADR-0015, PRIV-02).
//!
//! The derive macro `#[derive(Entity)]` already stops the build for an unmarked field. This check
//! reads the same marks from the source, so the mistake shows up right after the edit, and it
//! checks what the compiler cannot: the words of the marks, and that every entity is named in the
//! `entities!` list that the personal-data map is built from. The counts of marked fields are on
//! the generated page `docs/privacy/personal-data.md` (`cargo xtask gen privacy`).
//!
//! A table is stored data too: every table of `bb-store` (an enum with a `Table` variant in
//! `schema.rs`) must be named in the `stored_tables!` list with the entity it holds, and that
//! entity must carry marks and be in the `entities!` list. A table that is created in a migration
//! without being listed is found as well (ADR-0015, ADR-0003).

use std::collections::BTreeSet;

use super::{Check, Context, Finding};
use crate::generate::privacy::{self, Mark, Model};
use crate::rust_source::read_product_sources;

pub(crate) struct DataMarks;

impl Check for DataMarks {
    fn id(&self) -> &'static str {
        "DATA-MARKS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let sources = read_product_sources(context.repo)?;
        let model = privacy::read_model(&sources)?;
        let mut findings = evaluate(&model);
        findings.extend(evaluate_tables(&model, &sources));
        Ok(findings)
    }
}

/// Where the tables are named.
const SCHEMA_PATH: &str = "crates/bb-store/src/schema.rs";
/// Where the tables are created.
const MIGRATIONS_FOLDER: &str = "crates/bb-store/src/migrations";

/// The `Table => Entity` lines of the `stored_tables!` list: the table's enum and the entity's
/// name (the last part of its path).
fn listed_tables(schema: &str) -> Vec<(String, String)> {
    let Some(start) = schema.find("stored_tables! {\n") else {
        return Vec::new();
    };
    schema[start..]
        .lines()
        .skip(1)
        .take_while(|line| !line.starts_with('}'))
        .filter_map(|line| {
            let (table, entity) = line.split_once("=>")?;
            let entity = entity.trim().trim_end_matches(',');
            Some((
                table.trim().to_owned(),
                entity.rsplit("::").next()?.trim().to_owned(),
            ))
        })
        .collect()
}

/// The enums of the schema that stand for a table: they have a `Table` variant.
fn table_enums(schema: &str) -> Vec<String> {
    let before_macro = schema.split("macro_rules!").next().unwrap_or(schema);
    let mut enums = Vec::new();
    let mut current: Option<String> = None;
    for line in before_macro.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("pub(crate) enum ") {
            current = rest.split_whitespace().next().map(str::to_owned);
        } else if line == "Table," {
            enums.extend(current.take());
        }
    }
    enums
}

/// The tables that migrations create: the name inside `.table(Name::Table)` after
/// `Table::create()`.
fn created_tables(text: &str) -> Vec<String> {
    let mut tables = Vec::new();
    for (at, _) in text.match_indices("Table::create()") {
        let rest = &text[at..];
        if let Some(open) = rest.find(".table(") {
            let path: String = rest[open + ".table(".len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | ':'))
                .collect();
            // `Event::Table` or `schema::Event::Table`: the table is the part before `::Table`.
            let name = path.strip_suffix("::Table").unwrap_or(&path);
            tables.extend(name.rsplit("::").next().map(str::to_owned));
        }
    }
    tables
}

fn evaluate_tables(model: &Model, sources: &[crate::rust_source::Source]) -> Vec<Finding> {
    let Some((_, schema)) = sources.iter().find(|(path, _)| path == SCHEMA_PATH) else {
        return Vec::new();
    };
    let listed = listed_tables(schema);
    let entities: BTreeSet<&str> = model.entities.iter().map(|e| e.name.as_str()).collect();
    let in_list: BTreeSet<&str> = model.listed.iter().map(String::as_str).collect();
    let mut findings = Vec::new();
    for table in table_enums(schema) {
        if !listed.iter().any(|(name, _)| *name == table) {
            findings.push(Finding::at(
                SCHEMA_PATH,
                format!(
                    "The table `{table}` has no entity. Name it in `stored_tables!` with the \
                     struct that holds its rows; that struct derives `Entity` and carries the \
                     personal-data marks (ADR-0015)."
                ),
            ));
        }
    }
    for (table, entity) in &listed {
        if !entities.contains(entity.as_str()) {
            findings.push(Finding::at(
                SCHEMA_PATH,
                format!(
                    "The table `{table}` stores `{entity}`, but no struct with that name derives \
                     `Entity`, so its fields have no personal-data marks."
                ),
            ));
        } else if !in_list.contains(entity.as_str()) {
            findings.push(Finding::at(
                SCHEMA_PATH,
                format!(
                    "The table `{table}` stores `{entity}`, which is not in the `entities!` \
                     list in {}.",
                    privacy::LIST_PATH
                ),
            ));
        }
    }
    for (path, text) in sources
        .iter()
        .filter(|(path, _)| path.starts_with(MIGRATIONS_FOLDER))
    {
        for table in created_tables(text) {
            if !listed.iter().any(|(name, _)| *name == table) {
                findings.push(Finding::at(
                    path,
                    format!(
                        "A migration creates the table `{table}`, which is not in \
                         `stored_tables!` in {SCHEMA_PATH}, so its rows have no personal-data marks."
                    ),
                ));
            }
        }
    }
    findings
}

fn evaluate(model: &Model) -> Vec<Finding> {
    let mut findings = Vec::new();
    if privacy::require_vocabulary(model).is_err() {
        findings.push(Finding::at(
            privacy::VOCABULARY_PATH,
            "The enums `Category` and `Basis` are missing; the marks have no words to use.",
        ));
        return findings;
    }
    let categories: BTreeSet<&str> = model
        .vocabulary
        .categories
        .iter()
        .map(|w| w.code.as_str())
        .collect();
    let bases: BTreeSet<&str> = model
        .vocabulary
        .bases
        .iter()
        .map(|w| w.code.as_str())
        .collect();
    let listed: BTreeSet<&str> = model.listed.iter().map(String::as_str).collect();
    let mut names = BTreeSet::new();
    for entity in &model.entities {
        if !names.insert(entity.name.as_str()) {
            findings.push(Finding::at(
                &entity.path,
                format!(
                    "Another entity is also called `{}`; give each entity its own name.",
                    entity.name
                ),
            ));
        }
        if !listed.contains(entity.name.as_str()) {
            findings.push(Finding::at(
                &entity.path,
                format!(
                    "`{}` is not in the `entities!` list in {}, so the personal-data map would miss it.",
                    entity.name,
                    privacy::LIST_PATH
                ),
            ));
        }
        for field in &entity.fields {
            let place = format!("{}: {}.{}", entity.path, entity.name, field.name);
            match &field.mark {
                Err(problem) => findings.push(Finding::at(place, format!("The field {problem}."))),
                Ok(Mark::NotPersonal) => {}
                Ok(Mark::Personal { category, basis }) => {
                    if !categories.contains(category.as_str()) {
                        findings.push(Finding::at(
                            place.clone(),
                            format!(
                                "`{category}` is not a category; use one of {}.",
                                join(&categories)
                            ),
                        ));
                    }
                    if !bases.contains(basis.as_str()) {
                        findings.push(Finding::at(
                            place,
                            format!(
                                "`{basis}` is not a legal basis; use one of {}.",
                                join(&bases)
                            ),
                        ));
                    }
                }
            }
        }
    }
    let known: BTreeSet<&str> = model.entities.iter().map(|e| e.name.as_str()).collect();
    for name in listed.difference(&known) {
        findings.push(Finding::at(
            privacy::LIST_PATH,
            format!(
                "`{name}` is in the `entities!` list but no struct with that name derives `Entity`."
            ),
        ));
    }
    findings
}

fn join(words: &BTreeSet<&str>) -> String {
    words.iter().copied().collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rust_source::Source;

    const VOCABULARY: &str = "words! { pub enum Category { Identity => \"identity\", EmployeeTime => \"employee_time\" } }
words! { pub enum Basis { Contract => \"contract\", LegalObligation => \"legal_obligation\" } }
";
    const LIST: &str = "crate::entities! { crate::person::Person }";

    fn findings(person: &str, list: &str) -> Vec<Finding> {
        let sources: Vec<Source> = vec![
            (privacy::VOCABULARY_PATH.to_owned(), VOCABULARY.to_owned()),
            (privacy::LIST_PATH.to_owned(), list.to_owned()),
            ("crates/bb-core/src/person.rs".to_owned(), person.to_owned()),
        ];
        evaluate(&privacy::read_model(&sources).unwrap())
    }

    #[test]
    fn good_example() {
        let person = "#[derive(Entity)]\npub struct Person {\n    #[not_personal]\n    pub id: u32,\n    #[personal(category = \"identity\", basis = \"contract\")]\n    pub email: String,\n}\n";
        assert!(findings(person, LIST).is_empty());
    }

    #[test]
    fn bad_example() {
        let person = "#[derive(Entity)]\npub struct Person {\n    pub id: u32,\n    #[personal(category = \"secrets\", basis = \"whim\")]\n    pub email: String,\n}\n#[derive(Entity)]\npub struct Forgotten {\n    #[not_personal]\n    pub id: u32,\n}\n";
        let messages: Vec<String> = findings(
            person,
            "crate::entities! { crate::person::Person, crate::x::Ghost }",
        )
        .into_iter()
        .map(|f| f.message)
        .collect();
        let all = messages.join("\n");
        assert!(all.contains("has no mark"), "{all}");
        assert!(all.contains("`secrets` is not a category"), "{all}");
        assert!(all.contains("`whim` is not a legal basis"), "{all}");
        assert!(
            all.contains("`Forgotten` is not in the `entities!` list"),
            "{all}"
        );
        assert!(all.contains("`Ghost` is in the `entities!` list"), "{all}");
    }

    const SCHEMA_GOOD: &str = "pub(crate) enum Event {\n    Table,\n    Id,\n}\nmacro_rules! stored_tables {\n    () => {};\n}\nstored_tables! {\n    Event => bb_core::event::DomainEvent,\n}\n";
    const LIST_GOOD: &str = "crate::entities! { crate::event::DomainEvent }";
    const MIGRATION_GOOD: &str = "Table::create()\n    .table(Event::Table)\n    .col(x)\n";

    fn table_findings(schema: &str, migration: &str, list: &str) -> String {
        let entity = "#[derive(Entity)]\npub struct DomainEvent {\n    #[not_personal]\n    pub id: u32,\n}\n";
        let mut sources: Vec<Source> = vec![
            (privacy::VOCABULARY_PATH.to_owned(), VOCABULARY.to_owned()),
            (privacy::LIST_PATH.to_owned(), list.to_owned()),
            ("crates/bb-core/src/event.rs".to_owned(), entity.to_owned()),
        ];
        let model = privacy::read_model(&sources).unwrap();
        sources.push((SCHEMA_PATH.to_owned(), schema.to_owned()));
        sources.push((
            format!("{MIGRATIONS_FOLDER}/m0001.rs"),
            migration.to_owned(),
        ));
        evaluate_tables(&model, &sources)
            .into_iter()
            .map(|f| f.message)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn good_example_every_table_belongs_to_an_entity_with_marks() {
        assert_eq!(table_findings(SCHEMA_GOOD, MIGRATION_GOOD, LIST_GOOD), "");
    }

    #[test]
    fn bad_example_a_table_without_an_entity() {
        let schema = format!("pub(crate) enum Orphan {{\n    Table,\n}}\n{SCHEMA_GOOD}");
        let migration = format!("{MIGRATION_GOOD}Table::create().table(Orphan::Table)\n");
        let all = table_findings(&schema, &migration, LIST_GOOD);
        assert!(all.contains("The table `Orphan` has no entity"), "{all}");
        assert!(all.contains("creates the table `Orphan`"), "{all}");
    }

    #[test]
    fn bad_example_a_table_whose_entity_has_no_marks_or_is_not_listed() {
        let ghost = SCHEMA_GOOD.replace("DomainEvent", "Ghost");
        let all = table_findings(&ghost, MIGRATION_GOOD, LIST_GOOD);
        assert!(
            all.contains("`Ghost`, but no struct with that name derives"),
            "{all}"
        );
        let unlisted = table_findings(
            SCHEMA_GOOD,
            MIGRATION_GOOD,
            "crate::entities! { crate::x::Y }",
        );
        assert!(
            unlisted.contains("not in the `entities!` list"),
            "{unlisted}"
        );
    }

    #[test]
    fn the_entities_of_this_repository_pass() {
        let repo = crate::repo::Repo::discover().unwrap();
        let sources = read_product_sources(&repo).unwrap();
        let model = privacy::read_model(&sources).unwrap();
        let mut findings = evaluate(&model);
        findings.extend(evaluate_tables(&model, &sources));
        assert!(
            findings.is_empty(),
            "{:?}",
            findings.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }
}
