//! The port of the records of *Download my data* (PRIV-03): what the use cases need from storage.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::data_export::{DataExport, ExportState};
use bb_core::id::{DataExportId, UserId};
use jiff::Timestamp;

use crate::ports::ChangeStore;
use crate::store_macros::{queries, reads, writes};

/// What the use cases of *Download my data* need from storage.
pub trait ExportStore: ChangeStore + 'static {
    queries! {
        /// The export with this ID.
        fn data_export(id: DataExportId) -> Option<DataExport> = data_export_in;
        /// The export of `person` that is still waiting for its file, if there is one: there is
        /// never more than one (ADR-0044, *Limits*).
        fn waiting_export_of(person: UserId) -> Option<DataExport> = waiting_export_of_in;
    }

    reads! {
        /// The newest export of `person` that `asked_by` asked for.
        fn latest_export_of(person: UserId, asked_by: UserId) -> Option<DataExport>;
        /// A page of every export, newest first: at most `limit`, after skipping `skip`.
        fn data_exports_page(skip: u64, limit: u64) -> Vec<DataExport>;
        /// At most `limit` exports asked for before `before`, oldest first: those past their keep
        /// period.
        fn exports_asked_before(before: Timestamp, limit: u64) -> Vec<DataExport>;
    }

    writes! {
        /// Holds the exports of `person` until the change ends, so two requests at the same moment
        /// cannot both find no waiting export.
        fn lock_exports_of(person: UserId) -> ();
        /// Adds an export.
        fn insert_data_export(export: &DataExport) -> ();
        /// Ends a waiting export in `state`, with the moment and the size of a file that is ready;
        /// says whether it was still waiting.
        fn finish_data_export(
            id: DataExportId,
            state: ExportState,
            ready_at: Option<Timestamp>,
            bytes: Option<i64>
        ) -> bool;
        /// Deletes an export; says whether it was there.
        fn delete_data_export(id: DataExportId) -> bool;
    }
}
