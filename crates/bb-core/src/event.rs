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

/// Where the change came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// The browser app.
    Ui,
    /// The API, called by a script or app.
    Api,
    /// An outside AI app connected through MCP.
    Mcp,
    /// The built-in assistant.
    Assistant,
    /// Bugaboo itself.
    System,
}

impl Channel {
    /// Every channel.
    pub const ALL: [Self; 5] = [
        Self::Ui,
        Self::Api,
        Self::Mcp,
        Self::Assistant,
        Self::System,
    ];

    /// The word used in the API and the event log.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Ui => "ui",
            Self::Api => "api",
            Self::Mcp => "mcp",
            Self::Assistant => "assistant",
            Self::System => "system",
        }
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

/// The record an event is about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityRef {
    /// What kind of record it is.
    pub kind: EntityKind,
    /// Which one.
    pub id: EntityId,
}

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
    /// The fields that changed. They can hold anything a person typed.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub changes: Vec<FieldChange>,
    /// Why, when the person said so.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::personal::{Category, FieldMark, Personal};

    fn sample() -> DomainEvent {
        DomainEvent {
            id: EventId::new(),
            time: "2026-10-04T08:30:00Z".parse().unwrap(),
            actor: Actor::Person(UserId::new()),
            channel: Channel::Ui,
            entity: EntityRef {
                kind: EntityKind::new("issue").unwrap(),
                id: UserId::new().into(),
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
        let person = UserId::new();
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
