//! What the routes of AI need (AI-01, AI-02, AI-03, PRIV-08): the marks *Never sent to AI* (read by
//! everybody who is signed in, set with *Change organization settings* for a built-in field and
//! *Change project settings* for a project), the organization's settings,
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
    route("listNeverSentToAi", Declared::SignedIn, Answer::Ordinary),
    route(
        "setFieldNeverSentToAi",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Ordinary,
    ),
    route(
        "setProjectNeverSentToAi",
        Declared::Permission(Permission::ChangeProjectSettings),
        Answer::Ordinary,
    ),
];
