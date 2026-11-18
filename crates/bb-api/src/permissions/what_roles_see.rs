//! What the routes of *What roles see* and *Check access* need: deciding what roles see of fields
//! and people, and checking what a person may do. An answer that names people and their rights is
//! never stored by a cache.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of what roles see and of checking access, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route(
        "getFieldPermissions",
        Declared::Permission(Permission::DecideWhatRolesSee),
        Answer::NeverStored,
    ),
    route(
        "changeFieldPermission",
        Declared::Permission(Permission::DecideWhatRolesSee),
        Answer::Ordinary,
    ),
    route(
        "getPeopleVisibility",
        Declared::Permission(Permission::DecideWhatRolesSee),
        Answer::NeverStored,
    ),
    route(
        "changePeopleVisibility",
        Declared::Permission(Permission::DecideWhatRolesSee),
        Answer::Ordinary,
    ),
    route(
        "checkAccess",
        Declared::Permission(Permission::CheckAccess),
        Answer::NeverStored,
    ),
    route(
        "listPeopleToCheck",
        Declared::Permission(Permission::CheckAccess),
        Answer::NeverStored,
    ),
];
