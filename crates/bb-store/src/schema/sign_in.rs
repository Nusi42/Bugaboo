//! The tables of people and of signing in.

use super::{Iden, StoredTable};

/// The people who can sign in. The password is not here: it has its own table, so no query for a
/// person can hand a hash to code that does not need it.
#[derive(Clone, Copy, Iden)]
pub(crate) enum PersonRow {
    #[iden = "person"]
    Table,
    Id,
    DisplayName,
    /// The name as names are told apart (`bb_core::names::name_key`), made in Rust (migration 26).
    NameKey,
    Email,
    CreatedAt,
}

/// Columns that an earlier migration made and a later one took away. Old migrations keep their
/// text (a changed migration is refused), so the name stays here; no entity stands behind it.
#[derive(Clone, Copy, Iden)]
pub(crate) enum Retired {
    Administrator,
    /// `mention.read_at`, taken away by migration 20.
    ReadAt,
    /// `saved_search.spacing`, taken away by migration 30: every list follows the person's
    /// setting *Spacing* (ADR-0034, amendment).
    Spacing,
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

stored_tables! {
    PersonRow => bb_core::person::Person {
        Id => id,
        DisplayName => display_name,
        NameKey => display_name,
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
}
