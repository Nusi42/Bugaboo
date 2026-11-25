//! What the routes of dashboards need (CFG-03, ADR-0050): the rows of the table of permissions for
//! this area, kept beside it so the table does not grow. Every route takes a session only; the use
//! cases turn an access key away (ADR-0050, decision 10).

use bb_policy::Permission;

use super::{Answer, Declared, Route, route, route_reading};

/// The routes of dashboards, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("listDashboards", Declared::SignedIn, Answer::NeverStored),
    route("createDashboard", Declared::SignedIn, Answer::Ordinary),
    route("getHome", Declared::SignedIn, Answer::NeverStored),
    route("getDashboard", Declared::SignedIn, Answer::NeverStored),
    route("changeDashboard", Declared::SignedIn, Answer::Ordinary),
    route("deleteDashboard", Declared::SignedIn, Answer::Ordinary),
    route("restoreDashboard", Declared::SignedIn, Answer::Ordinary),
    route("copyDashboard", Declared::SignedIn, Answer::Ordinary),
    route(
        "shareDashboard",
        Declared::Permission(Permission::ShareDashboards),
        Answer::Ordinary,
    ),
    route("stopSharingDashboard", Declared::SignedIn, Answer::Ordinary),
    route(
        "listDashboardShareChoices",
        Declared::SignedIn,
        Answer::NeverStored,
    ),
    route("getDashboardBox", Declared::SignedIn, Answer::NeverStored),
    route_reading("previewDashboardBox", Declared::SignedIn, Answer::Ordinary),
    route(
        "listReadyMadeBoxes",
        Declared::SignedIn,
        Answer::NeverStored,
    ),
];
