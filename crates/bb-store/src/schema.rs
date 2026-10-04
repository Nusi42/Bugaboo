//! The names of tables and columns, written once. Migrations create the tables from them and
//! repositories query them; no table or column name is written as text anywhere else.
//!
//! Every table belongs to an entity with personal-data marks (ADR-0015): [`stored_tables!`] ties
//! them together, so a table without an entity does not compile, and DATA-MARKS reads the same
//! list to find a table that is created without being listed.
//!
//! All whole numbers are `BIGINT` and all moments are microseconds since 1970 in a `BIGINT`, so
//! both databases hand back the same Rust types. IDs are 16-byte blobs. JSON is stored as text.

use sea_query::Iden;

#[derive(Clone, Copy, Iden)]
pub(crate) enum Event {
    Table,
    Id,
    Time,
    ActorKind,
    ActorId,
    Channel,
    EntityKind,
    EntityId,
    Changes,
    Reason,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum Outbox {
    Table,
    Position,
    EventId,
    CreatedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum OutboxPosition {
    Table,
    Consumer,
    Position,
    UpdatedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum Job {
    Table,
    Id,
    Kind,
    Payload,
    RunAt,
    Attempts,
    MaxAttempts,
    State,
    LeaseOwner,
    LeaseUntil,
    LastFailure,
    CreatedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum SecurityLog {
    Table,
    Seq,
    Time,
    Person,
    Action,
    SubjectKind,
    SubjectId,
    Network,
    PreviousHash,
    Hash,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum SecurityLogCheckpoint {
    Table,
    Seq,
    Hash,
    Signature,
    CreatedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum Secret {
    Table,
    Name,
    KeyId,
    WrappedKey,
    WrappedNonce,
    Nonce,
    Ciphertext,
    UpdatedAt,
}

/// The values of settings. Only `settings.rs` reads or writes this table (ARCH-SETTINGS).
#[derive(Clone, Copy, Iden)]
pub(crate) enum SettingRow {
    #[iden = "setting"]
    Table,
    Key,
    Level,
    Scope,
    Value,
    Locked,
    UpdatedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum Migration {
    Table,
    Version,
    Name,
    AppliedAt,
}

/// Names given to computed columns in a query, so they are read back like any other column.
#[derive(Clone, Copy, Iden)]
pub(crate) enum Computed {
    Total,
    Block,
    First,
    Last,
    Newest,
}

/// Names the entity of every table: `stored_tables! { Event => DomainEvent, … }`. The name before
/// the arrow is the table's enum in this file.
macro_rules! stored_tables {
    ($($table:ident => $entity:ty),+ $(,)?) => {
        /// The name of every table with the marks of the entity stored in it, for the
        /// personal-data map.
        #[must_use]
        pub fn tables() -> Vec<(String, bb_core::personal::EntityMarks)> {
            vec![$((
                sea_query::Iden::to_string(&$table::Table),
                <$entity as bb_core::personal::Entity>::MARKS,
            )),+]
        }
    };
}

stored_tables! {
    Event => bb_core::event::DomainEvent,
    Job => bb_core::job::Job,
    Migration => bb_core::migration::MigrationRecord,
    Outbox => bb_core::outbox::OutboxEntry,
    OutboxPosition => bb_core::outbox::OutboxPosition,
    Secret => bb_core::secret::StoredSecret,
    SecurityLog => bb_core::security_log::SecurityLogEntry,
    SecurityLogCheckpoint => bb_core::security_log::SecurityLogCheckpoint,
    SettingRow => bb_core::settings::StoredSetting,
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_table_has_its_own_name_and_an_entity() {
        let tables = tables();
        let names: HashSet<&str> = tables.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names.len(), tables.len());
        assert!(names.contains("setting"));
        assert!(names.contains("security_log_checkpoint"));
        for (name, marks) in &tables {
            assert!(!marks.fields().is_empty(), "{name}");
        }
    }
}
