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
    /// Who or what made a backup or ran a restore.
    pub enum BackupMaker {
        /// The schedule of the organization (`backups.daily`).
        Schedule => "schedule",
        /// Somebody on the command line of the server (`bugaboo backup`, `bugaboo restore`).
        CommandLine => "command-line",
        /// A person with *Change organization settings*, with the button.
        Person => "person",
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

/// The kind of record the events and the security log name a backup by.
pub const ENTITY_KIND: &str = "backup";

impl BackupRun {
    /// What the record is, for an event or a line of the security log.
    ///
    /// # Errors
    /// Never: the kind is a plain word (by the tests).
    pub fn entity(&self) -> Result<EntityRef, InvalidKind> {
        Ok(EntityRef {
            kind: EntityKind::new(ENTITY_KIND)?,
            id: EntityId::from(self.id),
        })
    }

    /// The action of the line of the security log that the backup or restore writes.
    #[must_use]
    pub const fn security_action(&self) -> SecurityAction {
        match self.event {
            BackupEvent::Made => SecurityAction::BackupMade,
            BackupEvent::Restored => SecurityAction::BackupRestored,
        }
    }

    /// The domain event of the record (ADR-0007): who made it, and its name, size, checksum and
    /// the lost range of a restore.
    ///
    /// # Errors
    /// Never: the kind is a plain word (by the tests).
    pub fn domain_event(
        &self,
        id: EventId,
        actor: Actor,
        channel: Channel,
    ) -> Result<DomainEvent, InvalidKind> {
        let change = |field: &str, after: serde_json::Value| FieldChange {
            field: field.to_owned(),
            before: None,
            after: Some(after),
        };
        let mut changes = vec![
            change("event", self.event.code().into()),
            change("name", self.name.to_string().into()),
            change("size", self.size.into()),
            change(
                "checksum",
                crate::hex::encode(self.checksum.as_bytes()).into(),
            ),
            change("maker", self.maker.code().into()),
        ];
        if let (Some(from), Some(to)) = (self.lost_from, self.lost_to) {
            changes.push(change("lost_from", from.into()));
            changes.push(change("lost_to", to.into()));
        }
        Ok(DomainEvent {
            id,
            time: self.at,
            actor,
            channel,
            entity: self.entity()?,
            changes,
            reason: None,
            run: None,
        })
    }
}
