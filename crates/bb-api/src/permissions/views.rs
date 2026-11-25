//! What the routes of the board and the calendar need (PLAN-06, PLAN-09, PRJ-06, ADR-0034,
//! amendment of 2026-11-24). Both read through the compiled search, so they need a signed-in person:
//! what they may read is decided in the statement and by the field levels, and a card's move is the
//! change of the issue, judged by its own route. Every read names people and is never stored by a
//! cache.

use super::{Answer, Declared, Route, route_reading};

/// The routes of the board and the calendar, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    // Posted reads: the answer to a POST is never kept anyway.
    route_reading("getBoard", Declared::SignedIn, Answer::Ordinary),
    route_reading("getCalendar", Declared::SignedIn, Answer::Ordinary),
];
