//! The tables of people and of signing in.

use super::Iden;

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
    /// `mention.read_at`, taken away by migration 20.
    ReadAt,
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
