//! Migration 43: the sign-in of AI apps (AI-22, ADR-0042).
//!
//! - `oauth_client` holds the apps that registered themselves, with the return addresses they
//!   named (one per line).
//! - `oauth_grant` holds what a person allowed an app: the scope (the words of access keys), the
//!   moments, and how it ended. `oauth_grant_project` holds the projects a grant is limited to.
//! - `oauth_token` and `oauth_code` hold the hash of every access token, renewing token and code,
//!   never the secret. A renewing token that was used stays until it runs out, so that using it
//!   again can be told.
//! - `oauth_web_address` holds the web addresses an administrator allowed.
//!
//! Nothing is written into the tables, so an upgraded database has no app and no grant.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{
    OAuthClientRow, OAuthCodeRow, OAuthGrantProjectRow, OAuthGrantRow, OAuthTokenRow,
    OAuthWebAddressRow, PersonRow, ProjectRow,
};

/// The tables of the apps and the grants.
fn grant_steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(OAuthClientRow::Table)
                .col(id_column(OAuthClientRow::Id).primary_key())
                .col(ColumnDef::new(OAuthClientRow::Name).text().not_null())
                .col(
                    ColumnDef::new(OAuthClientRow::ReturnAddresses)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(OAuthClientRow::RegisteredAt)
                        .big_integer()
                        .not_null(),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(OAuthGrantRow::Table)
                .col(id_column(OAuthGrantRow::Id).primary_key())
                .col(id_column(OAuthGrantRow::Client))
                .col(id_column(OAuthGrantRow::Person))
                .col(ColumnDef::new(OAuthGrantRow::Scope).text().not_null())
                .col(
                    ColumnDef::new(OAuthGrantRow::AllowedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(OAuthGrantRow::LastUsedAt).big_integer())
                .col(
                    ColumnDef::new(OAuthGrantRow::EndsAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(OAuthGrantRow::EndedAt).big_integer())
                .col(ColumnDef::new(OAuthGrantRow::Ended).text())
                .foreign_key(
                    ForeignKey::create()
                        .from(OAuthGrantRow::Table, OAuthGrantRow::Client)
                        .to(OAuthClientRow::Table, OAuthClientRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(OAuthGrantRow::Table, OAuthGrantRow::Person)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("oauth_grant_by_person")
                .table(OAuthGrantRow::Table)
                .col(OAuthGrantRow::Person)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(OAuthGrantProjectRow::Table)
                .col(id_column(OAuthGrantProjectRow::Grant))
                .col(id_column(OAuthGrantProjectRow::Project))
                .primary_key(
                    Index::create()
                        .col(OAuthGrantProjectRow::Grant)
                        .col(OAuthGrantProjectRow::Project),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(OAuthGrantProjectRow::Table, OAuthGrantProjectRow::Grant)
                        .to(OAuthGrantRow::Table, OAuthGrantRow::Id),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(OAuthGrantProjectRow::Table, OAuthGrantProjectRow::Project)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .to_owned(),
        ),
    ]
}

/// The tables of the projects of a grant, the tokens, the codes and the web addresses.
fn token_steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(OAuthTokenRow::Table)
                .col(id_column(OAuthTokenRow::SecretHash).primary_key())
                .col(id_column(OAuthTokenRow::Grant))
                .col(ColumnDef::new(OAuthTokenRow::Kind).text().not_null())
                .col(
                    ColumnDef::new(OAuthTokenRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(OAuthTokenRow::ExpiresAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(OAuthTokenRow::UsedAt).big_integer())
                .foreign_key(
                    ForeignKey::create()
                        .from(OAuthTokenRow::Table, OAuthTokenRow::Grant)
                        .to(OAuthGrantRow::Table, OAuthGrantRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("oauth_token_by_grant")
                .table(OAuthTokenRow::Table)
                .col(OAuthTokenRow::Grant)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(OAuthCodeRow::Table)
                .col(id_column(OAuthCodeRow::SecretHash).primary_key())
                .col(id_column(OAuthCodeRow::Grant))
                .col(
                    ColumnDef::new(OAuthCodeRow::ReturnAddress)
                        .text()
                        .not_null(),
                )
                .col(ColumnDef::new(OAuthCodeRow::Challenge).text().not_null())
                .col(
                    ColumnDef::new(OAuthCodeRow::ExpiresAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(OAuthCodeRow::UsedAt).big_integer())
                .foreign_key(
                    ForeignKey::create()
                        .from(OAuthCodeRow::Table, OAuthCodeRow::Grant)
                        .to(OAuthGrantRow::Table, OAuthGrantRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("oauth_code_by_grant")
                .table(OAuthCodeRow::Table)
                .col(OAuthCodeRow::Grant)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(OAuthWebAddressRow::Table)
                .col(
                    ColumnDef::new(OAuthWebAddressRow::Address)
                        .text()
                        .not_null()
                        .primary_key(),
                )
                .col(id_column(OAuthWebAddressRow::AddedBy))
                .col(
                    ColumnDef::new(OAuthWebAddressRow::AddedAt)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(OAuthWebAddressRow::Table, OAuthWebAddressRow::AddedBy)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
    ]
}

pub(super) fn steps() -> Vec<Step> {
    let mut steps = grant_steps();
    steps.extend(token_steps());
    steps
}
