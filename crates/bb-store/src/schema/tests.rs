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

/// The walk of a person's data finds a row by the columns of a linked field (ADR-0044): one column
/// with the person's ID, or a kind and an ID (an actor, a reference to a record), in that order.
#[test]
fn priv_02_every_field_that_names_a_person_has_a_link_on_a_stored_column() {
    let tables = tables();
    for table in &tables {
        for link in table.marks.person_links().links() {
            let columns = table
                .columns
                .iter()
                .filter(|(_, field)| *field == link.field())
                .count();
            let expected = if link.kind().is_some() { 2..=2 } else { 1..=2 };
            assert!(
                expected.contains(&columns),
                "{}.{} is stored in {columns} columns",
                table.name,
                link.field()
            );
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
