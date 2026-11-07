//! Migration 37: the agreement for the AI provider (PRIV-08).
//!
//! An administrator confirms that a data processing agreement with an outside provider is in
//! place. The confirmation is kept with the destination it was given for, who gave it and when. The
//! table holds at most one row, and a change of the provider deletes it.

use sea_query::{ColumnDef, Table};

use super::Step;
use crate::schema::AiAgreementRow;

pub(super) fn steps() -> Vec<Step> {
    vec![Step::Table(
        Table::create()
            .table(AiAgreementRow::Table)
            .col(
                ColumnDef::new(AiAgreementRow::Destination)
                    .text()
                    .not_null()
                    .primary_key(),
            )
            .col(
                ColumnDef::new(AiAgreementRow::ConfirmedBy)
                    .blob()
                    .not_null(),
            )
            .col(
                ColumnDef::new(AiAgreementRow::ConfirmedAt)
                    .big_integer()
                    .not_null(),
            )
            .to_owned(),
    )]
}
