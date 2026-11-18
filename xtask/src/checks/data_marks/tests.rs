use super::*;
use crate::source::Source;

const VOCABULARY: &str =
    "words! { pub enum Category { Identity => \"identity\", EmployeeTime => \"employee_time\" } }
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
    table_findings_with(schema, None, migration, list)
}

/// The findings for `schema` (the file with the list) and, if given, `apart` (the enums in a
/// file of `schema/` beside it).
fn table_findings_with(schema: &str, apart: Option<&str>, migration: &str, list: &str) -> String {
    let entity =
        "#[derive(Entity)]\npub struct DomainEvent {\n    #[not_personal]\n    pub id: u32,\n}\n";
    let mut sources: Vec<Source> = vec![
        (privacy::VOCABULARY_PATH.to_owned(), VOCABULARY.to_owned()),
        (privacy::LIST_PATH.to_owned(), list.to_owned()),
        ("crates/bb-core/src/event.rs".to_owned(), entity.to_owned()),
    ];
    let model = privacy::read_model(&sources).unwrap();
    sources.push((SCHEMA_PATH.to_owned(), schema.to_owned()));
    if let Some(enums) = apart {
        sources.push((format!("{SCHEMA_FOLDER}system.rs"), enums.to_owned()));
    }
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
    let found = findings_with_enums_apart(enums, list);
    assert!(found.is_empty(), "{found}");
}

/// Findings for a schema whose enums are in a file beside the list, after the macro that
/// the list of `schema.rs` is made of.
fn findings_with_enums_apart(enums: &str, list: &str) -> String {
    table_findings_with(list, Some(enums), MIGRATION_GOOD, LIST_GOOD)
}

#[test]
fn bad_example_the_columns_of_enums_in_a_file_beside_the_list_are_checked() {
    let (enums, list) = SCHEMA_GOOD.split_at(SCHEMA_GOOD.find("macro_rules!").unwrap());
    let uncovered = enums.replace(
        "    Time,
}",
        "    Time,
    Extra,
}",
    );
    let all = findings_with_enums_apart(&uncovered, list);
    assert!(
        all.contains("The column `Event::Extra` is not covered"),
        "{all}"
    );
    // A mapped column renamed to one the enum does not have.
    let renamed = enums.replace(
        "    Time,
}",
        "    Moment,
}",
    );
    let all = findings_with_enums_apart(&renamed, list);
    assert!(all.contains("names the column `Event::Time`"), "{all}");
    assert!(
        all.contains("The column `Event::Moment` is not covered"),
        "{all}"
    );
}

/// The schema with a second table that takes the columns of `Event`; `own` are its own
/// variants besides `Table`.
fn schema_with_a_shared_table(own: &str) -> String {
    SCHEMA_GOOD
        .replace(
            "macro_rules!",
            &format!(
                "pub(crate) enum Copy {{
    Table,
{own}}}
macro_rules!"
            ),
        )
        .replace(
            "    },
}
",
            "    },
    Copy(Event) => bb_core::event::DomainEvent {
        Id => id,
        Time => id,
    },
}
",
        )
}

#[test]
fn good_example_a_table_takes_the_columns_of_another_tables_enum() {
    let found = table_findings(&schema_with_a_shared_table(""), MIGRATION_GOOD, LIST_GOOD);
    assert!(found.is_empty(), "{found}");
}

#[test]
fn bad_example_a_table_with_shared_columns_is_checked_against_them() {
    // A column the shared enum has not.
    let schema = schema_with_a_shared_table("").replace(
        "        Time => id,
    },
}
",
        "        Time => id,
        Extra => id,
    },
}
",
    );
    let all = table_findings(&schema, MIGRATION_GOOD, LIST_GOOD);
    assert!(all.contains("names the column `Copy::Extra`"), "{all}");
    // A table that has columns of its own besides the shared ones.
    let all = table_findings(
        &schema_with_a_shared_table(
            "    Own,
",
        ),
        MIGRATION_GOOD,
        LIST_GOOD,
    );
    assert!(
        all.contains("`Copy` takes its columns from `Event`"),
        "{all}"
    );
}

#[test]
fn good_example_a_table_made_by_its_old_name_is_renamed_to_one_with_marks() {
    let migration = format!(
        "{MIGRATION_GOOD}Table::create().table(Alias::new(OLD)).col(x)\nconst OLD: &str = \"old_event\";\nTable::rename().table(Alias::new(\"old_event\"), Event::Table)\n"
    );
    assert_eq!(table_findings(SCHEMA_GOOD, &migration, LIST_GOOD), "");
}

#[test]
fn bad_example_a_table_renamed_to_one_without_marks() {
    let migration = format!(
        "{MIGRATION_GOOD}Table::create().table(Alias::new(\"old_rows\")).col(x)
Table::rename().table(Alias::new(\"old_rows\"), Hidden::Table)
"
    );
    let all = table_findings(SCHEMA_GOOD, &migration, LIST_GOOD);
    assert!(all.contains("renames a table to `Hidden`"), "{all}");
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
fn priv_02_every_field_that_names_a_person_has_a_link() {
    let repo = crate::repo::Repo::discover().unwrap();
    let sources = crate::source::read_sources(&repo, None, Kind::Rust).unwrap();
    let model = privacy::read_model(&sources).unwrap();
    let mut findings = evaluate(&model);
    findings.extend(links::evaluate_links(&model));
    findings.extend(evaluate_tables(&model, &sources));
    assert!(
        findings.is_empty(),
        "{:?}",
        findings
            .iter()
            .map(|f| (&f.location, &f.message))
            .collect::<Vec<_>>()
    );
}
