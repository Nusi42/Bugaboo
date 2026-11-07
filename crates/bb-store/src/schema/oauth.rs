//! The tables of the sign-in of AI apps (AI-22, ADR-0042).

use super::{Iden, StoredTable};

/// The AI apps that registered themselves.
#[derive(Clone, Copy, Iden)]
pub(crate) enum OAuthClientRow {
    #[iden = "oauth_client"]
    Table,
    Id,
    Name,
    ReturnAddresses,
    RegisteredAt,
}

/// What people allowed AI apps.
#[derive(Clone, Copy, Iden)]
pub(crate) enum OAuthGrantRow {
    #[iden = "oauth_grant"]
    Table,
    Id,
    Client,
    Person,
    Scope,
    AllowedAt,
    LastUsedAt,
    EndsAt,
    EndedAt,
    Ended,
}

/// The projects a grant is limited to.
#[derive(Clone, Copy, Iden)]
pub(crate) enum OAuthGrantProjectRow {
    #[iden = "oauth_grant_project"]
    Table,
    Grant,
    Project,
}

/// The access tokens and renewing tokens of grants, by the hash of their secret.
#[derive(Clone, Copy, Iden)]
pub(crate) enum OAuthTokenRow {
    #[iden = "oauth_token"]
    Table,
    SecretHash,
    Grant,
    Kind,
    CreatedAt,
    ExpiresAt,
    UsedAt,
}

/// The codes apps got after *Allow*, by the hash of their secret.
#[derive(Clone, Copy, Iden)]
pub(crate) enum OAuthCodeRow {
    #[iden = "oauth_code"]
    Table,
    SecretHash,
    Grant,
    ReturnAddress,
    Challenge,
    ExpiresAt,
    UsedAt,
}

/// The web addresses administrators allowed apps to come back to.
#[derive(Clone, Copy, Iden)]
pub(crate) enum OAuthWebAddressRow {
    #[iden = "oauth_web_address"]
    Table,
    Address,
    AddedBy,
    AddedAt,
}

stored_tables! {
    OAuthClientRow => bb_core::oauth::OAuthClient {
        Id => id,
        Name => name,
        ReturnAddresses => return_addresses,
        RegisteredAt => registered_at,
    },
    OAuthGrantRow => bb_core::oauth::OAuthGrant {
        Id => id,
        Client => client,
        Person => person,
        Scope => scope,
        AllowedAt => allowed_at,
        LastUsedAt => last_used_at,
        EndsAt => ends_at,
        EndedAt => ended_at,
        Ended => ended,
    },
    OAuthGrantProjectRow => bb_core::oauth::OAuthGrantProject {
        Grant => grant,
        Project => project,
    },
    OAuthTokenRow => bb_core::oauth::OAuthToken {
        SecretHash => secret_hash,
        Grant => grant,
        Kind => kind,
        CreatedAt => created_at,
        ExpiresAt => expires_at,
        UsedAt => used_at,
    },
    OAuthCodeRow => bb_core::oauth::OAuthCode {
        SecretHash => secret_hash,
        Grant => grant,
        ReturnAddress => return_address,
        Challenge => challenge,
        ExpiresAt => expires_at,
        UsedAt => used_at,
    },
    OAuthWebAddressRow => bb_core::oauth::OAuthWebAddress {
        Address => address,
        AddedBy => added_by,
        AddedAt => added_at,
    },
}
