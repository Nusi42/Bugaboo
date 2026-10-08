//! The permission catalogue (ADR-0025 decision 1, architecture §7): the one list of what a role
//! can allow.
//!
//! Every permission has a key (`see-projects`, also the value of `x-permission` in the API
//! description), a topic, a plain sentence as a message, where it counts and a kind. The API
//! description, the role page, SEC-MATRIX and the docs are made from this list. A later
//! milestone adds its permissions here with their sentences, kinds and starting values in the
//! same change. Enforced by: ARCH-PERMS (every route declares one of these keys or `public`,
//! `signed-in`, `own-account`; every permission is used and has its message).

use bb_core::{words, words_in_api};

pub use crate::parts::{Counts, Kind, Topic};

/// The permissions an access key never keeps, whatever its choice, even *Everything I can do*
/// and single permissions (owner decisions of 2026-10-06): a key cannot invite people, change who
/// has which role or change the settings of the organization (which could switch off the two-step
/// requirement, open signing up or change how long sessions last). These stay in the app, where
/// the person is signed in. Defined once, here
/// beside the kinds a key's choice keeps; narrowing, the key form and the key's own checks all
/// read this list.
pub const NOT_FOR_KEYS: &[Permission] = &[
    Permission::ManageInvitations,
    Permission::ManagePeople,
    Permission::ManageGroups,
    Permission::ManageRoles,
    Permission::DecideRoles,
    Permission::DecideWhatRolesSee,
    Permission::ChangeOrganizationSettings,
];

/// Declares the permissions: one line per permission, with the key, topic, where it counts and
/// kind. The sentence on the role page is the message `permission.<key>`. Everything else is made
/// from this list.
macro_rules! permissions {
    ($($(#[$doc:meta])* $variant:ident => ($key:literal, $topic:ident, $counts:ident, $kind:ident)),+ $(,)?) => {
        words! {
            /// One thing a role can allow. The key is as stored and as written in `x-permission`.
            pub enum Permission {
                $($(#[$doc])* $variant => $key),+
            }
            messages sentence = "permission.";
        }

        impl Permission {
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

            /// Whether an access key may hold the permission at all ([`NOT_FOR_KEYS`] says which it
            /// may not).
            #[must_use]
            pub fn keys_may_hold(self) -> bool {
                !NOT_FOR_KEYS.contains(&self)
            }
        }
    };
}

permissions! {
    /// See the project and what is in it.
    SeeProjects => ("see-projects", Projects, Project, See),
    /// Make projects at the top level.
    MakeProjects => ("make-projects", Projects, Organization, Change),
    /// Make sub-projects inside the project.
    MakeSubProjects => ("make-sub-projects", Projects, Project, Change),
    /// Move sub-projects into or out of the project.
    MoveProjects => ("move-projects", Projects, Project, Change),
    /// Change the project's name, short name and settings.
    ChangeProjectSettings => ("change-project-settings", Projects, Project, Manage),
    /// Decide who has which role, including *No access*.
    DecideRoles => ("decide-roles", People, Project, Manage),
    /// Decide which fields and which people information each role sees.
    DecideWhatRolesSee => ("decide-what-roles-see", People, Project, Manage),
    /// Check what other people can do, and why.
    CheckAccess => ("check-access", People, Project, See),
    /// See everybody's e-mail address and correct their names.
    ManagePeople => ("manage-people", People, Organization, Manage),
    /// Invite people and take invitations back.
    ManageInvitations => ("manage-invitations", People, Organization, Manage),
    /// Make, rename and delete groups and change who is in them.
    ManageGroups => ("manage-groups", People, Organization, Manage),
    /// Make, change and delete roles.
    ManageRoles => ("manage-roles", People, Organization, Manage),
    /// Change organization settings.
    ChangeOrganizationSettings => ("change-organization-settings", Settings, Organization, Manage),
    /// See how the server is doing.
    SeeStatus => ("see-status", Settings, Organization, See),
}

words_in_api!(Permission, "One permission of the catalogue.");

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use bb_core::UnknownWord;
    use bb_core::message::Message;
    use bb_core::technical::TechnicalName;

    use super::*;

    /// The word with a capital, as the table writes the kind.
    fn capital(word: &str) -> String {
        let mut letters = word.chars();
        letters
            .next()
            .map(|first| first.to_uppercase().chain(letters).collect())
            .unwrap_or_default()
    }

    /// The table "The permission catalogue" of architecture §7 is the only other place that
    /// holds the topic, the place and the kind of each permission: the test reads it from the
    /// page, in the order of the catalogue, so there is no third copy.
    #[test]
    fn the_catalogue_is_the_table_of_architecture_7() {
        let architecture = include_str!("../../../docs/architecture/07-cross-cutting.md");
        let rows: Vec<Vec<&str>> = architecture
            .split("### The permission catalogue")
            .nth(1)
            .unwrap()
            .lines()
            .skip_while(|line| !line.starts_with('|'))
            .take_while(|line| line.starts_with('|'))
            .skip(2)
            .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
            .collect();
        assert_eq!(rows.len(), Permission::ALL.len(), "a row per permission");
        for (permission, row) in Permission::ALL.iter().zip(&rows) {
            let sentence = Message::from(permission.sentence()).render();
            assert_eq!(
                row.iter()
                    .map(|cell| cell.replace(['`', '*'], ""))
                    .collect::<Vec<_>>(),
                [
                    permission.code().to_owned(),
                    Message::from(permission.topic().name()).render(),
                    permission.counts().code().to_owned(),
                    capital(permission.kind().code()),
                    sentence.replace('*', "")
                ],
                "{permission}"
            );
        }
    }

    #[test]
    fn the_keys_are_technical_names_and_unique() {
        let mut seen = HashSet::new();
        for permission in Permission::ALL {
            assert!(TechnicalName::new(permission.code()).is_ok());
            assert!(seen.insert(permission.code()));
            assert_eq!(permission.code().parse(), Ok(*permission));
        }
        assert_eq!("see-everything".parse::<Permission>(), Err(UnknownWord));
    }

    #[test]
    fn every_permission_and_topic_has_its_own_sentence_in_the_catalog() {
        let mut seen = HashSet::new();
        for permission in Permission::ALL {
            let key = permission.sentence();
            assert_eq!(key.as_str(), format!("permission.{}", permission.code()));
            assert!(seen.insert(key));
            assert!(!Message::from(key).render().is_empty());
        }
        for topic in Topic::ALL {
            assert!(!Message::from(topic.name()).render().is_empty());
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
