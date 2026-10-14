//! Domain events (ADR-0007): the facts every consumer reads.
//!
//! A use case appends one event per change, in the same transaction as the change. History,
//! notifications, webhooks, the security log, the search index and live updates read the same
//! events. An event contains personal data (who did it, what changed), so it carries the marks of
//! ADR-0015 and follows the keep-and-delete rules.

use std::fmt;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::id::{EntityId, EventId, UserId};
use crate::personal::Entity;

/// Who did it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum Actor {
    /// A person.
    Person(UserId),
    /// Bugaboo itself, for example a background job.
    System,
}

words! {
    /// Where the change came from.
    pub enum Channel {
        /// The browser app.
        Ui => "ui",
        /// The API, called by a script or app.
        Api => "api",
        /// An outside AI app connected through MCP.
        Mcp => "mcp",
        /// The built-in assistant.
        Assistant => "assistant",
        /// Bugaboo itself.
        System => "system",
    }
}

/// The text is not the name of a kind of record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidKind;

impl fmt::Display for InvalidKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "a kind of record is lower-case words joined by underscores, like time_entry",
        )
    }
}

impl std::error::Error for InvalidKind {}

/// The kind of a record, like `issue` or `time_entry`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct EntityKind(String);

impl EntityKind {
    /// The kind named `name`.
    ///
    /// # Errors
    /// Fails unless `name` is lower-case words joined by single underscores.
    pub fn new(name: &str) -> Result<Self, InvalidKind> {
        let plain = name.starts_with(|c: char| c.is_ascii_lowercase())
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            && !name.contains("__")
            && !name.ends_with('_');
        if plain {
            Ok(Self(name.to_owned()))
        } else {
            Err(InvalidKind)
        }
    }

    /// The name of the kind.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for EntityKind {
    type Error = InvalidKind;

    fn try_from(name: String) -> Result<Self, InvalidKind> {
        Self::new(&name)
    }
}

impl From<EntityKind> for String {
    fn from(kind: EntityKind) -> Self {
        kind.0
    }
}

words! {
    /// The kinds of record the use cases write events and security-log entries about. The word
    /// is the [`EntityKind`] that is stored, so a kind is spelled once and a misspelling does not
    /// compile.
    pub enum RecordKind {
        /// An access key.
        AccessKey => "access_key",
        /// One run of a change to many issues (ISS-09).
        BulkRun => "bulk_run",
        /// An own field (CF-01).
        Field => "field",
        /// One row of *Who has which role*.
        Assignment => "assignment",
        /// A comment on an issue.
        Comment => "comment",
        /// Who follows an issue (the record is the issue).
        Follower => "follower",
        /// A group of people.
        Group => "group",
        /// An invitation.
        Invitation => "invitation",
        /// An issue.
        Issue => "issue",
        /// An issue type.
        IssueType => "issue_type",
        /// A label of the organization's list (TAG-02).
        Label => "label",
        /// A group of labels (TAG-02).
        LabelGroup => "label_group",
        /// A passkey.
        Passkey => "passkey",
        /// A person.
        Person => "person",
        /// A project.
        Project => "project",
        /// One relation between two issues.
        Relation => "relation",
        /// A layout of the issue page or the new-issue form (CFG-01).
        Layout => "layout",
        /// A kind of relation.
        RelationKind => "relation_kind",
        /// A role.
        Role => "role",
        /// A sign-in session.
        Session => "session",
        /// A setting.
        Setting => "setting",
        /// The setup code of the first start.
        SetupCode => "setup_code",
        /// The making of the starting roles.
        StartingRoles => "starting_roles",
        /// A tag (TAG-01).
        Tag => "tag",
    }
}

impl From<RecordKind> for EntityKind {
    fn from(kind: RecordKind) -> Self {
        // Every word of `RecordKind` is a valid name (a test checks it).
        Self(kind.code().to_owned())
    }
}

/// The record an event is about.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityRef {
    /// What kind of record it is.
    pub kind: EntityKind,
    /// Which one.
    pub id: EntityId,
}

// A kind name and an ID of a record: no personal data of their own (see `EntityId`).
impl crate::personal::NotPersonal for EntityKind {}
impl crate::personal::NotPersonal for EntityRef {}

/// One field that changed, with its value before and after. `None` means the field had no value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldChange {
    /// The name of the field.
    pub field: String,
    /// The value before the change.
    pub before: Option<serde_json::Value>,
    /// The value after the change.
    pub after: Option<serde_json::Value>,
}

/// What happened, to whom, by whom, and why (ADR-0007).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Entity)]
pub struct DomainEvent {
    /// The ID of the event.
    #[not_personal]
    pub id: EventId,
    /// When it happened.
    #[not_personal]
    pub time: Timestamp,
    /// Who did it.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub actor: Actor,
    /// Where the change came from.
    #[not_personal]
    pub channel: Channel,
    /// The record it is about.
    #[not_personal]
    pub entity: EntityRef,
    /// The fields that changed. They can hold anything a person typed, but never credentials,
    /// password hashes or secrets: use cases leave those fields out of the changes (review).
    #[personal(category = "content", basis = "legitimate_interests")]
    pub changes: Vec<FieldChange>,
    /// Why, when the person said so; for a change a job made as the person, the item of the job
    /// (a change to many issues, `BulkRun::item_reason`).
    #[personal(category = "content", basis = "legitimate_interests")]
    pub reason: Option<String>,
}

impl DomainEvent {
    /// Whether the event counted the version of its record up (it holds the change `version`).
    /// Events about a lock, a member of a group or a level do not.
    #[must_use]
    pub fn counted_version(&self) -> bool {
        self.changes
            .iter()
            .any(|change| change.field == crate::version::FIELD)
    }

    /// The version the record had after the event, when the event counted it up.
    #[must_use]
    pub fn version_after(&self) -> Option<i64> {
        self.changes
            .iter()
            .find(|change| change.field == crate::version::FIELD)?
            .after
            .as_ref()?
            .as_i64()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::id::SequentialIds;
    use crate::personal::{Category, FieldMark, Personal};

    fn sample() -> DomainEvent {
        let mut ids = SequentialIds::default();
        DomainEvent {
            id: EventId::generate(&mut ids).unwrap(),
            time: "2026-10-04T08:30:00Z".parse().unwrap(),
            actor: Actor::Person(UserId::generate(&mut ids)),
            channel: Channel::Ui,
            entity: EntityRef {
                kind: EntityKind::new("issue").unwrap(),
                id: UserId::generate(&mut ids).into(),
            },
            changes: vec![FieldChange {
                field: "status".to_owned(),
                before: Some(json!("Open")),
                after: Some(json!("Fixing")),
            }],
            reason: Some("Started work".to_owned()),
        }
    }

    #[test]
    fn an_event_survives_json() {
        let event = sample();
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(serde_json::from_str::<DomainEvent>(&json).unwrap(), event);
    }

    #[test]
    fn the_version_after_is_the_one_the_event_counted_up_to() {
        let mut event = sample();
        assert_eq!(event.version_after(), None);
        assert!(!event.counted_version());
        event.changes.push(FieldChange {
            field: crate::version::FIELD.to_owned(),
            before: Some(json!(3)),
            after: Some(json!(4)),
        });
        assert_eq!(event.version_after(), Some(4));
        assert!(event.counted_version());
    }

    #[test]
    fn every_kind_of_record_is_a_valid_name_for_an_entity_kind() {
        for kind in RecordKind::ALL {
            assert_eq!(
                EntityKind::new(kind.code()).as_ref(),
                Ok(&EntityKind::from(*kind))
            );
        }
    }

    #[test]
    fn channels_are_written_as_their_codes() {
        for channel in Channel::ALL {
            assert_eq!(
                serde_json::to_value(channel).unwrap(),
                json!(channel.code())
            );
        }
    }

    #[test]
    fn actor_is_tagged_so_a_system_event_has_no_person() {
        assert_eq!(
            serde_json::to_value(Actor::System).unwrap(),
            json!({"kind": "system"})
        );
        let person = UserId::generate(&mut SequentialIds::default());
        assert_eq!(
            serde_json::to_value(Actor::Person(person)).unwrap(),
            json!({"kind": "person", "id": person.to_string()})
        );
    }

    #[test]
    fn kinds_are_lower_case_words() {
        assert!(EntityKind::new("time_entry").is_ok());
        for bad in ["", "Issue", "time__entry", "time_", "time entry", "1issue"] {
            assert_eq!(EntityKind::new(bad), Err(InvalidKind), "{bad}");
        }
        assert!(serde_json::from_str::<EntityKind>("\"Not Valid\"").is_err());
    }

    #[test]
    fn priv_02_actor_and_changes_carry_personal_data_marks() {
        let marks = <DomainEvent as Entity>::MARKS;
        let personal: Vec<&str> = marks
            .fields()
            .iter()
            .filter(|field| matches!(field.personal_data(), Personal::Yes { .. }))
            .map(FieldMark::name)
            .collect();
        assert_eq!(personal, ["actor", "changes", "reason"]);
        let actor = marks
            .fields()
            .iter()
            .find(|field| field.name() == "actor")
            .unwrap();
        assert!(matches!(
            actor.personal_data(),
            Personal::Yes {
                category: Category::Activity,
                ..
            }
        ));
    }
}
