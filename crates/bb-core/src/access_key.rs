//! What access keys store (AUTH-07, ADR-0025 decision 8): the key of a person for scripts and
//! apps, what it may do and where, and when it runs out. Every field carries its personal-data
//! mark (ADR-0015). The secret itself is never stored, only its hash, and it never goes into an
//! event or the security log.

use jiff::Timestamp;

use crate::id::{AccessKeyId, ProjectId, UserId};
use crate::personal::Entity;
use crate::security_log::Digest;
use crate::technical::TechnicalName;

/// The most projects one key can be limited to. A list this long is a sign that the key should
/// count in a parent project instead, which covers its sub-projects.
pub const MOST_PROJECTS: usize = 50;

/// The most keys one person can have at the same time (also the most the list of keys shows).
pub const MOST_KEYS: usize = 100;

// The list of keys is read in one page (the browser asks for the largest page and shows all of it).
const _: () = assert!(MOST_KEYS <= crate::paging::PAGE_SIZE_MAX as usize);

words! {
    /// *What it may do* (M2-Q6): the three plain choices, or single permissions under *More
    /// options*.
    pub enum KeyChoice {
        /// *See only*: the permissions of the kind *See*, every field at most *Can see*.
        SeeOnly => "see-only",
        /// *See and change*: the permissions of the kinds *See* and *Change*.
        SeeAndChange => "see-and-change",
        /// *Everything I can do*: every kind.
        Everything => "everything",
        /// Single permissions, picked under *More options*.
        Permissions => "permissions",
    }
}

/// In the API description *What it may do* is one of the known words.
#[cfg(feature = "openapi")]
impl utoipa::PartialSchema for KeyChoice {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{ObjectBuilder, Type};
        ObjectBuilder::new()
            .schema_type(Type::String)
            .description(Some(
                "What the key may do: see only, see and change, everything its owner can do, or single permissions.",
            ))
            .enum_values(Some(Self::ALL.iter().map(|choice| choice.code())))
            .into()
    }
}

#[cfg(feature = "openapi")]
impl utoipa::ToSchema for KeyChoice {}

/// An access key. The person who owns it uses it as themselves, with at most what they may do
/// at that moment.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct AccessKey {
    /// The ID of the key: what the list shows and what deleting names. It opens nothing.
    #[not_personal]
    pub id: AccessKeyId,
    /// Whose key it is.
    #[personal(category = "identity", basis = "contract")]
    pub owner: UserId,
    /// What the person called it, like "Nightly report".
    #[personal(category = "content", basis = "contract")]
    pub name: String,
    /// The public part after `bb_`: it tells the keys of a person apart in the list. It is not a
    /// secret, and the key does not work without the rest.
    #[personal(category = "credentials", basis = "contract")]
    pub prefix: String,
    /// The hash of the 256-bit secret. The secret itself is shown once and never stored.
    #[personal(category = "credentials", basis = "contract")]
    pub secret_hash: Digest,
    /// *What it may do*.
    #[not_personal]
    pub choice: KeyChoice,
    /// When the key was made.
    #[personal(category = "activity", basis = "contract")]
    pub created_at: Timestamp,
    /// When the key stops working. Every key runs out.
    #[not_personal]
    pub expires_at: Timestamp,
    /// The last time the key was used, to the minute.
    #[personal(category = "activity", basis = "contract")]
    pub last_used_at: Option<Timestamp>,
}

/// One permission of a key whose *What it may do* is single permissions.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct AccessKeyPermission {
    /// The key.
    #[not_personal]
    pub key: AccessKeyId,
    /// The key of the permission (`see-projects`).
    #[not_personal]
    pub permission: TechnicalName,
}

/// One project a key is limited to. The key counts in it and its sub-projects.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct AccessKeyProject {
    /// The key.
    #[not_personal]
    pub key: AccessKeyId,
    /// The project, by ID so moving it changes nothing.
    #[not_personal]
    pub project: ProjectId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_07_the_choices_have_the_words_of_the_api() {
        let words: Vec<&str> = KeyChoice::ALL.iter().map(|choice| choice.code()).collect();
        assert_eq!(
            words,
            ["see-only", "see-and-change", "everything", "permissions"]
        );
    }
}
