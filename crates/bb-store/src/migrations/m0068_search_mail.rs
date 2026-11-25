//! Migration 68: e-mailed searches (QRY-07, ADR-0033, amendment 2026-11-24).
//!
//! One row for each saved search a person gets by e-mail: the weekdays (one bit each, Monday the
//! lowest), the whole hour in the person's time zone, *Only when something is new or changed*, why
//! it waits for its person (none while it sends), when its next e-mail is due, the due time whose
//! e-mail was last claimed (each goes once) and when its last one went. The issues of an e-mail are never kept. A person has one row at most for each saved
//! search. Nothing is written into the table.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{SavedSearchRow, SearchMailRow};

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(SearchMailRow::Table)
                .col(
                    ColumnDef::new(SearchMailRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(id_column(SearchMailRow::Holder))
                .col(id_column(SearchMailRow::Search))
                .col(ColumnDef::new(SearchMailRow::Days).big_integer().not_null())
                .col(ColumnDef::new(SearchMailRow::Hour).big_integer().not_null())
                .col(
                    ColumnDef::new(SearchMailRow::OnlyWhenChanged)
                        .boolean()
                        .not_null(),
                )
                .col(ColumnDef::new(SearchMailRow::Paused).text())
                .col(
                    ColumnDef::new(SearchMailRow::NextAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(SearchMailRow::LastDue).big_integer())
                .col(ColumnDef::new(SearchMailRow::LastSentAt).big_integer())
                .col(
                    ColumnDef::new(SearchMailRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(SearchMailRow::UpdatedAt)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(SearchMailRow::Table, SearchMailRow::Search)
                        .to(SavedSearchRow::Table, SavedSearchRow::Id),
                )
                .to_owned(),
        ),
        // One e-mailed search of a saved search for a person; the list of a person and the walk of
        // their data read by it.
        Step::Index(
            Index::create()
                .name("search_mail_by_holder")
                .table(SearchMailRow::Table)
                .col(SearchMailRow::Holder)
                .col(SearchMailRow::Search)
                .unique()
                .to_owned(),
        ),
        // The e-mailed searches of a saved search go with it.
        Step::Index(
            Index::create()
                .name("search_mail_by_search")
                .table(SearchMailRow::Table)
                .col(SearchMailRow::Search)
                .to_owned(),
        ),
        // The hourly round finds the ones that are due.
        Step::Index(
            Index::create()
                .name("search_mail_by_next")
                .table(SearchMailRow::Table)
                .col(SearchMailRow::NextAt)
                .to_owned(),
        ),
    ]
}
