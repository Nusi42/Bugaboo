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

use std::collections::{BTreeMap, BTreeSet};

use super::{Check, Context, Finding};
use crate::generate::privacy::{self, Mark, Model};
use crate::rust_source::strip_comments;
use crate::source::Kind;

pub(crate) struct DataMarks;

impl Check for DataMarks {
    fn id(&self) -> &'static str {
        "DATA-MARKS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let sources = context.sources(self.id(), Kind::Rust)?;
        let model = privacy::read_model(&sources)?;
        let mut findings = evaluate(&model);
        findings.extend(evaluate_tables(&model, &sources));
        Ok(findings)
    }
}

/// Where the tables are named: the file with the list `stored_tables!`, and the files beside it
/// that hold the enums of the tables by area.
const SCHEMA_PATH: &str = "crates/bb-store/src/schema.rs";
const SCHEMA_FOLDER: &str = "crates/bb-store/src/schema/";
/// Where the tables are created.
const MIGRATIONS_FOLDER: &str = "crates/bb-store/src/migrations";

/// One entry of the `stored_tables!` list: a table, the entity stored in it, and the field behind
/// each of its columns.
struct ListedTable {
    table: String,
    entity: String,
    columns: Vec<(String, String)>,
}

/// The entries of the `stored_tables!` list:
///
/// ```text
/// Table => path::Entity {
///     Column => field,
/// },
/// ```
fn listed_tables(schema: &str) -> Vec<ListedTable> {
    let Some(start) = schema.find("stored_tables! {\n") else {
        return Vec::new();
    };
    let mut tables: Vec<ListedTable> = Vec::new();
    let mut open = false;
    for line in schema[start..].lines().skip(1) {
        let line = line.trim();
        if line.starts_with('}') && !open {
            break;
        }
        if open {
            if line.starts_with('}') {
                open = false;
                continue;
            }
            for pair in line.split(',') {
                if let (Some((column, field)), Some(table)) =
                    (pair.split_once("=>"), tables.last_mut())
                {
                    table
                        .columns
                        .push((column.trim().to_owned(), field.trim().to_owned()));
                }
            }
        } else if let Some((table, rest)) = line.split_once("=>") {
            let entity = rest.trim().trim_end_matches('{').trim();
            tables.push(ListedTable {
                table: table.trim().to_owned(),
                entity: entity
                    .rsplit("::")
                    .next()
                    .unwrap_or(entity)
                    .trim()
                    .to_owned(),
                columns: Vec::new(),
            });
            open = rest.trim_end().ends_with('{');
        }
    }
    tables
}

/// The enums of the schema that stand for a table (they have a `Table` variant) with their other
/// variants, which are its columns.
fn table_enums(schema: &str) -> Vec<(String, Vec<String>)> {
    let schema = strip_comments(schema);
    let before_macro = schema.split("macro_rules!").next().unwrap_or(&schema);
    let mut enums = Vec::new();
    let mut current: Option<(String, Vec<String>, bool)> = None;
    for line in before_macro.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("pub(crate) enum ") {
            let name = rest
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_owned();
            current = Some((name, Vec::new(), false));
        } else if line.starts_with('}') {
            if let Some((name, columns, is_table)) = current.take()
                && is_table
            {
                enums.push((name, columns));
            }
        } else if let Some((_, columns, is_table)) = current.as_mut()
            && !line.starts_with('#')
            && let Some(variant) = line.strip_suffix(',')
        {
            if variant == "Table" {
                *is_table = true;
            } else {
                columns.push(variant.to_owned());
            }
        }
    }
    enums
}

/// A table that a migration creates.
#[derive(Debug, PartialEq, Eq)]
enum Created {
    /// By the enum of the schema (`.table(Event::Table)`).
    Schema(String),
    /// By its name (`.table(Alias::new("issue_assignee"))`), the way an old migration keeps the name
    /// of a table that a later migration renamed.
    Named(String),
}

/// The name that `Alias::new` is given: a string, or a constant of the file that holds one.
/// `argument` starts at the `(` after `Alias::new`.
fn alias_name(text: &str, argument: &str) -> Option<String> {
    let rest = argument.strip_prefix('(')?.trim_start();
    if let Some(literal) = rest.strip_prefix('"') {
        return literal.split('"').next().map(str::to_owned);
    }
    let constant: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    let declared = text.find(&format!("const {constant}: &str = \""))?;
    text[declared..].split('"').nth(1).map(str::to_owned)
}

/// The tables that migrations create: the name inside `.table(Name::Table)` after
/// `Table::create()`, or the name that `Alias::new` is given.
fn created_tables(text: &str) -> Vec<Created> {
    let mut tables = Vec::new();
    for (at, _) in text.match_indices("Table::create()") {
        let rest = &text[at..];
        if let Some(open) = rest.find(".table(") {
            let after = &rest[open + ".table(".len()..];
            if let Some(name) = after
                .strip_prefix("Alias::new")
                .and_then(|argument| alias_name(text, argument))
            {
                tables.push(Created::Named(name));
                continue;
            }
            let path: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | ':'))
                .collect();
            // `Event::Table` or `schema::Event::Table`: the table is the part before `::Table`.
            let name = path.strip_suffix("::Table").unwrap_or(&path);
            tables.extend(
                name.rsplit("::")
                    .next()
                    .map(|name| Created::Schema(name.to_owned())),
            );
        }
    }
    tables
}

/// The names of the tables that migrations rename (`Table::rename().table(Alias::new("old"), …)`).
fn renamed_tables(text: &str) -> Vec<String> {
    text.match_indices("Table::rename()")
        .filter_map(|(at, _)| {
            let rest = &text[at..];
            let open = rest.find(".table(Alias::new")?;
            alias_name(text, &rest[open + ".table(Alias::new".len()..])
        })
        .collect()
}

/// The tables the migrations create without the marks of an entity behind them.
fn migration_findings(listed: &[ListedTable], sources: &[crate::source::Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let renamed: Vec<String> = sources
        .iter()
        .filter(|(path, _)| path.starts_with(MIGRATIONS_FOLDER))
        .flat_map(|(_, text)| renamed_tables(text))
        .collect();
    for (path, text) in sources
        .iter()
        .filter(|(path, _)| path.starts_with(MIGRATIONS_FOLDER))
    {
        for created in created_tables(text) {
            match created {
                Created::Schema(table) if !listed.iter().any(|entry| entry.table == table) => {
                    findings.push(Finding::at(
                        path,
                        format!(
                            "A migration creates the table `{table}`, which is not in \
                             `stored_tables!` in {SCHEMA_PATH}, so its rows have no personal-data marks."
                        ),
                    ));
                }
                Created::Named(name) if !renamed.contains(&name) => {
                    findings.push(Finding::at(
                        path,
                        format!(
                            "A migration creates the table `{name}` by its name, so its rows have \
                             no personal-data marks. Create it from the schema, or rename it in a \
                             later migration to a table of `stored_tables!` in {SCHEMA_PATH}."
                        ),
                    ));
                }
                _ => {}
            }
        }
    }
    findings
}

fn evaluate_tables(model: &Model, sources: &[crate::source::Source]) -> Vec<Finding> {
    if !sources.iter().any(|(path, _)| path == SCHEMA_PATH) {
        return Vec::new();
    }
    let schema = sources
        .iter()
        .filter(|(path, _)| path == SCHEMA_PATH || path.starts_with(SCHEMA_FOLDER))
        .map(|(_, text)| text.as_str())
        .collect::<Vec<_>>()
        .join(
            "
",
        );
    let schema = schema.as_str();
    let listed = listed_tables(schema);
    let entities: BTreeMap<&str, &Vec<privacy::Field>> = model
        .entities
        .iter()
        .map(|entity| (entity.name.as_str(), &entity.fields))
        .collect();
    let in_list: BTreeSet<&str> = model.listed.iter().map(String::as_str).collect();
    let mut findings = Vec::new();
    for (table, variants) in table_enums(schema) {
        let Some(entry) = listed.iter().find(|entry| entry.table == table) else {
            findings.push(Finding::at(
                SCHEMA_PATH,
                format!(
                    "The table `{table}` has no entity. Name it in `stored_tables!` with the \
                     struct that holds its rows; that struct derives `Entity` and carries the \
                     personal-data marks (ADR-0015)."
                ),
            ));
            continue;
        };
        for variant in &variants {
            if !entry.columns.iter().any(|(column, _)| column == variant) {
                findings.push(Finding::at(
                    SCHEMA_PATH,
                    format!(
                        "The column `{table}::{variant}` is not covered by a marked field. Add \
                         `{variant} => field` to its entry in `stored_tables!`; the field of \
                         `{}` carries the personal-data mark (ADR-0015).",
                        entry.entity
                    ),
                ));
            }
        }
        for (column, field) in &entry.columns {
            if !variants.contains(column) {
                findings.push(Finding::at(
                    SCHEMA_PATH,
                    format!("`stored_tables!` names the column `{table}::{column}`, which the table has not."),
                ));
            }
            if let Some(fields) = entities.get(entry.entity.as_str())
                && !fields.iter().any(|candidate| candidate.name == *field)
            {
                findings.push(Finding::at(
                    SCHEMA_PATH,
                    format!(
                        "The column `{table}::{column}` points to `{field}`, which `{}` has no \
                         field called.",
                        entry.entity
                    ),
                ));
            }
        }
    }
    for entry in &listed {
        let (table, entity) = (&entry.table, &entry.entity);
        if !entities.contains_key(entity.as_str()) {
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
    findings.extend(migration_findings(&listed, sources));
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
    use crate::source::Source;

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

    const SCHEMA_GOOD: &str = "pub(crate) enum Event {\n    Table,\n    Id,\n    Time,\n}\nmacro_rules! stored_tables {\n    () => {};\n}\nstored_tables! {\n    Event => bb_core::event::DomainEvent {\n        Id => id,\n        Time => id,\n    },\n}\n";
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
    fn good_example_the_enums_of_the_tables_may_be_in_files_beside_the_list() {
        let (enums, list) = SCHEMA_GOOD.split_at(SCHEMA_GOOD.find("macro_rules!").unwrap());
        let entity = "#[derive(Entity)]
pub struct DomainEvent {
    #[not_personal]
    pub id: u32,
}
";
        let mut sources: Vec<Source> = vec![
            (privacy::VOCABULARY_PATH.to_owned(), VOCABULARY.to_owned()),
            (privacy::LIST_PATH.to_owned(), LIST_GOOD.to_owned()),
            ("crates/bb-core/src/event.rs".to_owned(), entity.to_owned()),
        ];
        let model = privacy::read_model(&sources).unwrap();
        sources.push((SCHEMA_PATH.to_owned(), list.to_owned()));
        sources.push((format!("{SCHEMA_FOLDER}system.rs"), enums.to_owned()));
        sources.push((
            format!("{MIGRATIONS_FOLDER}/m0001.rs"),
            MIGRATION_GOOD.to_owned(),
        ));
        let found: Vec<String> = evaluate_tables(&model, &sources)
            .into_iter()
            .map(|finding| finding.message)
            .collect();
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn good_example_a_table_made_by_its_old_name_is_renamed_to_one_with_marks() {
        let migration = format!(
            "{MIGRATION_GOOD}Table::create().table(Alias::new(OLD)).col(x)\nconst OLD: &str = \"old_event\";\nTable::rename().table(Alias::new(\"old_event\"), Event::Table)\n"
        );
        assert_eq!(table_findings(SCHEMA_GOOD, &migration, LIST_GOOD), "");
    }

    #[test]
    fn bad_example_a_table_made_by_its_name_and_never_renamed() {
        let migration =
            format!("{MIGRATION_GOOD}Table::create().table(Alias::new(\"hidden_rows\")).col(x)\n");
        let all = table_findings(SCHEMA_GOOD, &migration, LIST_GOOD);
        assert!(
            all.contains("creates the table `hidden_rows` by its name"),
            "{all}"
        );
    }

    #[test]
    fn bad_example_a_column_without_a_marked_field() {
        let schema = SCHEMA_GOOD.replace("    Time,\n}", "    Time,\n    Extra,\n}");
        let all = table_findings(&schema, MIGRATION_GOOD, LIST_GOOD);
        assert!(
            all.contains("The column `Event::Extra` is not covered"),
            "{all}"
        );
    }

    #[test]
    fn bad_example_a_column_that_points_to_a_missing_field_or_does_not_exist() {
        let wrong_field = SCHEMA_GOOD.replace("Time => id", "Time => nothing");
        let all = table_findings(&wrong_field, MIGRATION_GOOD, LIST_GOOD);
        assert!(all.contains("points to `nothing`"), "{all}");
        let no_column = SCHEMA_GOOD.replace(
            "        Time => id,\n",
            "        Time => id,\n        Ghost => id,\n",
        );
        let all = table_findings(&no_column, MIGRATION_GOOD, LIST_GOOD);
        assert!(all.contains("names the column `Event::Ghost`"), "{all}");
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
        let sources = crate::source::read_sources(&repo, None, Kind::Rust).unwrap();
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
