//! What the routes of the timeline need (PLAN-01, PLAN-08, PLAN-13, PLAN-14, ADR-0030 and its
//! amendment M8). A page of the timeline reads through the compiled search, so it needs a signed-in
//! person: what they may read is decided in the statement and by the field levels. The preview of
//! *Move it after* and the time factor are read on an issue the person sees. Accepting a warning
//! and setting the time factor are changes of the issue: the person on the bar may make them on
//! their issue whatever their role says, which the use case judges beside *Change issues* and the
//! field level. Every read names people or their work times and is never stored by a cache.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route, route_reading};

/// The routes of the timeline, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    // Posted reads: the answer to a POST is never kept anyway.
    route_reading("getTimeline", Declared::SignedIn, Answer::Ordinary),
    route_reading(
        "previewMoveAfter",
        Declared::Permission(Permission::SeeIssues),
        Answer::Ordinary,
    ),
    route(
        "acceptWarning",
        Declared::Permission(Permission::ChangeIssues),
        Answer::Ordinary,
    ),
    route(
        "getTimeFactor",
        Declared::Permission(Permission::SeeIssues),
        Answer::NeverStored,
    ),
    route(
        "setTimeFactor",
        Declared::Permission(Permission::ChangeIssues),
        Answer::Ordinary,
    ),
];
