//! What a permission is made of besides its key: the topic it is listed under, where it counts and
//! what an access key's choice keeps of it (ADR-0025, decisions 1 and 8). The catalogue
//! (`catalogue.rs`) gives each permission its values.

use bb_core::message::{self, MessageKey};
use bb_core::{words, words_in_api};

words! {
    /// What a group of permissions is about, for the role page.
    pub enum Topic {
        /// Projects and sub-projects.
        Projects => "projects",
        /// People, groups, roles and who has which role.
        People => "people",
        /// Organization settings and the state of the server.
        Settings => "settings",
    }
}

words_in_api!(Topic, "What a group of permissions is about.");

impl Topic {
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

words! {
    /// Where a permission counts (ADR-0025, decision 1).
    pub enum Counts {
        /// Only from a role given for the whole organization. A role given on a project that holds
        /// such a permission gives nothing for it, and the role page says so.
        Organization => "organization",
        /// In projects: from a role given for the whole organization (every project) or on a
        /// project.
        Project => "project",
    }
}

words! {
    /// What an access key's choice keeps (ADR-0025, decision 8).
    pub enum Kind {
        /// Changes nothing.
        See => "see",
        /// Work in projects.
        Change => "change",
        /// People, roles and settings.
        Manage => "manage",
    }
}
