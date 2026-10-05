//! IDs: one newtype per kind of record (ADR-0021, coding-standards §3). Records are `UUIDv7`
//! inside, so they sort by creation time; a person's ID is a random `UUIDv4`, so it says nothing
//! about when the account was made (ADR-0015, amendment M1).
//!
//! No ID is made from the clock or the system's randomness here: this crate does no I/O and builds
//! for WebAssembly (ARCH-WASM). The caller passes the time and the random bytes, usually through an
//! [`IdSource`]. The UUID itself never appears in a public signature, so an ID of one kind cannot
//! be passed where another is expected. The readable issue ID (`WEB-K7Q2X`) is a different thing
//! and comes with the issues in M3.

use std::fmt;
use std::str::FromStr;

use jiff::Timestamp;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::{Builder, Uuid};

/// The version number that stands for "any version" (for `EntityId`, which names records of every kind).
const ANY_VERSION: usize = 0;

/// Why a text, a byte string or a moment is not usable for an ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdError {
    /// The text is not a UUID in the usual form (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`).
    NotAnId,
    /// It is a UUID, but not the version this kind of ID uses.
    WrongVersion,
    /// The moment is before 1970 and cannot be part of a time-ordered ID.
    BadTime,
}

impl fmt::Display for IdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotAnId => {
                "not an ID: expected 36 characters, like 0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c"
            }
            Self::WrongVersion => "not an ID of this kind: the UUID version is wrong",
            Self::BadTime => "an ID cannot be made for a moment before 1970",
        })
    }
}

impl std::error::Error for IdError {}

/// Where new IDs get their time and randomness. The server's context implements it with the real
/// clock and a secure random source (WP-05); this crate only knows the shape.
pub trait IdSource {
    /// The current moment.
    fn now(&mut self) -> Timestamp;

    /// 16 secure random bytes. Every call gives new ones.
    fn random(&mut self) -> [u8; 16];
}

/// A predictable [`IdSource`] for tests and in-memory fakes: it counts up and is not random, so
/// it must never make IDs in production. It exists only in this crate's own tests and behind the
/// `fake-ids` feature, which only `[dev-dependencies]` switch on.
#[cfg(any(test, feature = "fake-ids"))]
#[derive(Clone, Debug, Default)]
pub struct SequentialIds {
    counter: u64,
}

#[cfg(any(test, feature = "fake-ids"))]
impl SequentialIds {
    /// The first moment it hands out: 2026-01-01T00:00:00Z.
    const START_MILLIS: i64 = 1_767_225_600_000;
}

#[cfg(any(test, feature = "fake-ids"))]
impl IdSource for SequentialIds {
    fn now(&mut self) -> Timestamp {
        self.counter += 1;
        let offset = i64::try_from(self.counter).unwrap_or(i64::MAX);
        Timestamp::from_millisecond(Self::START_MILLIS.saturating_add(offset)).unwrap_or_default()
    }

    fn random(&mut self) -> [u8; 16] {
        self.counter += 1;
        let mut bytes = [0xA5; 16];
        bytes[8..].copy_from_slice(&self.counter.to_be_bytes());
        bytes
    }
}

/// Reads the one spelling IDs have. `Uuid::try_parse` alone would also take braces, URNs and
/// the form without hyphens.
fn parse(text: &str, version: usize) -> Result<Uuid, IdError> {
    if text.len() != 36 {
        return Err(IdError::NotAnId);
    }
    check(
        Uuid::try_parse(text).map_err(|_| IdError::NotAnId)?,
        version,
    )
}

fn check(uuid: Uuid, version: usize) -> Result<Uuid, IdError> {
    if version == ANY_VERSION || uuid.get_version_num() == version {
        Ok(uuid)
    } else {
        Err(IdError::WrongVersion)
    }
}

/// The methods every ID has, whatever its UUID version.
macro_rules! id_common {
    ($name:ident, $version:expr) => {
        impl $name {
            /// The ID with these 16 bytes, as stored.
            ///
            /// # Errors
            /// Fails when the bytes are not a UUID of the version this kind of ID uses.
            pub fn from_bytes(bytes: [u8; 16]) -> Result<Self, IdError> {
                check(Uuid::from_bytes(bytes), $version).map(Self)
            }

            /// The 16 bytes of this ID, as stored.
            #[must_use]
            pub const fn to_bytes(self) -> [u8; 16] {
                *self.0.as_bytes()
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
                parse(text, $version).map(Self)
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

/// An ID that sorts by creation time (`UUIDv7`).
macro_rules! define_ordered_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Uuid);

        id_common!($name, 7);

        impl crate::personal::NotPersonal for $name {}

        impl $name {
            /// An ID made at `time` from 10 random bytes.
            ///
            /// # Errors
            /// Fails for a moment before 1970.
            pub fn from_parts(time: Timestamp, random: [u8; 10]) -> Result<Self, IdError> {
                let millis = u64::try_from(time.as_millisecond()).map_err(|_| IdError::BadTime)?;
                Ok(Self(Builder::from_unix_timestamp_millis(millis, &random).into_uuid()))
            }

            /// A new ID, with time and randomness from `source`.
            ///
            /// # Errors
            /// Fails when the source gives a moment before 1970.
            pub fn generate(source: &mut impl IdSource) -> Result<Self, IdError> {
                let time = source.now();
                let mut random = [0; 10];
                random.copy_from_slice(&source.random()[..10]);
                Self::from_parts(time, random)
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
    };
}

/// An ID that is fully random (`UUIDv4`) and so carries no time.
macro_rules! define_random_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Uuid);

        id_common!($name, 4);

        impl $name {
            /// The ID made from 16 random bytes (6 bits are fixed by the UUID format).
            #[must_use]
            pub fn from_random(random: [u8; 16]) -> Self {
                Self(Builder::from_random_bytes(random).into_uuid())
            }

            /// A new ID, with randomness from `source`.
            #[must_use]
            pub fn generate(source: &mut impl IdSource) -> Self {
                Self::from_random(source.random())
            }
        }
    };
}

define_random_id! {
    /// The ID of a person's account. Fully random, so it does not reveal when the account was made.
    UserId
}

define_ordered_id! {
    /// The ID of a project.
    ProjectId
}

define_ordered_id! {
    /// The ID of a group of people.
    GroupId
}

define_ordered_id! {
    /// The ID of a role.
    RoleId
}

define_ordered_id! {
    /// The ID of one row of *Who has which role*.
    AssignmentId
}

define_ordered_id! {
    /// The ID of an issue (the internal one; the readable issue ID comes in M3, ADR-0021).
    IssueId
}

define_ordered_id! {
    /// The ID of a domain event (ADR-0007).
    EventId
}

define_ordered_id! {
    /// The ID of a background job (ADR-0017).
    JobId
}

define_random_id! {
    /// The ID of a sign-in session as people see it in *Where you're signed in*. It is not the
    /// secret in the cookie and opens nothing.
    SessionId
}

define_random_id! {
    /// The ID of a passkey a person registered.
    PasskeyId
}

define_random_id! {
    /// The ID of an invitation.
    InvitationId
}

// Random references to records; they say nothing about a person by themselves.
impl crate::personal::NotPersonal for SessionId {}
impl crate::personal::NotPersonal for PasskeyId {}
impl crate::personal::NotPersonal for InvitationId {}

/// The ID of a record whose kind is named next to it, for example in an event (ADR-0007). It can
/// hold the ID of any kind of record, so it takes any UUID version.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(Uuid);

id_common!(EntityId, ANY_VERSION);

// A generic record ID may point to a person's record. It counts as not personal here because the
// person is protected by the marks on what names them (the actor and changes of an event); see
// "Limits of the marks" in docs/privacy/personal-data.md.
impl crate::personal::NotPersonal for EntityId {}

/// Every kind of ID can stand for "some record", next to the name of its kind. The conversion
/// keeps the bytes.
macro_rules! into_entity_id {
    ($($name:ident),+) => {
        $(impl From<$name> for EntityId {
            fn from(id: $name) -> Self {
                Self(id.0)
            }
        })+
    };
}

into_entity_id!(
    UserId,
    ProjectId,
    GroupId,
    RoleId,
    AssignmentId,
    IssueId,
    EventId,
    JobId,
    SessionId,
    PasskeyId,
    InvitationId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_ids_are_version_7_and_sort_by_time() {
        let mut source = SequentialIds::default();
        let first = IssueId::generate(&mut source).unwrap();
        let second = IssueId::generate(&mut source).unwrap();
        assert_eq!(first.0.get_version_num(), 7);
        assert!(first < second);
        assert!(first.created_at_millis() < second.created_at_millis());
        assert_eq!(first.created_at_millis(), 1_767_225_600_001);
    }

    #[test]
    fn from_parts_puts_the_time_into_the_id() {
        let time: Timestamp = "2026-10-04T08:30:00Z".parse().unwrap();
        let id = ProjectId::from_parts(time, [7; 10]).unwrap();
        assert_eq!(
            i64::try_from(id.created_at_millis()).unwrap(),
            time.as_millisecond()
        );
        assert_eq!(ProjectId::from_parts(time, [7; 10]), Ok(id));
        let before_1970: Timestamp = "1969-12-31T23:59:59Z".parse().unwrap();
        assert_eq!(
            ProjectId::from_parts(before_1970, [0; 10]),
            Err(IdError::BadTime)
        );
    }

    #[test]
    fn person_ids_are_random_version_4() {
        let mut source = SequentialIds::default();
        let first = UserId::generate(&mut source);
        let second = UserId::generate(&mut source);
        assert_eq!(first.0.get_version_num(), 4);
        assert_ne!(first, second);
        assert!(first.to_string().parse::<UserId>().is_ok());
        // A time-ordered ID is not a person ID, and the other way round.
        assert_eq!(
            first.to_string().parse::<ProjectId>(),
            Err(IdError::WrongVersion)
        );
    }

    #[test]
    fn display_and_parse_round_trip() {
        let id = EventId::generate(&mut SequentialIds::default()).unwrap();
        let text = id.to_string();
        assert_eq!(text.len(), 36);
        assert_eq!(text.parse::<EventId>(), Ok(id));
        assert_eq!(EventId::from_bytes(id.to_bytes()), Ok(id));
    }

    #[test]
    fn only_the_plain_form_of_the_right_uuid_version_is_accepted() {
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
        let id = EventId::generate(&mut SequentialIds::default()).unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{id}\""));
        assert_eq!(serde_json::from_str::<EventId>(&json).unwrap(), id);
        assert!(serde_json::from_str::<EventId>("\"nope\"").is_err());
        assert!(serde_json::from_str::<EventId>("7").is_err());
    }

    #[test]
    fn a_generic_entity_id_takes_every_kind_of_id() {
        let person = UserId::generate(&mut SequentialIds::default());
        let entity = EntityId::from(person);
        assert_eq!(entity.to_string(), person.to_string());
        assert_eq!(entity.to_string().parse::<EntityId>(), Ok(entity));
    }

    #[test]
    fn debug_names_the_kind_and_not_the_wrapper() {
        let id = UserId::generate(&mut SequentialIds::default());
        assert_eq!(format!("{id:?}"), format!("UserId({id})"));
    }
}
