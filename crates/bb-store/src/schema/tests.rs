//! The list of tables against the entities and their marks.

use std::collections::HashSet;

use super::*;

#[test]
fn every_table_has_its_own_name_and_every_column_a_marked_field() {
    let tables = tables();
    let names: HashSet<&str> = tables.iter().map(|table| table.name.as_str()).collect();
    assert_eq!(names.len(), tables.len());
    assert!(names.contains("setting"));
    assert!(names.contains("security_log_checkpoint"));
    for table in &tables {
        assert!(!table.marks.fields().is_empty(), "{}", table.name);
        let fields: HashSet<&str> = table
            .marks
            .fields()
            .iter()
            .map(bb_core::personal::FieldMark::name)
            .collect();
        for (column, field) in &table.columns {
            assert!(
                fields.contains(field),
                "{}.{column} points to the field `{field}`, which the entity does not have",
                table.name
            );
        }
        // Every field of the entity is stored in some column.
        let stored: HashSet<&str> = table.columns.iter().map(|(_, field)| *field).collect();
        for field in &fields {
            assert!(
                stored.contains(field),
                "{}: `{field}` is not stored",
                table.name
            );
        }
    }
}

/// The columns of `field` in `table`.
fn columns_of<'t>(table: &'t StoredTable, field: &str) -> Vec<&'t str> {
    table
        .columns
        .iter()
        .filter(|(_, of)| *of == field)
        .map(|(column, _)| column.as_str())
        .collect()
}

/// The walk of a person's data finds a row by the columns of a linked field (ADR-0044): one column
/// with the person's ID, or a kind and an ID (an actor, a reference to a record), in that order;
/// the field a link reads the kind or the type of an own field from is stored too.
#[test]
fn priv_02_every_field_that_names_a_person_has_a_link_on_a_stored_column() {
    use bb_core::personal::LinkScope;
    let tables = tables();
    for table in &tables {
        for link in table.marks.person_links().links() {
            let columns = columns_of(table, link.field()).len();
            let (expected, other) = match link.scope() {
                LinkScope::Every => (1..=2, None),
                LinkScope::Record { kind: None } => (2..=2, None),
                LinkScope::Record { kind: Some(kind) } => (1..=1, Some(kind)),
                LinkScope::FieldType { key, .. } => (1..=1, Some(key)),
                LinkScope::PartOf { .. }
                | LinkScope::Email
                | LinkScope::Changes
                | LinkScope::Json => (1..=1, None),
            };
            assert!(
                expected.contains(&columns),
                "{}.{} is stored in {columns} columns",
                table.name,
                link.field()
            );
            if let Some(other) = other {
                assert_eq!(
                    columns_of(table, other).len(),
                    1,
                    "{}.{other} is not stored in one column",
                    table.name
                );
            }
        }
    }
    // Every entity that names a person is stored in a table, so the walk reaches its rows.
    for marks in bb_core::entities::ALL {
        if !marks.person_links().links().is_empty() {
            assert!(
                tables
                    .iter()
                    .any(|table| table.marks.name() == marks.name()),
                "{} names a person but is stored in no table",
                marks.name()
            );
        }
    }
}

/// The second hop of the walk (ADR-0044): a record that belongs to a person is found in the table
/// of its entity by the first column of its ID field, and the owner is a field of that entity that names a person in every
/// row.
#[test]
fn priv_02_every_kind_of_record_says_whose_it_is_through_a_stored_owner() {
    use bb_core::event::{RecordKind, Whose, whose};
    use bb_core::personal::LinkScope;
    let tables = tables();
    for kind in RecordKind::ALL {
        let Whose::Owner { of } = whose(*kind) else {
            continue;
        };
        let (marks, owner) = (of.marks, of.value);
        let table = tables
            .iter()
            .find(|table| table.marks.name() == marks.name())
            .unwrap_or_else(|| panic!("{} is stored in no table", marks.name()));
        // The walk reads the ID of a record from the first column of the field.
        assert_eq!(
            columns_of(table, of.id).first(),
            Some(&of.id),
            "{}.{}",
            table.name,
            of.id
        );
        assert!(
            marks
                .person_links()
                .of_field(owner)
                .any(|link| link.scope() == LinkScope::Every),
            "{}.{owner} does not name a person in every row",
            marks.name()
        );
    }
}
