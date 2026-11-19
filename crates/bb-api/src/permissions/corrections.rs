//! What the routes of corrections need (PRIV-06, ADR-0044, decision 9): the rows of the table of
//! permissions for this area, kept beside it so the table does not grow.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of the profile and of the corrections of an administrator, by operation ID. Only a
/// session reaches them; the link to a new address needs none, its secret proves the address.
pub(super) const ROUTES: &[Route] = &[
    route("changeMyName", Declared::OwnAccount, Answer::Ordinary),
    route("askEmailChange", Declared::OwnAccount, Answer::Ordinary),
    route("confirmEmailChange", Declared::Public, Answer::Ordinary),
    route(
        "correctName",
        Declared::Permission(Permission::ManagePeople),
        Answer::Ordinary,
    ),
    route(
        "correctEmail",
        Declared::Permission(Permission::ManagePeople),
        Answer::Ordinary,
    ),
];
