//! The table of the files of *Download my data* (PRIV-03, ADR-0044).

use super::{Iden, StoredTable};

/// The files of *Download my data*. Neither person has a reference: a removed person leaves a
/// tombstone (PRIV-04).
#[derive(Clone, Copy, Iden)]
pub(crate) enum DataExportRow {
    #[iden = "data_export"]
    Table,
    Id,
    Person,
    AskedBy,
    Reason,
    State,
    AskedAt,
    ReadyAt,
    Bytes,
}

stored_tables! {
    DataExportRow => bb_core::data_export::DataExport {
        Id => id,
        Person => person,
        AskedBy => asked_by,
        Reason => reason,
        State => state,
        AskedAt => asked_at,
        ReadyAt => ready_at,
        Bytes => bytes,
    },
}
