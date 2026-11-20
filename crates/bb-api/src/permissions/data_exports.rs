//! What the routes of *Download my data* need (PRIV-03, ADR-0044, decisions 2 and 4): the rows of
//! the table of permissions for this area, kept beside it so the table does not grow.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of *Download my data*, by operation ID. Every read names people or holds a reason,
/// so none is stored by a cache.
pub(super) const ROUTES: &[Route] = &[
    route("askForMyData", Declared::OwnAccount, Answer::Ordinary),
    route("getMyDataExport", Declared::OwnAccount, Answer::NeverStored),
    route(
        "askForSomeonesData",
        Declared::Permission(Permission::DownloadSomeonesData),
        Answer::Ordinary,
    ),
    route(
        "listDataExports",
        Declared::Permission(Permission::DownloadSomeonesData),
        Answer::NeverStored,
    ),
    route(
        "listPeopleOfDataExports",
        Declared::Permission(Permission::DownloadSomeonesData),
        Answer::NeverStored,
    ),
    route(
        "getDataExportAddress",
        Declared::SignedIn,
        Answer::NeverStored,
    ),
    route(
        "downloadDataExport",
        Declared::SignedIn,
        Answer::NeverStored,
    ),
];
