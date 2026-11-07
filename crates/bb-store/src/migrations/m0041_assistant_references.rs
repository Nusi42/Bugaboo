//! Migration 41: the references of the assistant that were used (AI-05).
//!
//! A reference that shows a proposed change works once. `assistant_reference` holds the random
//! word of each reference that was applied until the reference runs out, so a second *Apply* (a
//! retry, a double click) saves nothing again. The word is random and tells nothing; the index of
//! the time serves the deletion of what has run out.
//!
//! Nothing is written into the table.

use sea_query::{ColumnDef, Index, Table};

use super::Step;
use crate::schema::UsedReferenceRow;

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(UsedReferenceRow::Table)
                .col(
                    ColumnDef::new(UsedReferenceRow::Nonce)
                        .text()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(UsedReferenceRow::ExpiresAt)
                        .big_integer()
                        .not_null(),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("assistant_reference_by_time")
                .table(UsedReferenceRow::Table)
                .col(UsedReferenceRow::ExpiresAt)
                .to_owned(),
        ),
    ]
}
