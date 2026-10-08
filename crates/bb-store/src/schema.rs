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

/// The values of settings that people chose for themselves in *My settings*. Only `settings.rs`
/// reads or writes this table (ARCH-SETTINGS); it is its own table so the person it belongs to is
/// a column of its own with a personal-data mark.
#[derive(Clone, Copy, Iden)]
pub(crate) enum PersonSettingRow {
    #[iden = "person_setting"]
    Table,
    Person,
    Key,
    Value,
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
}

/// Columns that an earlier migration made and a later one took away. Old migrations keep their
/// text (a changed migration is refused), so the name stays here; no entity stands behind it.
#[derive(Clone, Copy, Iden)]
pub(crate) enum Retired {
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

/// Projects (PRJ-01): the tree that roles are given on.
#[derive(Clone, Copy, Iden)]
pub(crate) enum ProjectRow {
    #[iden = "project"]
    Table,
    Id,
    Parent,
    ShortName,
    Name,
    Version,
}

/// The old short names of projects: each stays with its project (PRJ-01, M2-Q4).
#[derive(Clone, Copy, Iden)]
pub(crate) enum ProjectAliasRow {
    #[iden = "project_alias"]
    Table,
    ShortName,
    Project,
    RetiredAt,
}

/// Groups of people. The table is called `person_group`: `group` is a word of SQL.
#[derive(Clone, Copy, Iden)]
pub(crate) enum GroupRow {
    #[iden = "person_group"]
    Table,
    Id,
    Name,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum GroupMemberRow {
    #[iden = "group_member"]
    Table,
    Group,
    Person,
    AddedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum RoleRow {
    #[iden = "role"]
    Table,
    Id,
    Name,
    Starting,
    Position,
    TwoStep,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum RolePermissionRow {
    #[iden = "role_permission"]
    Table,
    Role,
    Permission,
}

/// *Who has which role*: a person or a group, a role or *No access* (no role), and where it counts.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AssignmentRow {
    #[iden = "assignment"]
    Table,
    Id,
    Person,
    Group,
    Role,
    Project,
    AppliesTo,
    GivenAt,
}

#[derive(Clone, Copy, IdenStatic)]
pub(crate) enum RoleFieldLevelRow {
    #[iden = "role_field_level"]
    Table,
    Role,
    Field,
    Level,
}

#[derive(Clone, Copy, IdenStatic)]
pub(crate) enum ProjectFieldLevelRow {
    #[iden = "project_field_level"]
    Table,
    Project,
    Role,
    Field,
    Level,
}

#[derive(Clone, Copy, IdenStatic)]
pub(crate) enum RolePeopleLevelRow {
    #[iden = "role_people_level"]
    Table,
    Role,
    Kind,
    Level,
}

#[derive(Clone, Copy, IdenStatic)]
pub(crate) enum ProjectPeopleLevelRow {
    #[iden = "project_people_level"]
    Table,
    Project,
    Role,
    Kind,
    Level,
}

/// The access keys of people (AUTH-07). Only the hash of the secret is stored.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AccessKeyRow {
    #[iden = "access_key"]
    Table,
    Id,
    Owner,
    Name,
    Prefix,
    SecretHash,
    Choice,
    CreatedAt,
    ExpiresAt,
    LastUsedAt,
}

/// The single permissions of a key whose choice is *More options*.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AccessKeyPermissionRow {
    #[iden = "access_key_permission"]
    Table,
    Key,
    Permission,
}

/// The projects a key is limited to.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AccessKeyProjectRow {
    #[iden = "access_key_project"]
    Table,
    Key,
    Project,
}

/// The people who were administrators in M1, until the first start gives them the role.
#[derive(Clone, Copy, Iden)]
pub(crate) enum LegacyAdministratorRow {
    #[iden = "legacy_administrator"]
    Table,
    Person,
}

/// One row, once the starting roles were made.
#[derive(Clone, Copy, Iden)]
pub(crate) enum StartingRolesMadeRow {
    #[iden = "starting_roles_made"]
    Table,
    MadeAt,
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
    ProjectRow => bb_core::project::Project {
        Id => id,
        Parent => parent,
        ShortName => short_name,
        Name => name,
        Version => version,
    },
    ProjectAliasRow => bb_core::project::ProjectAlias {
        ShortName => short_name,
        Project => project,
        RetiredAt => retired_at,
    },
    GroupRow => bb_core::access::Group {
        Id => id,
        Name => name,
    },
    GroupMemberRow => bb_core::access::GroupMember {
        Group => group,
        Person => person,
        AddedAt => added_at,
    },
    RoleRow => bb_core::access::Role {
        Id => id,
        Name => name,
        Starting => starting,
        Position => position,
        TwoStep => two_step,
    },
    RolePermissionRow => bb_core::access::RolePermission {
        Role => role,
        Permission => permission,
    },
    AssignmentRow => bb_core::access::Assignment {
        Id => id,
        Person => person,
        Group => group,
        Role => role,
        Project => project,
        AppliesTo => applies_to,
        GivenAt => given_at,
    },
    RoleFieldLevelRow => bb_core::access::RoleFieldLevel {
        Role => role,
        Field => field,
        Level => level,
    },
    ProjectFieldLevelRow => bb_core::access::ProjectFieldLevel {
        Project => project,
        Role => role,
        Field => field,
        Level => level,
    },
    RolePeopleLevelRow => bb_core::access::RolePeopleLevel {
        Role => role,
        Kind => kind,
        Level => level,
    },
    ProjectPeopleLevelRow => bb_core::access::ProjectPeopleLevel {
        Project => project,
        Role => role,
        Kind => kind,
        Level => level,
    },
    LegacyAdministratorRow => bb_core::access::LegacyAdministrator {
        Person => person,
    },
    StartingRolesMadeRow => bb_core::access::StartingRolesMade {
        MadeAt => made_at,
    },
    AccessKeyRow => bb_core::access_key::AccessKey {
        Id => id,
        Owner => owner,
        Name => name,
        Prefix => prefix,
        SecretHash => secret_hash,
        Choice => choice,
        CreatedAt => created_at,
        ExpiresAt => expires_at,
        LastUsedAt => last_used_at,
    },
    AccessKeyPermissionRow => bb_core::access_key::AccessKeyPermission {
        Key => key,
        Permission => permission,
    },
    AccessKeyProjectRow => bb_core::access_key::AccessKeyProject {
        Key => key,
        Project => project,
    },
    SettingRow => bb_core::settings::StoredSetting {
        Key => key,
        Level => level,
        Scope => scope,
        Value => value,
        Locked => locked,
        UpdatedAt => updated_at,
    },
    PersonSettingRow => bb_core::settings::StoredPersonSetting {
        Person => person,
        Key => key,
        Value => value,
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
