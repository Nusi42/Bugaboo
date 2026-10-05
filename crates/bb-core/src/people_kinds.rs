//! The kinds of people information (ADR-0022, ADR-0025 decision 11): the places where a person
//! can be shown. *Who sees people* is set per role and per kind, and every person in an answer
//! is shown as its kind says.
//!
//! The list is declared here once; the policy engine, the API description and the screens read it.

use crate::message::{self, MessageKey};

words! {
    /// One place where a person can be shown.
    pub enum PeopleKind {
        /// *Assigned to*.
        AssignedTo => "assigned-to",
        /// *Reported by*.
        ReportedBy => "reported-by",
        /// *Last changed by*.
        LastChangedBy => "last-changed-by",
        /// The authors of comments and history entries.
        Authors => "authors",
        /// "… is editing".
        Editing => "editing",
        /// The followers of an issue or a document.
        Followers => "followers",
        /// The people on time entries.
        TimeEntries => "time-entries",
        /// People that are mentioned in a text.
        Mentions => "mentions",
        /// Pictures of people. A picture is shown only where the kind of the place also gives
        /// *Names*.
        Pictures => "pictures",
        /// Member lists of a project (M2-Q3).
        ProjectMembers => "project-members",
    }
}

impl PeopleKind {
    /// The message that names the kind (`people-kind.<key>`).
    #[must_use]
    pub const fn name(self) -> MessageKey {
        match self {
            Self::AssignedTo => message::PEOPLE_KIND_ASSIGNED_TO,
            Self::ReportedBy => message::PEOPLE_KIND_REPORTED_BY,
            Self::LastChangedBy => message::PEOPLE_KIND_LAST_CHANGED_BY,
            Self::Authors => message::PEOPLE_KIND_AUTHORS,
            Self::Editing => message::PEOPLE_KIND_EDITING,
            Self::Followers => message::PEOPLE_KIND_FOLLOWERS,
            Self::TimeEntries => message::PEOPLE_KIND_TIME_ENTRIES,
            Self::Mentions => message::PEOPLE_KIND_MENTIONS,
            Self::Pictures => message::PEOPLE_KIND_PICTURES,
            Self::ProjectMembers => message::PEOPLE_KIND_PROJECT_MEMBERS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_its_own_name_in_the_catalog() {
        let mut seen = std::collections::HashSet::new();
        for kind in PeopleKind::ALL {
            let key = kind.name();
            assert_eq!(key.as_str(), format!("people-kind.{}", kind.code()));
            assert!(seen.insert(key));
            assert!(!message::Message::from(key).render().is_empty());
        }
        assert_eq!(PeopleKind::ALL.len(), 10);
    }
}
