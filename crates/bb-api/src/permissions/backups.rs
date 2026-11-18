//! What the routes of backups need (OPS-07, ADR-0046 decision 3): *Change organization settings*,
//! from a session only (no access key, no agent). There is no route that restores. Kept beside the
//! table so it does not grow.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of backups, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    // The records name the person who created each backup.
    route(
        "listBackups",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::NeverStored,
    ),
    route(
        "createBackup",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::Ordinary,
    ),
];
