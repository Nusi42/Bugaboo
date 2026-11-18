//! Migration 54: the record of every backup made and restored (OPS-07, ADR-0046).
//!
//! One row for each backup made (by the schedule, the command line or the button) and each
//! restore: the name and size of the file, its SHA-256, who or what made it, when, and for a
//! restore the range of security-log entries that are gone. The line of the security log that
//! every backup and restore writes points to its row. Nothing is written into the table. The
//! person has no reference to the account (a removed person leaves a tombstone, PRIV-04).

use sea_query::{ColumnDef, Index, Table};

use super::Step;
use crate::schema::BackupRunRow;

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(BackupRunRow::Table)
                .col(
                    ColumnDef::new(BackupRunRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(BackupRunRow::Event).text().not_null())
                .col(ColumnDef::new(BackupRunRow::Name).text().not_null())
                .col(ColumnDef::new(BackupRunRow::Size).big_integer().not_null())
                .col(ColumnDef::new(BackupRunRow::Checksum).blob().not_null())
                .col(ColumnDef::new(BackupRunRow::Maker).text().not_null())
                .col(ColumnDef::new(BackupRunRow::Person).blob())
                .col(ColumnDef::new(BackupRunRow::At).big_integer().not_null())
                .col(ColumnDef::new(BackupRunRow::LostFrom).big_integer())
                .col(ColumnDef::new(BackupRunRow::LostTo).big_integer())
                .to_owned(),
        ),
        // The list shows the newest first.
        Step::Index(
            Index::create()
                .name("backup_run_by_moment")
                .table(BackupRunRow::Table)
                .col(BackupRunRow::At)
                .to_owned(),
        ),
    ]
}
