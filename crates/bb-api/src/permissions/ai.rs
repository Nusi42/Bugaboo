//! What the routes of setting up AI need (AI-01, AI-02, PRIV-08): the organization's settings,
//! the key, the agreement, *What goes where* and the try of the provider need *Change organization
//! settings*, the switch of a project *Change project settings* there. Kept beside the table so it
//! does not grow.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of the setup of AI, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route(
        "getAiSetup",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::NeverStored,
    ),
    route(
        "changeAiSetup",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Ordinary,
    ),
    route(
        "setAiKey",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Ordinary,
    ),
    route(
        "confirmAiAgreement",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Ordinary,
    ),
    route(
        "getWhatGoesWhere",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::NeverStored,
    ),
    route(
        "tryAiProvider",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Ordinary,
    ),
    route(
        "getProjectAi",
        Declared::Permission(Permission::ChangeProjectSettings),
        Answer::NeverStored,
    ),
    route(
        "changeProjectAi",
        Declared::Permission(Permission::ChangeProjectSettings),
        Answer::Ordinary,
    ),
];
