//! The names of the members of a record in its events, defined once: the use cases that write the
//! events spell them with these, and the conflict answer (`editing/conflict.rs`) lists the ones it
//! may show with the same constants. The words are what the events already store, so no stored
//! event is renamed.

/// The members of a project.
pub(crate) mod project {
    pub(crate) const NAME: &str = "name";
    pub(crate) const SHORT_NAME: &str = "short_name";
}

/// The members of a group.
pub(crate) mod group {
    pub(crate) const NAME: &str = "name";
}

/// The members of a role.
pub(crate) mod role {
    pub(crate) const NAME: &str = "name";
    pub(crate) const PERMISSIONS: &str = "permissions";
    pub(crate) const TWO_STEP: &str = "two_step";
}

/// The members of the sign-up policy (`bb_core::settings::SIGN_UP_POLICY`).
pub(crate) mod sign_up_policy {
    pub(crate) const ENABLED: &str = "auth_sign_up_enabled";
    pub(crate) const DOMAINS: &str = "auth_sign_up_domains";
}

/// The members of a layout.
pub(crate) mod layout {
    pub(crate) const KIND: &str = "kind";
    pub(crate) const PROJECT: &str = "project";
    pub(crate) const ISSUE_TYPE: &str = "issue_type";
    pub(crate) const SECTIONS: &str = "sections";
}

/// The members of a kind of relation.
pub(crate) mod relation_kind {
    pub(crate) const FORWARD: &str = "forward";
    pub(crate) const BACKWARD: &str = "backward";
    pub(crate) const RETIRED: &str = "retired";
    /// The list of the kinds the first start made.
    pub(crate) const STARTING: &str = "relation-kinds";
}

/// The members of a relation between two issues. History reads the two issues from `FROM` and
/// `TO`.
pub(crate) mod relation {
    pub(crate) const KIND: &str = "kind";
    pub(crate) const FROM: &str = "from";
    pub(crate) const TO: &str = "to";
}

/// The members of a comment. History reads the issue of a new comment from `ISSUE`.
pub(crate) mod comment {
    pub(crate) const ISSUE: &str = "issue";
    pub(crate) const TEXT: &str = "text";
    pub(crate) const DELETED_AT: &str = crate::issues::member::DELETED_AT;
}

/// The member of the event of someone following an issue or leaving it.
pub(crate) mod follower {
    pub(crate) const PERSON: &str = "follower";
}

/// The member of the event of a person reading their *Mentions*.
pub(crate) mod mentions {
    pub(crate) const READ: &str = "mentions-read";
}

/// The members of a tag.
pub(crate) mod tag {
    pub(crate) const NAME: &str = "name";
    pub(crate) const COLOR: &str = "color";
    pub(crate) const PROJECTS: &str = "projects";
    /// The tag a tag was combined into, in the event of the tag that went away.
    pub(crate) const COMBINED_INTO: &str = "combined-into";
}

/// The members of a label.
pub(crate) mod label {
    pub(crate) const NAME: &str = "name";
    pub(crate) const COLOR: &str = "color";
    pub(crate) const GROUP: &str = "group";
    pub(crate) const RETIRED: &str = "retired";
    /// The label a label was combined into, in the event of the label that went away.
    pub(crate) const COMBINED_INTO: &str = "combined-into";
}

/// The members of a group of labels.
pub(crate) mod label_group {
    pub(crate) const NAME: &str = "name";
    pub(crate) const PICK_ONE: &str = "pick-one";
    pub(crate) const RETIRED: &str = "retired";
}
