//! The names of tables and columns, written once. Migrations create the tables from them and
//! repositories query them; no table or column name is written as text anywhere else.
//!
//! Every table belongs to an entity with personal-data marks (ADR-0015): [`stored_tables!`] ties
//! them together, column by column (`Column => field`), so a table without an entity does not
//! compile, and DATA-MARKS reads the same list to find a table that is created without being
//! listed and a column that no marked field stands behind.
//!
//! All whole numbers are `BIGINT` and all moments are microseconds since 1970 in a `BIGINT`, so
//! both databases hand back the same Rust types. IDs are 16-byte blobs. JSON is stored as text.

use sea_query::{Iden, IdenStatic};

/// A table with the entity stored in it.
#[derive(Debug)]
pub struct StoredTable {
    /// The name of the table.
    pub name: String,
    /// The personal-data marks of the entity stored in it.
    pub marks: bb_core::personal::EntityMarks,
    /// Each column of the table with the field of the entity it holds (several columns can hold
    /// one field, for example a kind and an ID).
    pub columns: Vec<(String, &'static str)>,
}

/// Names the entity of every table and the field behind every column:
/// `stored_tables! { Event => DomainEvent { Id => id, … } }`. The name before the first arrow
/// is the table's enum in this file, and the names before the other arrows are its variants, so
/// a column that does not exist does not compile; DATA-MARKS requires that every variant has a
/// line and that the field exists in the entity.
///
/// A field that names a person (ADR-0044) is stored in one column that holds the person's ID, or in
/// two: a kind and then an ID (the actor of an event, a reference to a record); the walk of a
/// person's data reads the ID from the last of them.
/// The names of the columns, of the table's own enum or of the enum named in brackets whose
/// columns the table shares.
macro_rules! column_names {
    ($table:ident; $($column:ident),+) => {
        [$(sea_query::Iden::to_string(&$table::$column)),+]
    };
    ($table:ident, $columns:ident; $($column:ident),+) => {
        [$(sea_query::Iden::to_string(&$columns::$column)),+]
    };
}

macro_rules! stored_tables {
    ($($table:ident $(($columns:ident))? => $entity:ty {
        $($column:ident => $field:ident),+ $(,)?
    }),+ $(,)?) => {
        /// The name of every table with the marks of the entity stored in it and the field
        /// behind each column, for the personal-data map.
        #[must_use]
        pub(super) fn tables() -> Vec<StoredTable> {
            vec![$(StoredTable {
                name: sea_query::Iden::to_string(&$table::Table),
                marks: <$entity as bb_core::personal::Entity>::MARKS,
                columns: column_names!($table $(, $columns)?; $($column),+)
                    .into_iter()
                    .zip([$(stringify!($field)),+])
                    .collect(),
            }),+]
        }
    };
}

/// The areas of the schema, each a file that names its tables: declared, re-exported and listed
/// for the personal-data map from this one list, so an area cannot be left out of the map.
macro_rules! areas {
    ($($area:ident),+ $(,)?) => {
        $(mod $area;)+
        $(pub(crate) use $area::*;)+

        /// The name of every table with the marks of the entity stored in it and the field behind
        /// each column, for the personal-data map. Each area names its own tables in its own file.
        #[must_use]
        pub fn tables() -> Vec<StoredTable> {
            let mut tables = Vec::new();
            $(tables.extend($area::tables());)+
            tables
        }
    };
}

areas! {
    access,
    agent_work,
    agents,
    ai,
    agent_requests,
    backups,
    comments,
    dashboards,
    data_exports,
    e_mail_changes,
    files,
    notify,
    oauth,
    plan,
    releases,
    search_mail,
    searches,
    sign_in,
    system,
    text_index,
    themes,
    time,
    walk_index,
    work,
    work_times,
    working_hours,
}

#[cfg(test)]
mod tests;
