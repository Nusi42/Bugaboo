//! Migration 58: the changes of e-mail addresses that wait for their link (PRIV-06, ADR-0044,
//! decision 9).
//!
//! One row for each person at most: the keyed digest of the secret of the link (the secret is only
//! in the e-mail), the new address, when it was asked for and when the link runs out. The link is
//! looked up by its digest, and the daily job finds the rows that ran out by their end.

use sea_query::{ColumnDef, Index, Table};

use super::{Step, id_column};
use crate::schema::EmailChangeRow;

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(EmailChangeRow::Table)
                .col(id_column(EmailChangeRow::Person).primary_key())
                .col(
                    ColumnDef::new(EmailChangeRow::TokenDigest)
                        .blob()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(EmailChangeRow::NewEmail).text().not_null())
                .col(
                    ColumnDef::new(EmailChangeRow::AskedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(EmailChangeRow::ExpiresAt)
                        .big_integer()
                        .not_null(),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("e_mail_change_by_end")
                .table(EmailChangeRow::Table)
                .col(EmailChangeRow::ExpiresAt)
                .to_owned(),
        ),
    ]
}
