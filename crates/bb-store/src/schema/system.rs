//! The tables of the event log, the jobs, the security log, the secrets, the locks, the settings and the record of migrations.

use super::{Iden, StoredTable};

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
    Run,
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

/// The lease on one thing being edited. A row lives only while its lease runs (ADR-0019).
#[derive(Clone, Copy, Iden)]
pub(crate) enum EditLockRow {
    #[iden = "edit_lock"]
    Table,
    RecordKind,
    RecordId,
    Part,
    Holder,
    Editor,
    Token,
    TakenAt,
    ExpiresAt,
}

/// The unsaved text of a person (a draft), kept until it is saved, thrown away, or 30 days old.
#[derive(Clone, Copy, Iden)]
pub(crate) enum EditDraftRow {
    #[iden = "edit_draft"]
    Table,
    Author,
    Target,
    Text,
    BaseVersion,
    SavedAt,
}

/// A note for the person who was editing, read at their next heartbeat.
#[derive(Clone, Copy, Iden)]
pub(crate) enum EditNoticeRow {
    #[iden = "edit_notice"]
    Table,
    Recipient,
    RecordKind,
    RecordId,
    Part,
    Kind,
    FromPerson,
    At,
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

/// The version of a policy that is made of several settings: a row of its own, so that a change or
/// a reset of a setting cannot take the version back.
#[derive(Clone, Copy, Iden)]
pub(crate) enum PolicyVersionRow {
    #[iden = "policy_version"]
    Table,
    Policy,
    Version,
}

/// The one agreement an administrator confirmed for the AI provider (migration 37): at most one
/// row, so the destination it was given for is its key.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AiAgreementRow {
    #[iden = "ai_agreement"]
    Table,
    Destination,
    ConfirmedBy,
    ConfirmedAt,
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
    /// Counts up with every change of the value; a person's own value is changed from several
    /// browsers at a time, so a change made from an older version is refused (migration 27).
    Version,
}

/// The counter that gives the versions of a person's own settings, so a version never goes back
/// when a value is taken away and set again (migration 28).
#[derive(Clone, Copy, Iden)]
pub(crate) enum PersonSettingCounterRow {
    #[iden = "person_setting_counter"]
    Table,
    Person,
    Counter,
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
        Run => run,
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
    EditLockRow => bb_core::edit::EditLock {
        RecordKind => record,
        RecordId => record,
        Part => part,
        Holder => holder,
        Editor => editor,
        Token => token,
        TakenAt => taken_at,
        ExpiresAt => expires_at,
    },
    EditDraftRow => bb_core::edit::Draft {
        Author => author,
        Target => target,
        Text => text,
        BaseVersion => base_version,
        SavedAt => saved_at,
    },
    EditNoticeRow => bb_core::edit::EditNotice {
        Recipient => recipient,
        RecordKind => record,
        RecordId => record,
        Part => part,
        Kind => kind,
        FromPerson => from,
        At => at,
    },
    SettingRow => bb_core::settings::StoredSetting {
        Key => key,
        Level => level,
        Scope => scope,
        Value => value,
        Locked => locked,
        UpdatedAt => updated_at,
    },
    AiAgreementRow => bb_core::ai::Agreement {
        Destination => destination,
        ConfirmedBy => confirmed_by,
        ConfirmedAt => confirmed_at,
    },
    PolicyVersionRow => bb_core::settings::PolicyVersion {
        Policy => policy,
        Version => version,
    },
    PersonSettingCounterRow => bb_core::settings::PersonSettingCounter {
        Person => person,
        Counter => counter,
    },
    PersonSettingRow => bb_core::settings::StoredPersonSetting {
        Person => person,
        Key => key,
        Value => value,
        UpdatedAt => updated_at,
        Version => version,
    },
}
