//! Migration 2: people and everything signing in keeps (AUTH-01…03, AUTH-06, AUTH-09, SEC-12).

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::Step;
use crate::schema::{
    BackupCode, Invitation, PasskeyRow, PasswordLogin, PersonRow, Retired, SetupCodeRow,
    SignInSession, TwoStep,
};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = Vec::new();
    steps.extend(people());
    steps.extend(sessions());
    steps.extend(second_step());
    steps.extend(invitations_and_setup());
    steps
}

fn people() -> Vec<Step> {
    vec![
        // The e-mail address is kept in lower case, so one unique key means one account per
        // address however it was typed.
        Step::Table(
            Table::create()
                .table(PersonRow::Table)
                .col(
                    ColumnDef::new(PersonRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(PersonRow::DisplayName).text().not_null())
                .col(
                    ColumnDef::new(PersonRow::Email)
                        .text()
                        .not_null()
                        .unique_key(),
                )
                .col(
                    ColumnDef::new(PersonRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(Retired::Administrator).boolean().not_null())
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(PasswordLogin::Table)
                .col(
                    ColumnDef::new(PasswordLogin::Person)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(PasswordLogin::Hash).text().not_null())
                .col(
                    ColumnDef::new(PasswordLogin::ChangedAt)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(PasswordLogin::Table, PasswordLogin::Person)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
    ]
}

fn sessions() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(SignInSession::Table)
                .col(
                    ColumnDef::new(SignInSession::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(SignInSession::Person).blob().not_null())
                .col(
                    ColumnDef::new(SignInSession::TokenHash)
                        .blob()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(SignInSession::Stage).text().not_null())
                .col(
                    ColumnDef::new(SignInSession::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(SignInSession::LastSeenAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(SignInSession::Network).text())
                .col(ColumnDef::new(SignInSession::Device).text().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .from(SignInSession::Table, SignInSession::Person)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("sign_in_session_by_person")
                .table(SignInSession::Table)
                .col(SignInSession::Person)
                .to_owned(),
        ),
    ]
}

fn second_step() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(TwoStep::Table)
                .col(
                    ColumnDef::new(TwoStep::Person)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(TwoStep::Confirmed).boolean().not_null())
                .col(ColumnDef::new(TwoStep::LastStep).big_integer().not_null())
                .col(ColumnDef::new(TwoStep::CreatedAt).big_integer().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .from(TwoStep::Table, TwoStep::Person)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(BackupCode::Table)
                .col(ColumnDef::new(BackupCode::Person).blob().not_null())
                .col(ColumnDef::new(BackupCode::CodeHash).blob().not_null())
                .col(ColumnDef::new(BackupCode::UsedAt).big_integer())
                .primary_key(
                    Index::create()
                        .col(BackupCode::Person)
                        .col(BackupCode::CodeHash),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(BackupCode::Table, BackupCode::Person)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(PasskeyRow::Table)
                .col(
                    ColumnDef::new(PasskeyRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(PasskeyRow::Person).blob().not_null())
                .col(
                    ColumnDef::new(PasskeyRow::CredentialId)
                        .blob()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(PasskeyRow::Data).text().not_null())
                .col(ColumnDef::new(PasskeyRow::Label).text().not_null())
                .col(
                    ColumnDef::new(PasskeyRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(PasskeyRow::LastUsedAt).big_integer())
                .foreign_key(
                    ForeignKey::create()
                        .from(PasskeyRow::Table, PasskeyRow::Person)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("passkey_by_person")
                .table(PasskeyRow::Table)
                .col(PasskeyRow::Person)
                .to_owned(),
        ),
    ]
}

fn invitations_and_setup() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(Invitation::Table)
                .col(
                    ColumnDef::new(Invitation::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(Invitation::TokenHash)
                        .blob()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(Invitation::Email).text().not_null())
                .col(ColumnDef::new(Invitation::InvitedBy).blob().not_null())
                .col(
                    ColumnDef::new(Invitation::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Invitation::ExpiresAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(Invitation::UsedAt).big_integer())
                .foreign_key(
                    ForeignKey::create()
                        .from(Invitation::Table, Invitation::InvitedBy)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        // At most one setup code waits at a time: a new one replaces the old one.
        Step::Table(
            Table::create()
                .table(SetupCodeRow::Table)
                .col(
                    ColumnDef::new(SetupCodeRow::CodeHash)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(SetupCodeRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .to_owned(),
        ),
    ]
}
