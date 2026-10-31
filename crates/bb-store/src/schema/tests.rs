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
