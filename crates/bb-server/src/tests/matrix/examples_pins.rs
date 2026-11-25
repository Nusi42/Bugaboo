//! The example requests of the routes of pins (CFG-05, SEC-MATRIX).
//!
//! A pin is the person's own, so every route works on a saved search the actor made first, as the
//! routes of saved searches do; for somebody who is not signed in, or who comes with an access key,
//! the call is turned away before it looks at the pin.

use bb_testing::Failure;
use serde_json::json;

use super::examples::Request;
use super::examples_dashboards::theirs;
use super::examples_searches::their_search;
use super::routes::Fixture;
use crate::sign_in_tests::http::Browser;

const PINS: &str = "/api/v1/me/pins";

/// The request of a route of pins, if the operation is one.
pub(super) async fn pin_request(
    fixture: &Fixture<'_>,
    operation: &str,
    browser: &mut Browser,
) -> Result<Option<Request>, Failure> {
    let pinned = |kind: &str, target: &str| json!({"kind": kind, "target": target});
    Ok(Some(match operation {
        "listPins" => ("GET", PINS.into(), None),
        "createPin" => {
            // A dashboard of theirs, which the pins of a person may point to like a saved search.
            let (dashboard, _) = theirs(fixture, browser).await?;
            ("POST", PINS.into(), Some(pinned("dashboard", &dashboard)))
        }
        "deletePin" => {
            let search = their_search(fixture, browser).await?;
            (
                "DELETE",
                format!("{PINS}?kind=saved-search&target={search}"),
                None,
            )
        }
        "orderPins" => {
            let search = their_search(fixture, browser).await?;
            (
                "PUT",
                format!("{PINS}/order"),
                Some(json!({"pins": [pinned("saved-search", &search)]})),
            )
        }
        _ => return Ok(None),
    }))
}
