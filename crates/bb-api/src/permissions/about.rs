//! What the routes of *About* need (COMP-01): the version and the parts of the program are for
//! every signed-in person and for nobody else.

use super::{Answer, Declared, Route, route};

/// The routes of *About*, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("getAbout", Declared::SignedIn, Answer::Ordinary),
    route("downloadAboutFile", Declared::SignedIn, Answer::Ordinary),
];
