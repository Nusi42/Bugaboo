//! *Download my data* (PRIV-03, ADR-0044, decisions 2 to 4): the record of one file of a person's
//! data, who asked for it, for whom, when and in which state it is. The file itself lies in the
//! `exports/` folder of the data folder, named by the ID of its record and nothing a person typed.

use jiff::{SignedDuration, Timestamp};

use crate::files::{LINK_LIFETIME, SignedLink};
use crate::id::{DataExportId, UserId};
use crate::personal::Entity;

/// How many days the file of an export is kept after it was asked for (ADR-0044, decision 2): then
/// it is deleted with its record. The row *Data exports* of the keep rules shows it.
pub const EXPORT_KEPT_DAYS: i64 = 7;

/// The same as a length of time.
pub const EXPORT_KEPT: SignedDuration = crate::time::days(EXPORT_KEPT_DAYS);

/// How often a person's data may be asked for in a day, by the person and by administrators
/// together: each file is written in full and takes room on the disk until it is deleted.
pub const EXPORT_ASKS_PER_DAY: u32 = 3;

/// The day the asks are counted in: the time before an ask in which [`EXPORT_ASKS_PER_DAY`] asks
/// may have been made. The records of the exports are the count, so it holds across restarts and
/// for every server of the database; the records of that time stay until it is over.
pub const EXPORT_ASKS_WINDOW: SignedDuration = crate::time::days(1);

/// How many seconds an ask at `now` has to wait, when the asks made at `asked` (the moments of the
/// exports of the person in [`EXPORT_ASKS_WINDOW`] before `now`) leave none for the day: until the
/// oldest of them is a day old, at least one second. `None` when the ask may be made.
#[must_use]
pub fn wait_for_ask(asked: &[Timestamp], now: Timestamp) -> Option<u64> {
    if asked.len() < usize::try_from(EXPORT_ASKS_PER_DAY).unwrap_or(usize::MAX) {
        return None;
    }
    let free_at = asked.iter().min()?.checked_add(EXPORT_ASKS_WINDOW).ok()?;
    let seconds = free_at.duration_since(now).as_secs();
    Some(u64::try_from(seconds).unwrap_or(0).max(1))
}

/// The type the file of an export is sent as.
pub const EXPORT_MEDIA_TYPE: &str = "application/zip";

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
    /// Whether the file of the export is still within its keep period at `now`: it is deleted
    /// [`EXPORT_KEPT`] after it was asked for, and is not handed out after that, also while the
    /// job that deletes it has not run yet.
    #[must_use]
    pub fn kept_at(&self, now: Timestamp) -> bool {
        self.asked_less_than(EXPORT_KEPT, now)
    }

    /// Whether the export still counts against the asks of the day at `now`: it was asked for less
    /// than [`EXPORT_ASKS_WINDOW`] before.
    #[must_use]
    pub fn counts_as_ask_at(&self, now: Timestamp) -> bool {
        self.asked_less_than(EXPORT_ASKS_WINDOW, now)
    }

    /// Whether the export was asked for less than `length` before `now`.
    fn asked_less_than(&self, length: SignedDuration, now: Timestamp) -> bool {
        self.asked_at.checked_add(length).is_ok_and(|end| end > now)
    }

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
/// [`LINK_LIFETIME`]; their first byte names the kind ([`SignedLink`]), so neither is read as the
/// other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportLink {
    /// The export.
    pub export: DataExportId,
    /// The one person it is for: who asked for the export.
    pub person: UserId,
    /// The moment it stops working.
    pub expires_at: Timestamp,
}

/// The bytes of a link: its kind, the export, the person and the end, in this order.
const EXPORT_LINK_BYTES: usize = 1 + 16 + 16 + 8;

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
        bytes[0] = SignedLink::Export as u8;
        bytes[1..17].copy_from_slice(&self.export.to_bytes());
        bytes[17..33].copy_from_slice(&self.person.to_bytes());
        bytes[33..].copy_from_slice(&self.expires_at.as_microsecond().to_be_bytes());
        bytes
    }

    /// The link written in `bytes`, if they are one.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes: &[u8; EXPORT_LINK_BYTES] = bytes.try_into().ok()?;
        if bytes[0] != SignedLink::Export as u8 {
            return None;
        }
        Some(Self {
            export: DataExportId::from_bytes(bytes[1..17].try_into().ok()?).ok()?,
            person: UserId::from_bytes(bytes[17..33].try_into().ok()?).ok()?,
            expires_at: Timestamp::from_microsecond(i64::from_be_bytes(
                bytes[33..].try_into().ok()?,
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
    fn priv_03_a_fourth_ask_of_a_day_waits_until_the_first_is_a_day_old() {
        let now = Timestamp::UNIX_EPOCH
            .checked_add(SignedDuration::from_hours(100))
            .unwrap_or(Timestamp::UNIX_EPOCH);
        let hours_ago = |hours: i64| {
            now.checked_sub(SignedDuration::from_hours(hours))
                .unwrap_or(Timestamp::UNIX_EPOCH)
        };
        assert_eq!(wait_for_ask(&[], now), None);
        assert_eq!(wait_for_ask(&[hours_ago(1), hours_ago(2)], now), None);
        // Three asks in the day: the next waits for the oldest to be a day old.
        let three = [hours_ago(1), hours_ago(20), hours_ago(3)];
        assert_eq!(wait_for_ask(&three, now), Some(4 * 3600));
        // One that is a day old exactly as the next comes waits a second at least.
        let edge = [hours_ago(24), hours_ago(1), hours_ago(2)];
        assert_eq!(wait_for_ask(&edge, now), Some(1));
    }

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
        // The same bytes with the kind of a link of a file are no link of an export, and a link of
        // a file of any length is none either.
        let mut other_kind = bytes;
        other_kind[0] = SignedLink::File as u8;
        assert!(ExportLink::from_bytes(&other_kind).is_none());
        let file = DownloadLink::new(
            crate::id::AttachmentId::generate(&mut ids).map_err(|error| error.to_string())?,
            crate::files::LinkTarget::File,
            person,
            now,
        )
        .ok_or("a link of a file")?;
        assert!(ExportLink::from_bytes(&file.to_bytes()).is_none());
        assert!(ExportLink::from_bytes(&file.to_bytes()[1..]).is_none());
        assert!(link.works_at(now));
        let end = now
            .checked_add(LINK_LIFETIME)
            .map_err(|error| error.to_string())?;
        assert!(!link.works_at(end));
        Ok(())
    }
}
