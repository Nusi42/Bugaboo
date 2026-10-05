//! The permission catalogue (ADR-0025 decision 1, architecture §7): the one list of what a role
//! can allow.
//!
//! Every permission has a key (`see-projects`, also the value of `x-permission` in the API
//! description), a topic, a plain sentence as a message, where it counts and a kind. The API
//! description, the role page, SEC-MATRIX and the docs are made from this list. A later
//! milestone adds its permissions here with their sentences, kinds and starting values in the
//! same change. Enforced by: ARCH-PERMS (every route declares one of these keys or `public`,
//! `signed-in`, `own-account`; every permission is used and has its message).

use std::fmt;
use std::str::FromStr;

use bb_core::message::{self, MessageKey};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// What a group of permissions is about, for the role page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Topic {
    /// Projects and sub-projects.
    Projects,
    /// People, groups, roles and who has which role.
    People,
    /// Organization settings and the state of the server.
    Settings,
}

impl Topic {
    /// Every topic, in the order of the role page.
    pub const ALL: &'static [Self] = &[Self::Projects, Self::People, Self::Settings];

    /// The word of the topic in the API (`projects`).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Projects => "projects",
            Self::People => "people",
            Self::Settings => "settings",
        }
    }

    /// The message that names the topic (`permission-topic.<topic>`).
    #[must_use]
    pub const fn name(self) -> MessageKey {
        match self {
            Self::Projects => message::PERMISSION_TOPIC_PROJECTS,
            Self::People => message::PERMISSION_TOPIC_PEOPLE,
            Self::Settings => message::PERMISSION_TOPIC_SETTINGS,
        }
    }
}

impl Serialize for Topic {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code())
    }
}

/// In the API description a topic is one of its words, so clients get a union type.
#[cfg(feature = "openapi")]
impl utoipa::PartialSchema for Topic {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{ObjectBuilder, Type};
        ObjectBuilder::new()
            .schema_type(Type::String)
            .description(Some("What a group of permissions is about."))
            .enum_values(Some(Self::ALL.iter().map(|topic| topic.code())))
            .into()
    }
}

#[cfg(feature = "openapi")]
impl utoipa::ToSchema for Topic {}

/// Where a permission counts (ADR-0025, decision 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Counts {
    /// Only from a role given for the whole organization. A role given on a project that holds
    /// such a permission gives nothing for it, and the role page says so.
    Organization,
    /// In projects: from a role given for the whole organization (every project) or on a
    /// project.
    Project,
}

/// What an access key's choice keeps (ADR-0025, decision 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// Changes nothing.
    See,
    /// Work in projects.
    Change,
    /// People, roles and settings.
    Manage,
}

impl Kind {
    /// Every kind, from the least to the most.
    pub const ALL: &'static [Self] = &[Self::See, Self::Change, Self::Manage];
}

/// Declares the permissions: one line per permission, with the key, topic, where it counts, kind
/// and the message of its sentence. Everything else is made from this list.
macro_rules! permissions {
    ($($(#[$doc:meta])* $variant:ident => ($key:literal, $topic:ident, $counts:ident, $kind:ident, $sentence:path)),+ $(,)?) => {
        /// One thing a role can allow.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Permission {
            $($(#[$doc])* $variant),+
        }

        impl Permission {
            /// Every permission, in the order of the catalogue.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The key of the permission, as stored and as written in `x-permission`.
            #[must_use]
            pub const fn code(self) -> &'static str {
                match self { $(Self::$variant => $key),+ }
            }

            /// What the permission is about.
            #[must_use]
            pub const fn topic(self) -> Topic {
                match self { $(Self::$variant => Topic::$topic),+ }
            }

            /// Where the permission counts.
            #[must_use]
            pub const fn counts(self) -> Counts {
                match self { $(Self::$variant => Counts::$counts),+ }
            }

            /// What an access key's choice keeps.
            #[must_use]
            pub const fn kind(self) -> Kind {
                match self { $(Self::$variant => Kind::$kind),+ }
            }

            /// The sentence on the role page (`permission.<key>`).
            #[must_use]
            pub const fn sentence(self) -> MessageKey {
                match self { $(Self::$variant => $sentence),+ }
            }

            /// The permission with this key, if there is one.
            #[must_use]
            pub fn from_code(code: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|permission| permission.code() == code)
            }
        }
    };
}

permissions! {
    /// See the project and what is in it.
    SeeProjects => ("see-projects", Projects, Project, See, message::PERMISSION_SEE_PROJECTS),
    /// Make projects at the top level.
    MakeProjects => ("make-projects", Projects, Organization, Change, message::PERMISSION_MAKE_PROJECTS),
    /// Make sub-projects inside the project.
    MakeSubProjects => ("make-sub-projects", Projects, Project, Change, message::PERMISSION_MAKE_SUB_PROJECTS),
    /// Move sub-projects into or out of the project.
    MoveProjects => ("move-projects", Projects, Project, Change, message::PERMISSION_MOVE_PROJECTS),
    /// Change the project's name, short name and settings.
    ChangeProjectSettings => ("change-project-settings", Projects, Project, Manage, message::PERMISSION_CHANGE_PROJECT_SETTINGS),
    /// Decide who has which role, including *No access*.
    DecideRoles => ("decide-roles", People, Project, Manage, message::PERMISSION_DECIDE_ROLES),
    /// Decide which fields and which people information each role sees.
    DecideWhatRolesSee => ("decide-what-roles-see", People, Project, Manage, message::PERMISSION_DECIDE_WHAT_ROLES_SEE),
    /// Check what other people can do, and why.
    CheckAccess => ("check-access", People, Project, See, message::PERMISSION_CHECK_ACCESS),
    /// See everybody's e-mail address and correct their names.
    ManagePeople => ("manage-people", People, Organization, Manage, message::PERMISSION_MANAGE_PEOPLE),
    /// Invite people and take invitations back.
    ManageInvitations => ("manage-invitations", People, Organization, Manage, message::PERMISSION_MANAGE_INVITATIONS),
    /// Make, rename and delete groups and change who is in them.
    ManageGroups => ("manage-groups", People, Organization, Manage, message::PERMISSION_MANAGE_GROUPS),
    /// Make, change and delete roles.
    ManageRoles => ("manage-roles", People, Organization, Manage, message::PERMISSION_MANAGE_ROLES),
    /// Change organization settings.
    ChangeOrganizationSettings => ("change-organization-settings", Settings, Organization, Manage, message::PERMISSION_CHANGE_ORGANIZATION_SETTINGS),
    /// See how the server is doing.
    SeeStatus => ("see-status", Settings, Organization, See, message::PERMISSION_SEE_STATUS),
}

impl fmt::Display for Permission {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

/// The text is not the key of a permission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownPermission;

impl fmt::Display for UnknownPermission {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("not the key of a permission")
    }
}

impl std::error::Error for UnknownPermission {}

impl FromStr for Permission {
    type Err = UnknownPermission;

    fn from_str(text: &str) -> Result<Self, UnknownPermission> {
        Self::from_code(text).ok_or(UnknownPermission)
    }
}

impl Serialize for Permission {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for Permission {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// In the API description a permission is one of the keys of the catalogue, so clients get a
/// union type and the description is made from the same list.
#[cfg(feature = "openapi")]
impl utoipa::PartialSchema for Permission {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{ObjectBuilder, Type};
        ObjectBuilder::new()
            .schema_type(Type::String)
            .description(Some("One permission of the catalogue."))
            .enum_values(Some(Self::ALL.iter().map(|permission| permission.code())))
            .into()
    }
}

#[cfg(feature = "openapi")]
impl utoipa::ToSchema for Permission {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use Counts::{Organization as Org, Project as Proj};
    use Kind::{Change, Manage, See};
    use Topic::{People, Projects, Settings};
    use bb_core::technical::TechnicalName;

    use super::*;

    #[test]
    fn the_catalogue_has_the_fourteen_permissions_of_architecture_7() {
        let rows: Vec<(&str, Topic, Counts, Kind)> = Permission::ALL
            .iter()
            .map(|permission| {
                (
                    permission.code(),
                    permission.topic(),
                    permission.counts(),
                    permission.kind(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                ("see-projects", Projects, Proj, See),
                ("make-projects", Projects, Org, Change),
                ("make-sub-projects", Projects, Proj, Change),
                ("move-projects", Projects, Proj, Change),
                ("change-project-settings", Projects, Proj, Manage),
                ("decide-roles", People, Proj, Manage),
                ("decide-what-roles-see", People, Proj, Manage),
                ("check-access", People, Proj, See),
                ("manage-people", People, Org, Manage),
                ("manage-invitations", People, Org, Manage),
                ("manage-groups", People, Org, Manage),
                ("manage-roles", People, Org, Manage),
                ("change-organization-settings", Settings, Org, Manage),
                ("see-status", Settings, Org, See),
            ]
        );
    }

    #[test]
    fn the_keys_are_technical_names_and_unique() {
        let mut seen = HashSet::new();
        for permission in Permission::ALL {
            assert!(TechnicalName::new(permission.code()).is_ok());
            assert!(seen.insert(permission.code()));
            assert_eq!(permission.code().parse(), Ok(*permission));
        }
        assert_eq!(
            "see-everything".parse::<Permission>(),
            Err(UnknownPermission)
        );
    }

    #[test]
    fn every_permission_and_topic_has_its_own_sentence_in_the_catalog() {
        let mut seen = HashSet::new();
        for permission in Permission::ALL {
            let key = permission.sentence();
            assert_eq!(key.as_str(), format!("permission.{}", permission.code()));
            assert!(seen.insert(key));
            assert!(!message::Message::from(key).render().is_empty());
        }
        for topic in Topic::ALL {
            assert!(!message::Message::from(topic.name()).render().is_empty());
        }
    }

    #[test]
    fn a_permission_is_written_as_its_key_in_json() {
        let json = serde_json::to_string(&Permission::SeeProjects).unwrap();
        assert_eq!(json, "\"see-projects\"");
        assert_eq!(
            serde_json::from_str::<Permission>(&json).unwrap(),
            Permission::SeeProjects
        );
        assert!(serde_json::from_str::<Permission>("\"nope\"").is_err());
    }
}
