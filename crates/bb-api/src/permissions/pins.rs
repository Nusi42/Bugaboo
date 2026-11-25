//! What the routes of pins need (CFG-05): a signed-in person, for their own pins (the rows of the
//! table of permissions for this area, kept beside it so the table does not grow).

use super::{Answer, Declared, Route, route};

/// The routes of pins, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("listPins", Declared::SignedIn, Answer::NeverStored),
    route("createPin", Declared::SignedIn, Answer::Ordinary),
    route("deletePin", Declared::SignedIn, Answer::Ordinary),
    route("orderPins", Declared::SignedIn, Answer::Ordinary),
];
