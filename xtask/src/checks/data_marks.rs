//! DATA-MARKS: every stored field is marked as personal data or not (ADR-0015, PRIV-02).
//!
//! The derive macro `#[derive(Entity)]` already stops the build for an unmarked field. This check
//! reads the same marks from the source, so the mistake shows up right after the edit, and it
//! checks what the compiler cannot: the words of the marks, and that every entity is named in the
//! `entities!` list that the personal-data map is built from. The counts of marked fields are on
//! the generated page `docs/privacy/personal-data.md` (`cargo xtask gen privacy`).
//!
//! A table is stored data too: every table of `bb-store` (an enum with a `Table` variant in
//! `schema.rs` or in a file of `schema/`) must be named in the `stored_tables!` list with the entity it holds, and that
//! entity must carry marks and be in the `entities!` list. A table that is created in a migration
//! without being listed is found as well (ADR-0015, ADR-0003). A table whose columns are those of
//! another table's enum names that enum in brackets (`TimeEntryValueRow(FieldValueRow) => …`), and
//! its columns are checked against that enum.
//!
//! Every field that names a person carries a link that says what the person is to the row
//! (ADR-0044, decision 1); `links` finds the field that has none.

mod links;

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
        findings.extend(links::evaluate_links(&model));
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
    /// The enum whose columns the table shares, when it is not the table's own.
    columns_of: Option<String>,
    entity: String,
    columns: Vec<(String, String)>,
}

/// The entries of every `stored_tables!` list (one in each file of an area):
///
/// ```text
/// Table => path::Entity {
///     Column => field,
/// },
/// Table(ColumnsOf) => path::Entity {
///     Column => field,
/// },
/// ```
fn listed_tables(schema: &str) -> Vec<ListedTable> {
    let lists = schema.match_indices(
        "stored_tables! {
",
    );
    lists.flat_map(|(at, _)| listed_in(&schema[at..])).collect()
}

/// The entries of the list that starts at the first line of `list`.
fn listed_in(list: &str) -> Vec<ListedTable> {
    let mut tables: Vec<ListedTable> = Vec::new();
    let mut open = false;
    for line in list.lines().skip(1) {
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
            let table = table.trim();
            let (table, columns_of) = match table.split_once('(') {
                Some((table, columns_of)) => (
                    table.trim(),
                    Some(columns_of.trim_end_matches(')').trim().to_owned()),
                ),
                None => (table, None),
            };
            tables.push(ListedTable {
                table: table.to_owned(),
                columns_of,
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
    let mut enums = Vec::new();
    let mut current: Option<(String, Vec<String>, bool)> = None;
    for line in schema.lines() {
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

/// A table that a migration renames: its old name, and the enum of the schema it becomes.
struct Renamed {
    old: String,
    new: Option<String>,
}

/// The tables that migrations rename (`Table::rename().table(Alias::new("old"), New::Table)`).
fn renamed_tables(text: &str) -> Vec<Renamed> {
    text.match_indices("Table::rename()")
        .filter_map(|(at, _)| {
            let rest = &text[at..];
            let open = rest.find(".table(Alias::new")?;
            let argument = &rest[open + ".table(Alias::new".len()..];
            let old = alias_name(text, argument)?;
            let new = argument.split_once("),").and_then(|(_, target)| {
                let path: String = target
                    .trim_start()
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | ':'))
                    .collect();
                let table = path.strip_suffix("::Table")?;
                table.rsplit("::").next().map(str::to_owned)
            });
            Some(Renamed { old, new })
        })
        .collect()
}

/// The tables the migrations create without the marks of an entity behind them.
fn migration_findings(listed: &[ListedTable], sources: &[crate::source::Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let renamed: Vec<Renamed> = sources
        .iter()
        .filter(|(path, _)| path.starts_with(MIGRATIONS_FOLDER))
        .flat_map(|(_, text)| renamed_tables(text))
        .collect();
    for (path, text) in sources
        .iter()
        .filter(|(path, _)| path.starts_with(MIGRATIONS_FOLDER))
    {
        for new in renamed_tables(text).iter().filter_map(|r| r.new.as_deref()) {
            if !listed.iter().any(|entry| entry.table == new) {
                findings.push(Finding::at(
                    path,
                    format!(
                        "A migration renames a table to `{new}`, which is not in `stored_tables!` in {SCHEMA_PATH}, so its rows have no personal-data marks."
                    ),
                ));
            }
        }
    }
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
                Created::Named(name) if !renamed.iter().any(|r| r.old == name) => {
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

/// The columns of a listed table: its own variants, or those of the enum it names in brackets,
/// which it may take only with no columns of its own (none, with a finding, otherwise).
fn columns_of_entry<'a>(
    entry: &ListedTable,
    own: &'a Vec<String>,
    enums: &'a [(String, Vec<String>)],
    findings: &mut Vec<Finding>,
) -> Option<&'a Vec<String>> {
    let Some(columns_of) = &entry.columns_of else {
        return Some(own);
    };
    match enums.iter().find(|(name, _)| name == columns_of) {
        Some((_, shared)) if own.is_empty() => Some(shared),
        _ => {
            findings.push(Finding::at(
                SCHEMA_PATH,
                format!(
                    "`{}` takes its columns from `{columns_of}`, so it has no columns of its own                      and `{columns_of}` is the enum of a table.",
                    entry.table
                ),
            ));
            None
        }
    }
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
    let enums = table_enums(schema);
    for (table, own) in &enums {
        let Some(entry) = listed.iter().find(|entry| entry.table == *table) else {
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
        let Some(variants) = columns_of_entry(entry, own, &enums, &mut findings) else {
            continue;
        };
        for variant in variants {
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
mod tests;
