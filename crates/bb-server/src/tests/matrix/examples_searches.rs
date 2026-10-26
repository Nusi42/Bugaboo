//! The example requests of the routes of saved searches and of the list of a project (M5 WP-06,
//! SEC-MATRIX).
//!
//! Every route that works on a saved search lets the actor make one first, through the same
//! server: a saved search is private and the actor's own, so anybody who is signed in has one to
//! work on. For somebody who is not signed in, or who comes with an access key, the call is turned
//! away before it looks for the search, so a search nobody made stands in.

use bb_core::id::SavedSearchId;
use bb_testing::Failure;
use jiff::Timestamp;
use serde_json::json;

use super::actors::Project;
use super::examples::Request;
use super::routes::Fixture;
use crate::sign_in_tests::http::Browser;

/// An ID that no saved search has, in the form of one (a time-ordered ID): the route reads the
/// address before it asks who is calling.
fn nobodys_search() -> Result<String, Failure> {
    Ok(SavedSearchId::from_parts(Timestamp::UNIX_EPOCH, [5; 10])?.to_string())
}

const SEARCHES: &str = "/api/v1/saved-searches";

/// A saved search of the actor: its ID.
async fn their_search(fixture: &Fixture<'_>, browser: &mut Browser) -> Result<String, Failure> {
    let made = browser
        .send(
            "POST",
            SEARCHES,
            Some(json!({
                "name": format!("Search {}", fixture.next_number()),
                "text": "status is open",
            })),
        )
        .await?;
    match made.body["id"].as_str() {
        Some(id) => Ok(id.to_owned()),
        None => nobodys_search(),
    }
}

/// The request of a route of saved searches, if the operation is one.
pub(super) async fn search_request(
    fixture: &Fixture<'_>,
    operation: &str,
    browser: &mut Browser,
) -> Result<Option<Request>, Failure> {
    let target = fixture.project(Project::Target).to_string();
    Ok(Some(match operation {
        "listSavedSearches" => ("GET", SEARCHES.into(), None),
        "listShareChoices" => ("GET", format!("{SEARCHES}/share-choices?kind=group"), None),
        "createSavedSearch" => (
            "POST",
            SEARCHES.into(),
            Some(json!({"name": "Open bugs", "text": "status is open"})),
        ),
        "getSavedSearch" => {
            let search = their_search(fixture, browser).await?;
            ("GET", format!("{SEARCHES}/{search}"), None)
        }
        "changeSavedSearch" => {
            let search = their_search(fixture, browser).await?;
            (
                "PATCH",
                format!("{SEARCHES}/{search}"),
                Some(json!({"version": 1, "name": "Renamed"})),
            )
        }
        "deleteSavedSearch" => {
            let search = their_search(fixture, browser).await?;
            ("DELETE", format!("{SEARCHES}/{search}"), None)
        }
        "restoreSavedSearch" => {
            let search = their_search(fixture, browser).await?;
            browser
                .send("DELETE", &format!("{SEARCHES}/{search}"), None)
                .await?;
            ("POST", format!("{SEARCHES}/{search}/restore"), None)
        }
        "copySavedSearch" => {
            let search = their_search(fixture, browser).await?;
            ("POST", format!("{SEARCHES}/{search}/copy"), Some(json!({})))
        }
        "shareSavedSearch" => {
            let search = their_search(fixture, browser).await?;
            (
                "POST",
                format!("{SEARCHES}/{search}/shares"),
                Some(json!({"kind": "project", "id": target})),
            )
        }
        "stopSharingSavedSearch" => {
            let search = their_search(fixture, browser).await?;
            (
                "DELETE",
                format!("{SEARCHES}/{search}/shares/project/{target}"),
                None,
            )
        }
        "getProjectList" => ("GET", format!("/api/v1/projects/{target}/list"), None),
        "setProjectList" => (
            "PUT",
            format!("/api/v1/projects/{target}/list"),
            Some(json!({"search": null})),
        ),
        _ => return Ok(None),
    }))
}
