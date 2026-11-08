//! The example requests of the routes of the timer and of time entries (M6, SEC-MATRIX).
//!
//! Starting the timer and booking by hand name a new issue of the target project, reported by the
//! administrator of the matrix. The timer itself is the actor's own: a stop, a switch, a new start
//! time and *Throw away* work on the timer the actor has, and an actor without one is told so
//! (`timer.not-running`), which the matrix counts as let in. Changing or deleting an entry names
//! the actor's own entry, booked through the API first where they may, and one nobody made where
//! they may not. The time of an issue names a new issue of the target project, and the totals of a
//! project the target project.

use bb_testing::Failure;
use serde_json::json;

use super::actors::Project;
use super::examples::Request;
use super::examples_issues::an_issue;
use super::routes::Fixture;
use crate::sign_in_tests::http::Browser;

/// An entry ID that nobody made, for the actors who may not book an entry of their own.
const NOBODYS_ENTRY: &str = "018f0e6a-5c2b-7d3e-9a1f-2b3c4d5e6f70";

/// The version of the actor's timer, or 0 when they never started one.
pub(super) async fn timer_version(browser: &mut Browser) -> Result<i64, Failure> {
    let timer = browser.send("GET", "/api/v1/me/timer", None).await?;
    Ok(timer.body["version"].as_i64().unwrap_or_default())
}

/// An entry of the actor on a new issue of the target project, booked through the API where they
/// may: its ID and version; an ID nobody made otherwise.
async fn own_entry(fixture: &Fixture<'_>, browser: &mut Browser) -> Result<(String, i64), Failure> {
    let issue = an_issue(fixture).await?;
    let booked = browser
        .send(
            "POST",
            "/api/v1/me/time-entries",
            Some(json!({"issue": issue, "date": "2027-01-14", "minutes": 30})),
        )
        .await?;
    match booked.body["id"].as_str() {
        Some(id) if booked.status.is_success() => {
            Ok((id.to_owned(), booked.body["version"].as_i64().unwrap_or(1)))
        }
        _ => Ok((NOBODYS_ENTRY.to_owned(), 1)),
    }
}

/// The request of a route of the timer or of time entries, if the operation is one.
pub(super) async fn time_request(
    fixture: &Fixture<'_>,
    operation: &str,
    browser: &mut Browser,
) -> Result<Option<Request>, Failure> {
    Ok(Some(match operation {
        "getTimer" => ("GET", "/api/v1/me/timer".into(), None),
        "startTimer" => (
            "POST",
            "/api/v1/me/timer".into(),
            Some(json!({"issue": an_issue(fixture).await?})),
        ),
        "stopTimer" => (
            "POST",
            "/api/v1/me/timer/stop".into(),
            Some(json!({"version": timer_version(browser).await?})),
        ),
        "switchTimer" => (
            "POST",
            "/api/v1/me/timer/switch".into(),
            Some(json!({
                "version": timer_version(browser).await?,
                "issue": an_issue(fixture).await?,
            })),
        ),
        "changeTimerStart" => (
            "POST",
            "/api/v1/me/timer/start-time".into(),
            Some(json!({
                "version": timer_version(browser).await?,
                "started_at": fixture.rig().clock_now().to_string(),
            })),
        ),
        "throwAwayTimer" => (
            "POST",
            "/api/v1/me/timer/throw-away".into(),
            Some(json!({"version": timer_version(browser).await?})),
        ),
        "listMyTimeEntries" => (
            "GET",
            "/api/v1/me/time-entries?from=2027-01-11&to=2027-01-17".into(),
            None,
        ),
        "addTimeEntry" => (
            "POST",
            "/api/v1/me/time-entries".into(),
            Some(json!({
                "issue": an_issue(fixture).await?,
                "date": "2027-01-14",
                "minutes": 45,
            })),
        ),
        "changeTimeEntry" => {
            let (entry, version) = own_entry(fixture, browser).await?;
            (
                "PATCH",
                format!("/api/v1/me/time-entries/{entry}"),
                Some(json!({"version": version, "minutes": 50})),
            )
        }
        "deleteTimeEntry" => {
            let (entry, version) = own_entry(fixture, browser).await?;
            (
                "DELETE",
                format!("/api/v1/me/time-entries/{entry}?version={version}"),
                None,
            )
        }
        "listIssueTimeEntries" => (
            "GET",
            format!("/api/v1/issues/{}/time-entries", an_issue(fixture).await?),
            None,
        ),
        "getTimeEntryDefaults" => (
            "GET",
            format!(
                "/api/v1/issues/{}/time-entry-defaults",
                an_issue(fixture).await?
            ),
            None,
        ),
        "getIssueTime" => (
            "GET",
            format!("/api/v1/issues/{}/time", an_issue(fixture).await?),
            None,
        ),
        "getProjectTime" => (
            "GET",
            format!("/api/v1/projects/{}/time", fixture.project(Project::Target)),
            None,
        ),
        _ => return Ok(None),
    }))
}
