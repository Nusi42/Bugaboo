//! *Download my data* (PRIV-03, ADR-0044, decisions 2 to 4): the record of one file of a person's
//! data, who asked for it, for whom, when and in which state it is. The file itself lies in the
//! `exports/` folder of the data folder, named by the ID of its record and nothing a person typed.

use jiff::{SignedDuration, Timestamp};

use crate::files::LINK_LIFETIME;
use crate::id::{DataExportId, UserId};
use crate::personal::Entity;

/// How long the file of an export is kept after it was asked for (ADR-0044, decision 2): then it is
/// deleted with its record.
pub const EXPORT_KEPT: SignedDuration = SignedDuration::from_hours(7 * 24);

/// The type the file of an export is sent as.
pub const EXPORT_MEDIA_TYPE: &str = "application/zip";

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
    ExportState as "DataExportStatus",
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

/// What the signed link of the download of an export says: which export, for whom, until when. The
/// server signs these bytes like the links of files (`bb_auth::LinkKey`), good for the same
/// [`LINK_LIFETIME`]; their length differs from a link of a file, so neither is read as the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportLink {
    /// The export.
    pub export: DataExportId,
    /// The one person it is for: who asked for the export.
    pub person: UserId,
    /// The moment it stops working.
    pub expires_at: Timestamp,
}

/// The bytes of a link: the export, the person and the end, in this order.
const EXPORT_LINK_BYTES: usize = 16 + 16 + 8;

impl ExportLink {
    /// The link of `export` for `person`, good for [`LINK_LIFETIME`] from `now`; `None` when the
    /// end is beyond the moments that can be written.
    #[must_use]
    pub fn new(export: DataExportId, person: UserId, now: Timestamp) -> Option<Self> {
        Some(Self {
            export,
            person,
            expires_at: now.checked_add(LINK_LIFETIME).ok()?,
        })
    }

    /// The bytes that are signed.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; EXPORT_LINK_BYTES] {
        let mut bytes = [0_u8; EXPORT_LINK_BYTES];
        bytes[..16].copy_from_slice(&self.export.to_bytes());
        bytes[16..32].copy_from_slice(&self.person.to_bytes());
        bytes[32..].copy_from_slice(&self.expires_at.as_microsecond().to_be_bytes());
        bytes
    }

    /// The link written in `bytes`, if they are one.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes: &[u8; EXPORT_LINK_BYTES] = bytes.try_into().ok()?;
        Some(Self {
            export: DataExportId::from_bytes(bytes[..16].try_into().ok()?).ok()?,
            person: UserId::from_bytes(bytes[16..32].try_into().ok()?).ok()?,
            expires_at: Timestamp::from_microsecond(i64::from_be_bytes(
                bytes[32..].try_into().ok()?,
            ))
            .ok()?,
        })
    }

    /// Whether the link still works at `now`.
    #[must_use]
    pub fn works_at(&self, now: Timestamp) -> bool {
        now < self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::DownloadLink;
    use crate::id::SequentialIds;

    #[test]
    fn priv_03_the_link_of_an_export_reads_back_and_is_no_link_of_a_file() -> Result<(), String> {
        let mut ids = SequentialIds::default();
        let export = DataExportId::generate(&mut ids).map_err(|error| error.to_string())?;
        let person = UserId::generate(&mut ids);
        let now = Timestamp::UNIX_EPOCH;
        let link = ExportLink::new(export, person, now).ok_or("a link")?;
        let bytes = link.to_bytes();
        assert_eq!(ExportLink::from_bytes(&bytes), Some(link));
        assert!(DownloadLink::from_bytes(&bytes).is_none());
        assert!(link.works_at(now));
        let end = now
            .checked_add(LINK_LIFETIME)
            .map_err(|error| error.to_string())?;
        assert!(!link.works_at(end));
        Ok(())
    }
}
