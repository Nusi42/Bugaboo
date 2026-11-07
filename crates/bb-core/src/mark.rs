//! The mark of a text written with AI (COMP-08, COMP-09, ADR-0013): *Written with the assistant*,
//! *Written by an AI app* or *Written by an agent*. It is stored with each text that can carry it
//! (the description of an issue, a comment, a text of an own field), set from the actor and the
//! channel of the change and never from what a client sends, shown beside the text, given in the
//! API and in every export.
//!
//! A text a person writes by hand has no mark. A person who edits a marked text keeps the mark;
//! the history says who changed it.

use serde::{Deserialize, Serialize};

use crate::event::{Actor, Channel, EntityKind, FieldChange, RecordKind};
use crate::id::{AgentId, EntityId, FieldId};
use crate::personal::Entity;

words! {
    /// Who wrote a text with AI.
    pub enum WrittenBy {
        /// The built-in assistant, applied by a person.
        Assistant => "assistant",
        /// An outside AI app connected through MCP.
        App => "app",
        /// An agent.
        Agent => "agent",
    }
}

words_in_api!(WrittenBy, "How a text that AI wrote was written.");

/// The mark a change by `actor` through `channel` puts on the texts it writes; none for a person
/// writing by hand. The one function that decides it.
#[must_use]
pub const fn written_by(actor: Actor, channel: Channel) -> Option<WrittenBy> {
    match (actor, channel) {
        (Actor::Agent { .. }, _) => Some(WrittenBy::Agent),
        (_, Channel::Assistant) => Some(WrittenBy::Assistant),
        (_, Channel::Mcp) => Some(WrittenBy::App),
        _ => None,
    }
}

/// The part of a record that a member of an event is a text of: the description of an issue, the
/// text of a comment and the value of an own field when it is text. None for every other member.
#[must_use]
pub fn text_part(kind: &EntityKind, change: &FieldChange) -> Option<TextPart> {
    let is_text = change
        .after
        .as_ref()
        .is_some_and(serde_json::Value::is_string);
    let issue = EntityKind::from(RecordKind::Issue);
    let comment = EntityKind::from(RecordKind::Comment);
    let text_of_record = (*kind == issue && change.field == "description")
        || (*kind == comment && change.field == "text")
        || (*kind == issue && change.field.parse::<FieldId>().is_ok());
    (text_of_record && is_text).then(|| TextPart(change.field.clone()))
}

/// The part of a record that holds a text: `description`, `text`, or the ID of an own field. Never
/// anything a person typed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TextPart(String);

impl TextPart {
    /// The part with this name, as it is stored.
    #[must_use]
    pub const fn from_stored(name: String) -> Self {
        Self(name)
    }

    /// The name of the part.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl crate::personal::NotPersonal for TextPart {}

/// The mark of one text, as it is stored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Entity)]
pub struct TextMark {
    /// The kind of record that holds the text.
    #[not_personal]
    pub entity_kind: EntityKind,
    /// The record.
    #[not_personal]
    pub entity_id: EntityId,
    /// The part of the record: `description`, `text`, or the ID of an own field.
    #[not_personal]
    pub part: TextPart,
    /// How the text was written.
    #[not_personal]
    pub written_by: WrittenBy,
    /// The name of the AI app, for `App`.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub app: Option<String>,
    /// The agent, for `Agent`.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub agent: Option<AgentId>,
}
