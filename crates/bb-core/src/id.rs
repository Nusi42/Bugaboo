//! IDs: one newtype per kind of record, all `UUIDv7` inside (ADR-0021, coding-standards §3).
//!
//! The UUID itself never appears in a public signature, so an ID of one kind cannot be passed
//! where another is expected. The readable issue ID (`WEB-K7Q2X`) is a different thing and comes
//! with the issues in M3.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// The only UUID version used for IDs: the first bits are the time, so new records sort last.
const VERSION: usize = 7;

/// Why a text or a byte string is not an ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdError {
    /// The text is not a UUID in the usual form (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`).
    NotAnId,
    /// It is a UUID, but not version 7.
    WrongVersion,
}

impl fmt::Display for IdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotAnId => {
                "not an ID: expected 36 characters, like 0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c"
            }
            Self::WrongVersion => "not an ID: only version 7 UUIDs are used",
        })
    }
}

impl std::error::Error for IdError {}

/// Reads the one spelling IDs have. `Uuid::try_parse` alone would also take braces, URNs and
/// the form without hyphens.
fn parse(text: &str) -> Result<Uuid, IdError> {
    if text.len() != 36 {
        return Err(IdError::NotAnId);
    }
    check(Uuid::try_parse(text).map_err(|_| IdError::NotAnId)?)
}

fn check(uuid: Uuid) -> Result<Uuid, IdError> {
    if uuid.get_version_num() == VERSION {
        Ok(uuid)
    } else {
        Err(IdError::WrongVersion)
    }
}

/// Declares one ID type. Every ID has the same methods, so they are written here once.
macro_rules! define_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Uuid);

        impl $name {
            /// A new ID, made now.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// The ID with these 16 bytes, as stored.
            ///
            /// # Errors
            /// Fails when the bytes are not a version 7 UUID.
            pub fn from_bytes(bytes: [u8; 16]) -> Result<Self, IdError> {
                check(Uuid::from_bytes(bytes)).map(Self)
            }

            /// The 16 bytes of this ID, as stored.
            #[must_use]
            pub const fn to_bytes(self) -> [u8; 16] {
                *self.0.as_bytes()
            }

            /// When the ID was made, in milliseconds since 1970.
            #[must_use]
            pub fn created_at_millis(self) -> u64 {
                self.0.get_timestamp().map_or(0, |timestamp| {
                    let (seconds, nanos) = timestamp.to_unix();
                    seconds * 1000 + u64::from(nanos / 1_000_000)
                })
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0.hyphenated(), formatter)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}({})", stringify!($name), self.0.hyphenated())
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(text: &str) -> Result<Self, IdError> {
                parse(text).map(Self)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
                text.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

define_id! {
    /// The ID of a person's account.
    UserId
}

define_id! {
    /// The ID of a project.
    ProjectId
}

define_id! {
    /// The ID of an issue (the internal one; the readable issue ID comes in M3, ADR-0021).
    IssueId
}

define_id! {
    /// The ID of a domain event (ADR-0007).
    EventId
}

define_id! {
    /// The ID of a record whose kind is named next to it, for example in an event (ADR-0007).
    EntityId
}

/// Every kind of ID can stand for "some record", next to the name of its kind.
macro_rules! into_entity_id {
    ($($name:ident),+) => {
        $(impl From<$name> for EntityId {
            fn from(id: $name) -> Self {
                Self(id.0)
            }
        })+
    };
}

into_entity_id!(UserId, ProjectId, IssueId, EventId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_ids_are_version_7_and_sort_by_time() {
        let first = IssueId::new();
        let second = IssueId::new();
        assert_eq!(first.0.get_version_num(), 7);
        assert!(first.created_at_millis() <= second.created_at_millis());
        assert!(first.created_at_millis() > 1_700_000_000_000);
    }

    #[test]
    fn display_and_parse_round_trip() {
        let id = UserId::new();
        let text = id.to_string();
        assert_eq!(text.len(), 36);
        assert_eq!(text.parse::<UserId>(), Ok(id));
        assert_eq!(UserId::from_bytes(id.to_bytes()), Ok(id));
    }

    #[test]
    fn only_the_plain_form_of_a_version_7_uuid_is_accepted() {
        let good = "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c";
        assert!(good.parse::<ProjectId>().is_ok());
        assert!(good.to_uppercase().parse::<ProjectId>().is_ok());
        for text in [
            "",
            "0190f4c27a3e7c1d9b523d6e8f1a2b4c",
            "{0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c}",
            "urn:uuid:0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c",
            "WEB-142",
        ] {
            assert_eq!(text.parse::<ProjectId>(), Err(IdError::NotAnId), "{text}");
        }
        assert_eq!(
            "550e8400-e29b-41d4-a716-446655440000".parse::<ProjectId>(),
            Err(IdError::WrongVersion)
        );
    }

    #[test]
    fn serde_uses_the_text_form() {
        let id = EventId::new();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{id}\""));
        assert_eq!(serde_json::from_str::<EventId>(&json).unwrap(), id);
        assert!(serde_json::from_str::<EventId>("\"nope\"").is_err());
        assert!(serde_json::from_str::<EventId>("7").is_err());
    }

    #[test]
    fn every_kind_converts_to_a_generic_entity_id() {
        let user = UserId::new();
        assert_eq!(EntityId::from(user).to_bytes(), user.to_bytes());
    }

    #[test]
    fn debug_names_the_kind_and_not_the_wrapper() {
        let id = UserId::new();
        assert_eq!(format!("{id:?}"), format!("UserId({id})"));
    }
}
