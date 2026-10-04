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

/// The people who can sign in. The password is not here: it has its own table, so no query for a
/// person can hand a hash to code that does not need it.
#[derive(Clone, Copy, Iden)]
pub(crate) enum PersonRow {
    #[iden = "person"]
    Table,
    Id,
    DisplayName,
    Email,
    CreatedAt,
    Administrator,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum PasswordLogin {
    Table,
    Person,
    Hash,
    ChangedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum SignInSession {
    Table,
    Id,
    Person,
    TokenHash,
    Stage,
    CreatedAt,
    LastSeenAt,
    Network,
    Device,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum TwoStep {
    Table,
    Person,
    Confirmed,
    LastStep,
    CreatedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum BackupCode {
    Table,
    Person,
    CodeHash,
    UsedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum PasskeyRow {
    #[iden = "passkey"]
    Table,
    Id,
    Person,
    CredentialId,
    Data,
    Label,
    CreatedAt,
    LastUsedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum Invitation {
    Table,
    Id,
    TokenHash,
    Email,
    InvitedBy,
    CreatedAt,
    ExpiresAt,
    UsedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum SetupCodeRow {
    #[iden = "setup_code"]
    Table,
    CodeHash,
    CreatedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum Migration {
    Table,
    Version,
    Name,
    Checksum,
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
macro_rules! stored_tables {
    ($($table:ident => $entity:ty {
        $($column:ident => $field:ident),+ $(,)?
    }),+ $(,)?) => {
        /// The name of every table with the marks of the entity stored in it and the field
        /// behind each column, for the personal-data map.
        #[must_use]
        pub fn tables() -> Vec<StoredTable> {
            vec![$(StoredTable {
                name: sea_query::Iden::to_string(&$table::Table),
                marks: <$entity as bb_core::personal::Entity>::MARKS,
                columns: vec![$((
                    sea_query::Iden::to_string(&$table::$column),
                    stringify!($field),
                )),+],
            }),+]
        }
    };
}

stored_tables! {
    Event => bb_core::event::DomainEvent {
        Id => id,
        Time => time,
        ActorKind => actor,
        ActorId => actor,
        Channel => channel,
        EntityKind => entity,
        EntityId => entity,
        Changes => changes,
        Reason => reason,
    },
    Job => bb_core::job::Job {
        Id => id,
        Kind => kind,
        Payload => payload,
        RunAt => run_at,
        Attempts => attempts,
        MaxAttempts => max_attempts,
        State => state,
        LeaseOwner => lease_owner,
        LeaseUntil => lease_until,
        LastFailure => last_failure,
        CreatedAt => created_at,
    },
    Migration => bb_core::migration::MigrationRecord {
        Version => version,
        Name => name,
        Checksum => checksum,
        AppliedAt => applied_at,
    },
    Outbox => bb_core::outbox::OutboxEntry {
        Position => position,
        EventId => event_id,
        CreatedAt => created_at,
    },
    OutboxPosition => bb_core::outbox::OutboxPosition {
        Consumer => consumer,
        Position => position,
        UpdatedAt => updated_at,
    },
    Secret => bb_core::secret::StoredSecret {
        Name => name,
        KeyId => key_id,
        WrappedKey => wrapped_key,
        WrappedNonce => wrapped_nonce,
        Nonce => nonce,
        Ciphertext => ciphertext,
        UpdatedAt => updated_at,
    },
    SecurityLog => bb_core::security_log::SecurityLogEntry {
        Seq => seq,
        Time => time,
        Person => person,
        Action => action,
        SubjectKind => subject,
        SubjectId => subject,
        Network => network,
        PreviousHash => previous_hash,
        Hash => hash,
    },
    SecurityLogCheckpoint => bb_core::security_log::SecurityLogCheckpoint {
        Seq => seq,
        Hash => hash,
        Signature => signature,
        CreatedAt => created_at,
    },
    PersonRow => bb_core::person::Person {
        Id => id,
        DisplayName => display_name,
        Email => email,
        CreatedAt => created_at,
        Administrator => administrator,
    },
    PasswordLogin => bb_core::sign_in::PasswordLogin {
        Person => person,
        Hash => hash,
        ChangedAt => changed_at,
    },
    SignInSession => bb_core::sign_in::SignInSession {
        Id => id,
        Person => person,
        TokenHash => token_hash,
        Stage => stage,
        CreatedAt => created_at,
        LastSeenAt => last_seen_at,
        Network => network,
        Device => device,
    },
    TwoStep => bb_core::sign_in::TwoStep {
        Person => person,
        Confirmed => confirmed,
        LastStep => last_step,
        CreatedAt => created_at,
    },
    BackupCode => bb_core::sign_in::BackupCode {
        Person => person,
        CodeHash => code_hash,
        UsedAt => used_at,
    },
    PasskeyRow => bb_core::sign_in::StoredPasskey {
        Id => id,
        Person => person,
        CredentialId => credential_id,
        Data => data,
        Label => label,
        CreatedAt => created_at,
        LastUsedAt => last_used_at,
    },
    Invitation => bb_core::sign_in::Invitation {
        Id => id,
        TokenHash => token_hash,
        Email => email,
        InvitedBy => invited_by,
        CreatedAt => created_at,
        ExpiresAt => expires_at,
        UsedAt => used_at,
    },
    SetupCodeRow => bb_core::sign_in::SetupCode {
        CodeHash => code_hash,
        CreatedAt => created_at,
    },
    SettingRow => bb_core::settings::StoredSetting {
        Key => key,
        Level => level,
        Scope => scope,
        Value => value,
        Locked => locked,
        UpdatedAt => updated_at,
    },
}

#[cfg(test)]
mod tests {
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
}
