//! What the routes of the sign-in of AI apps need (AI-22, AI-24, ADR-0042): the consent step and
//! *Connected apps* are a person's own account, so a session opens them and a token of an app or an
//! access key never does; the web addresses an administrator allows need *Change organization
//! settings*. Kept beside the table so it does not grow.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of the sign-in of AI apps, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route(
        "getAuthorization",
        Declared::OwnAccount,
        Answer::NeverStored,
    ),
    // The address in the answer holds the code the app is given.
    route("decideAuthorization", Declared::OwnAccount, Answer::Secret),
    route(
        "listConnectedApps",
        Declared::OwnAccount,
        Answer::NeverStored,
    ),
    route("takeBackApp", Declared::OwnAccount, Answer::Ordinary),
    route(
        "listAppAddresses",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::NeverStored,
    ),
    route(
        "addAppAddress",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Ordinary,
    ),
    route(
        "removeAppAddress",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Ordinary,
    ),
];
