//! The record of one backup made or restored (OPS-07, migration 54): its name, size and
//! checksum, who or what made it, and for a restore the entries of the security log that are gone.
//! Every backup and every restore writes one line to the security log that points to its record
//! (ADR-0046, decisions 3 and 4).

use jiff::Timestamp;

use super::name::BackupName;
use crate::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef, FieldChange, InvalidKind};
use crate::id::{BackupId, EntityId, EventId, UserId};
use crate::personal::{Entity, NotPersonal};
use crate::security_log::Digest;
use crate::sign_in::SecurityAction;

// The name of a backup holds a moment and a version, nothing about a person.
impl NotPersonal for BackupName {}

/// The field of the event of a backup that says what happened to it.
pub const EVENT_FIELD: &str = "event";

/// The field of the event of a backup that says who or what made it.
pub const MAKER_FIELD: &str = "maker";

/// What the event of a backup asked for with the button says happened: it was asked for, and the
/// long lane of the jobs makes it (ADR-0046, decision 3). The record of the same ID follows when it
/// is made.
pub const ASKED: &str = "asked";

/// The field of the event of a backup that holds the size of its file.
pub const SIZE_FIELD: &str = "size";
/// The field of the event of a backup that holds the checksum of its file.
pub const CHECKSUM_FIELD: &str = "checksum";

words! {
    /// What happened to a backup.
    pub enum BackupEvent {
        /// It was made.
        Made => "made",
        /// It was restored.
        Restored => "restored",
    }
}

words! {
    /// Where a backup asked for with the button stands until it is made (ADR-0046, decision 3).
    pub enum AskedBackupState {
        /// Asked for; the job that makes it waits or works.
        Running => "running",
        /// The job gave up; no backup was made. The person may ask again.
        Failed => "failed",
    }
}

words_in_api!(
    AskedBackupState as "AskedBackupStatus",
    "Where a backup asked for with *Create a backup now* stands until it is made: running, or failed."
);

words! {
    /// Who or what made a backup or ran a restore.
    pub enum BackupMaker {
        /// The schedule of the organization (`backups.daily`).
        Schedule => "schedule",
        /// Somebody on the command line of the server (`bugaboo backup`, `bugaboo restore`).
        CommandLine => "command-line",
        /// A person with *Change organization settings*, with the button.
        Person => "person",
        /// A new program, before it changed the database of an older one (ADR-0046, decision 5).
        Upgrade => "upgrade",
    }
}

/// One backup made or restored.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct BackupRun {
    /// The ID of the record, which the line of the security log names.
    #[not_personal]
    pub id: BackupId,
    /// Whether the backup was made or restored.
    #[not_personal]
    pub event: BackupEvent,
    /// The name of the backup file.
    #[not_personal]
    pub name: BackupName,
    /// The size of the file in bytes.
    #[not_personal]
    pub size: i64,
    /// The SHA-256 of the file.
    #[not_personal]
    pub checksum: Digest,
    /// Who or what made it.
    #[not_personal]
    pub maker: BackupMaker,
    /// The person who pressed the button: proof of who copied all the organization's data.
    #[personal(category = "identity", basis = "legitimate_interests")]
    #[person(actor)]
    pub person: Option<UserId>,
    /// When it was made or restored.
    #[not_personal]
    pub at: Timestamp,
    /// For a restore: the first entry of the security log that is gone.
    #[not_personal]
    pub lost_from: Option<i64>,
    /// For a restore: the last entry of the security log that is gone.
    #[not_personal]
    pub lost_to: Option<i64>,
}

/// A backup file as its record names it: its name, its size and its checksum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatedFile {
    /// Its name.
    pub name: BackupName,
    /// The size of the file in bytes.
    pub size: u64,
    /// The SHA-256 of the file.
    pub checksum: Digest,
}

/// The kind of record the events and the security log name a backup by.
pub const ENTITY_KIND: &str = "backup";

impl BackupRun {
    /// The record of `file`, made or restored at `at` by `maker` (and `person`, who pressed the
    /// button). A restore adds the range of the security log it lost with [`Self::with_lost`].
    #[must_use]
    pub fn new(
        id: BackupId,
        event: BackupEvent,
        file: &CreatedFile,
        maker: BackupMaker,
        person: Option<UserId>,
        at: Timestamp,
    ) -> Self {
        Self {
            id,
            event,
            name: file.name.clone(),
            size: i64::try_from(file.size).unwrap_or(i64::MAX),
            checksum: file.checksum,
            maker,
            person,
            at,
            lost_from: None,
            lost_to: None,
        }
    }

    /// The record with the first and last entry of the security log that a restore lost.
    #[must_use]
    pub const fn with_lost(mut self, lost: Option<(i64, i64)>) -> Self {
        if let Some((from, to)) = lost {
            self.lost_from = Some(from);
            self.lost_to = Some(to);
        }
        self
    }

    /// What the record is, for an event or a line of the security log.
    ///
    /// # Errors
    /// Never: the kind is a plain word (by the tests).
    pub fn entity(&self) -> Result<EntityRef, InvalidKind> {
        entity_of(self.id)
    }

    /// The action of the line of the security log that the backup or restore writes.
    #[must_use]
    pub const fn security_action(&self) -> SecurityAction {
        match self.event {
            BackupEvent::Made => SecurityAction::BackupMade,
            BackupEvent::Restored => SecurityAction::BackupRestored,
        }
    }

    /// What the event of the record says: what happened, its name, size, checksum, who or what
    /// made it, and the lost range of a restore.
    #[must_use]
    pub fn changes(&self) -> Vec<FieldChange> {
        let change = |field: &str, after: serde_json::Value| FieldChange {
            field: field.to_owned(),
            before: None,
            after: Some(after),
        };
        let mut changes = vec![
            change(EVENT_FIELD, self.event.code().into()),
            change("name", self.name.to_string().into()),
            change(SIZE_FIELD, self.size.into()),
            change(
                CHECKSUM_FIELD,
                crate::hex::encode(self.checksum.as_bytes()).into(),
            ),
            change(MAKER_FIELD, self.maker.code().into()),
        ];
        if let (Some(from), Some(to)) = (self.lost_from, self.lost_to) {
            changes.push(change("lost_from", from.into()));
            changes.push(change("lost_to", to.into()));
        }
        changes
    }

    /// The domain event of the record (ADR-0007), for what writes it outside a request: the
    /// command line and the restore.
    ///
    /// # Errors
    /// Never: the kind is a plain word (by the tests).
    pub fn domain_event(
        &self,
        id: EventId,
        actor: Actor,
        channel: Channel,
    ) -> Result<DomainEvent, InvalidKind> {
        Ok(DomainEvent {
            id,
            time: self.at,
            actor,
            channel,
            entity: self.entity()?,
            changes: self.changes(),
            reason: None,
            run: None,
        })
    }
}

/// What the events and the security log name the backup with this ID by.
///
/// # Errors
/// Never: the kind is a plain word (by the tests).
pub fn entity_of(id: BackupId) -> Result<EntityRef, InvalidKind> {
    Ok(EntityRef {
        kind: EntityKind::new(ENTITY_KIND)?,
        id: EntityId::from(id),
    })
}

/// What the event of asking for a backup says: that it was asked for, and by whom.
#[must_use]
pub fn asked_changes(maker: BackupMaker) -> Vec<FieldChange> {
    [(EVENT_FIELD, ASKED), (MAKER_FIELD, maker.code())]
        .into_iter()
        .map(|(field, after)| FieldChange {
            field: field.to_owned(),
            before: None,
            after: Some(after.into()),
        })
        .collect()
}
