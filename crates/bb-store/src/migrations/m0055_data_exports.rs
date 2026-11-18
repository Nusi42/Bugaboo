//! Migration 55: the files of *Download my data* (PRIV-03, ADR-0044, decisions 2 to 4).
//!
//! One row for each file: whose data it holds, who asked for it (the person, or an administrator
//! with a reason), its state, when it was asked for and when it was ready, and its size. The file
//! in `exports/` is named by the ID. Nothing is written into the table. Neither person has a
//! reference (a removed person leaves a tombstone, PRIV-04).

use sea_query::{ColumnDef, Index, Table};

use super::{Step, id_column};
use crate::schema::DataExportRow;

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(DataExportRow::Table)
                .col(
                    ColumnDef::new(DataExportRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(id_column(DataExportRow::Person))
                .col(id_column(DataExportRow::AskedBy))
                .col(ColumnDef::new(DataExportRow::Reason).text())
                .col(ColumnDef::new(DataExportRow::State).text().not_null())
                .col(
                    ColumnDef::new(DataExportRow::AskedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(DataExportRow::ReadyAt).big_integer())
                .col(ColumnDef::new(DataExportRow::Bytes).big_integer())
                .to_owned(),
        ),
        // The exports of a person are read by person and state (one waiting at a time), and the
        // newest of them first.
        Step::Index(
            Index::create()
                .name("data_export_by_person")
                .table(DataExportRow::Table)
                .col(DataExportRow::Person)
                .col(DataExportRow::AskedAt)
                .to_owned(),
        ),
        // The page of every export and the deletion after the keep period read by the moment.
        Step::Index(
            Index::create()
                .name("data_export_by_moment")
                .table(DataExportRow::Table)
                .col(DataExportRow::AskedAt)
                .to_owned(),
        ),
    ]
}
