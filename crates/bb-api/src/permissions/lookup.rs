//! Looking rows up in the table of permissions: every row, the row of a route, and the routes that
//! are open without the token of a session.

use std::sync::OnceLock;

use axum::http::Method;

use super::{Declared, ROUTES, Route, saved_searches, search_routes};

/// Every row of the table: the rows above and those of the areas that have a file of their own
/// beside this one.
pub fn rows() -> impl Iterator<Item = &'static Route> {
    ROUTES
        .iter()
        .chain(saved_searches::ROUTES)
        .chain(search_routes::ROUTES)
}

/// The row of the route with this operation ID, if it is declared.
#[must_use]
pub fn declared(operation_id: &str) -> Option<&'static Route> {
    rows().find(|row| row.id == operation_id)
}

/// Whether a request with this method to this address is open without the token of a session: the
/// routes that are `public` and change something. People who sign in or sign up have no session
/// yet; they are protected by the origin check and carry no secret a forged request could use
/// (SEC-12).
#[must_use]
pub fn csrf_exempt(method: &Method, path: &str) -> bool {
    static OPEN: OnceLock<Vec<(Method, &'static str)>> = OnceLock::new();
    OPEN.get_or_init(|| {
        crate::openapi::operations()
            .iter()
            .filter(|operation| operation.needs == Declared::Public && operation.changes())
            .map(|operation| (operation.method.clone(), operation.path.as_str()))
            .collect()
    })
    .iter()
    .any(|(open_method, open_path)| open_method == method && *open_path == path)
}
