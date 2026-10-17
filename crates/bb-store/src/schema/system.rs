//! The tables of the event log, the jobs, the security log, the secrets, the locks, the settings and the record of migrations.

use super::Iden;

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
