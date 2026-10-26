//! What the routes of the search need (QRY-04, QRY-05, SEC-17).
//!
//! A search shows the issues of every project the person sees. What they may read is decided in
//! the statement itself (the sight of ADR-0004) and in the fields they may search by, so a route
//! needs a signed-in person and nothing more.

use super::{Answer, Declared, ROUTES as TABLE, Route, route};

/// The rows of the search routes, next to [`super::ROUTES`].
pub(super) const ROUTES: &[Route] = &[
    route("searchIssues", Declared::SignedIn, Answer::NeverStored),
    route(
        "countSearchedIssues",
        Declared::SignedIn,
        Answer::NeverStored,
    ),
    route(
        "groupSearchedIssues",
        Declared::SignedIn,
        Answer::NeverStored,
    ),
    route("listSearchFields", Declared::SignedIn, Answer::NeverStored),
];

/// The row of the route with this operation ID in the table or among the search routes.
pub(super) fn find(operation_id: &str) -> Option<&'static Route> {
    TABLE
        .iter()
        .chain(ROUTES)
        .find(|row| row.id == operation_id)
}
