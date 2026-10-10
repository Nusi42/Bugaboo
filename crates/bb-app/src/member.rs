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
