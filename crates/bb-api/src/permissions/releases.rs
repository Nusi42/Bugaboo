//! What the routes of the releases of a project need (ISS-08, ADR-0049, decision 5): the list
//! needs only seeing the project, every change *Manage releases* there.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of releases, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route(
        "listReleases",
        Declared::Permission(Permission::SeeProjects),
        Answer::Ordinary,
    ),
    route(
        "createRelease",
        Declared::Permission(Permission::ManageReleases),
        Answer::Ordinary,
    ),
    route(
        "changeRelease",
        Declared::Permission(Permission::ManageReleases),
        Answer::Ordinary,
    ),
    route(
        "deleteRelease",
        Declared::Permission(Permission::ManageReleases),
        Answer::Ordinary,
    ),
    route(
        "restoreRelease",
        Declared::Permission(Permission::ManageReleases),
        Answer::Ordinary,
    ),
];
