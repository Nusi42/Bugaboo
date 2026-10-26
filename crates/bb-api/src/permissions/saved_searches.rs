//! What the routes of saved searches and of the list of a project need (QRY-06, M5 WP-06): the rows
//! of the table of permissions for this area, kept beside it so the table does not grow (the file
//! `permissions.rs` is split by area, one area at a time).

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of saved searches, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("listSavedSearches", Declared::SignedIn, Answer::NeverStored),
    route("createSavedSearch", Declared::SignedIn, Answer::Ordinary),
    route("getSavedSearch", Declared::SignedIn, Answer::NeverStored),
    route("changeSavedSearch", Declared::SignedIn, Answer::Ordinary),
    route("deleteSavedSearch", Declared::SignedIn, Answer::Ordinary),
    route("restoreSavedSearch", Declared::SignedIn, Answer::Ordinary),
    route("copySavedSearch", Declared::SignedIn, Answer::Ordinary),
    route(
        "shareSavedSearch",
        Declared::Permission(Permission::ShareSearches),
        Answer::Ordinary,
    ),
    route(
        "stopSharingSavedSearch",
        Declared::SignedIn,
        Answer::Ordinary,
    ),
    route("listShareChoices", Declared::SignedIn, Answer::NeverStored),
    route("getProjectList", Declared::ByRecord, Answer::NeverStored),
    route(
        "setProjectList",
        Declared::Permission(Permission::ChangeProjectSettings),
        Answer::Ordinary,
    ),
];
