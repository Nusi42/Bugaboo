//! What the routes of *Switch off* and *Remove a person* need (PRIV-04, ADR-0044, decisions 5 and
//! 6): the rows of the table of permissions for this area, kept beside it so the table does not
//! grow.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of the standing of a person, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route(
        "switchPersonOff",
        Declared::Permission(Permission::ManagePeople),
        Answer::Ordinary,
    ),
    route(
        "switchPersonOn",
        Declared::Permission(Permission::ManagePeople),
        Answer::Ordinary,
    ),
];
