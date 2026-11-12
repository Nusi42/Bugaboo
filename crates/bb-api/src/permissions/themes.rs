//! What the routes of own themes need (THM-04, ADR-0026 update M8): the rows of the table of
//! permissions for this area. A theme for everyone is changed with *Change organization settings*
//! and a personal one by its maker, so the routes that change a theme judge the theme they name
//! (or the kind a new one gets); reading one and its style sheet needs a signed-in person who may
//! use it.

use super::{Answer, Declared, Route, route};

/// The routes of own themes, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("listThemes", Declared::SignedIn, Answer::Ordinary),
    route("createTheme", Declared::ByRecord, Answer::Ordinary),
    route("getTheme", Declared::SignedIn, Answer::Ordinary),
    route("saveTheme", Declared::ByRecord, Answer::Ordinary),
    route("deleteTheme", Declared::ByRecord, Answer::Ordinary),
    route("copyTheme", Declared::ByRecord, Answer::Ordinary),
    route("downloadTheme", Declared::SignedIn, Answer::Ordinary),
    route("uploadTheme", Declared::ByRecord, Answer::Ordinary),
    route("getThemeSheet", Declared::SignedIn, Answer::Ordinary),
    route(
        "getThemeDraftSheet",
        Declared::ByRecord,
        Answer::NeverStored,
    ),
];
