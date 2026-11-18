//! *Download my data* (PRIV-03, ADR-0044, decisions 2 to 4): the record of one file of a person's
//! data, who asked for it, for whom, when and in which state it is. The file itself lies in the
//! `exports/` folder of the data folder, named by the ID of its record and nothing a person typed.

use jiff::{SignedDuration, Timestamp};

use crate::id::{DataExportId, UserId};
use crate::personal::Entity;

/// How long the file of an export is kept after it was asked for (ADR-0044, decision 2): then it is
/// deleted with its record.
pub const EXPORT_KEPT: SignedDuration = SignedDuration::from_hours(7 * 24);

/// The longest reason an administrator may give for the export of somebody else's data.
pub const EXPORT_REASON_MOST: usize = 500;

words! {
    /// Where an export stands.
    pub enum ExportState {
        /// Asked for; the job that writes the file has not finished yet.
        Waiting => "waiting",
        /// The file is written and can be downloaded until it is deleted.
        Ready => "ready",
        /// The job gave up; nothing was written. The person may ask again.
        Failed => "failed",
    }
}

words_in_api!(
    ExportState,
    "Where a file of *Download my data* stands: waiting for the job that writes it, ready to download, or failed."
);

/// One file of *Download my data*.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct DataExport {
    /// The ID of the export; the file is named by it.
    #[not_personal]
    pub id: DataExportId,
    /// Whose data the file holds.
    #[personal(category = "activity", basis = "legal_obligation")]
    #[person(subject)]
    pub person: UserId,
    /// Who asked for it: the person, or an administrator with *Download someone's data*.
    #[personal(category = "activity", basis = "legal_obligation")]
    #[person(actor)]
    pub asked_by: UserId,
    /// Why an administrator asked for somebody else's data; none for a person's own export.
    #[personal(category = "content", basis = "legal_obligation")]
    pub reason: Option<String>,
    /// Where it stands.
    #[not_personal]
    pub state: ExportState,
    /// When it was asked for; the file is deleted [`EXPORT_KEPT`] after.
    #[not_personal]
    pub asked_at: Timestamp,
    /// When the file was ready.
    #[not_personal]
    pub ready_at: Option<Timestamp>,
    /// The size of the file in bytes, once it is ready.
    #[not_personal]
    pub bytes: Option<i64>,
}

impl DataExport {
    /// Whether an administrator asked for it for somebody else.
    #[must_use]
    pub fn by_somebody_else(&self) -> bool {
        self.asked_by != self.person
    }

    /// The moment the file and its record are deleted.
    #[must_use]
    pub fn deleted_at(&self) -> Timestamp {
        self.asked_at
            .saturating_add(EXPORT_KEPT)
            .unwrap_or(self.asked_at)
    }

    /// The name of the file in the `exports/` folder: the ID and nothing a person typed.
    #[must_use]
    pub fn file_name(&self) -> String {
        format!("{}.zip", self.id)
    }
}
