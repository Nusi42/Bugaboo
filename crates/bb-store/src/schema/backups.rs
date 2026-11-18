//! The table of the backups made and restored (OPS-07, migration 54).

use super::{Iden, StoredTable};

/// One backup made or restored. The person has no reference: a removed person leaves a tombstone
/// (PRIV-04), and the record stays as proof of who copied the organization's data.
#[derive(Clone, Copy, Iden)]
pub(crate) enum BackupRunRow {
    #[iden = "backup_run"]
    Table,
    Id,
    Event,
    Name,
    Size,
    Checksum,
    Maker,
    Person,
    At,
    LostFrom,
    LostTo,
}

stored_tables! {
    BackupRunRow => bb_core::backup::run::BackupRun {
        Id => id,
        Event => event,
        Name => name,
        Size => size,
        Checksum => checksum,
        Maker => maker,
        Person => person,
        At => at,
        LostFrom => lost_from,
        LostTo => lost_to,
    },
}
