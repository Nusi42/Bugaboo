//! The pieces of an event: the record it is about, who did it, and which fields changed.

use bb_core::event::{Actor, EntityRef, FieldChange, RecordKind};
use bb_core::id::{EntityId, UserId};

use super::error::AppError;

/// The record `id` of a kind, for events and the security log.
pub(crate) fn entity(kind: RecordKind, id: impl Into<EntityId>) -> EntityRef {
    EntityRef {
        kind: kind.into(),
        id: id.into(),
    }
}

/// The record that is one fixed thing (a setting, the setup code of the first start): its ID is
/// made from its name, so every event about it names the same record.
pub(crate) fn fixed_entity(kind: RecordKind, name: &str) -> Result<EntityRef, AppError> {
    let id = EntityId::from_bytes(bb_auth::stable_id(kind.code(), name))
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(entity(kind, id))
}

/// A field of a change whose value is not shown (credentials): only the fact is recorded.
pub(crate) fn changed(field: &str) -> FieldChange {
    FieldChange {
        field: field.to_owned(),
        before: None,
        after: None,
    }
}

/// A field of a change with its new value.
pub(crate) fn set_to(field: &str, value: serde_json::Value) -> FieldChange {
    FieldChange {
        field: field.to_owned(),
        before: None,
        after: Some(value),
    }
}

/// The actor of an event made by a person.
pub(crate) const fn by(person: UserId) -> Actor {
    Actor::Person(person)
}
