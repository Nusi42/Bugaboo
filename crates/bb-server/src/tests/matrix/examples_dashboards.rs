//! The example requests of the routes of dashboards (CFG-03, SEC-MATRIX).
//!
//! Every route that works on a dashboard lets the actor make one first, through the same server: a
//! dashboard is private and the actor's own, so anybody who is signed in has one to work on. For
//! somebody who is not signed in, or who comes with an access key, the call is turned away before
//! it looks for the dashboard, so a dashboard nobody made stands in.

use bb_core::id::DashboardId;
use bb_testing::Failure;
use jiff::Timestamp;
use serde_json::{Value, json};

use super::actors::Project;
use super::examples::Request;
use super::routes::Fixture;
use crate::sign_in_tests::http::Browser;

const DASHBOARDS: &str = "/api/v1/dashboards";

/// An ID that no dashboard and no box has, in the form of one: an address that is no ID is not
/// found before the use case is asked who is calling.
fn nobodys() -> Result<String, Failure> {
    Ok(DashboardId::from_parts(Timestamp::UNIX_EPOCH, [6; 10])?.to_string())
}

/// *My timer*: a box every actor's dashboard can keep.
fn a_timer() -> Value {
    json!({"kind": "my-timer", "title": "Timer", "column": 0, "row": 0, "width": 3, "height": 2})
}

/// A dashboard of the actor with one box: its ID and the ID of the box.
async fn theirs(fixture: &Fixture<'_>, browser: &mut Browser) -> Result<(String, String), Failure> {
    let made = browser
        .send(
            "POST",
            DASHBOARDS,
            Some(json!({
                "name": format!("Dashboard {}", fixture.next_number()),
                "boxes": [a_timer()],
            })),
        )
        .await?;
    match (
        made.body["id"].as_str(),
        made.body["boxes"][0]["id"].as_str(),
    ) {
        (Some(dashboard), Some(each)) => Ok((dashboard.to_owned(), each.to_owned())),
        _ => Ok((nobodys()?, nobodys()?)),
    }
}

/// The request of a route of dashboards, if the operation is one.
pub(super) async fn dashboard_request(
    fixture: &Fixture<'_>,
    operation: &str,
    browser: &mut Browser,
) -> Result<Option<Request>, Failure> {
    let target = fixture.project(Project::Target).to_string();
    Ok(Some(match operation {
        "listDashboards" => ("GET", DASHBOARDS.into(), None),
        "createDashboard" => (
            "POST",
            DASHBOARDS.into(),
            Some(json!({"name": "Team", "boxes": [a_timer()]})),
        ),
        "getHome" => ("GET", format!("{DASHBOARDS}/home"), None),
        "listReadyMadeBoxes" => ("GET", format!("{DASHBOARDS}/ready-made-boxes"), None),
        "listDashboardShareChoices" => (
            "GET",
            format!("{DASHBOARDS}/share-choices?kind=project"),
            None,
        ),
        "previewDashboardBox" => ("POST", format!("{DASHBOARDS}/box-preview"), Some(a_timer())),
        "getDashboard" => {
            let (dashboard, _) = theirs(fixture, browser).await?;
            ("GET", format!("{DASHBOARDS}/{dashboard}"), None)
        }
        "getDashboardBox" => {
            let (dashboard, each) = theirs(fixture, browser).await?;
            (
                "GET",
                format!("{DASHBOARDS}/{dashboard}/boxes/{each}"),
                None,
            )
        }
        "changeDashboard" => {
            let (dashboard, _) = theirs(fixture, browser).await?;
            (
                "PATCH",
                format!("{DASHBOARDS}/{dashboard}"),
                Some(json!({"version": 1, "name": "Renamed"})),
            )
        }
        "deleteDashboard" => {
            let (dashboard, _) = theirs(fixture, browser).await?;
            ("DELETE", format!("{DASHBOARDS}/{dashboard}"), None)
        }
        "restoreDashboard" => {
            let (dashboard, _) = theirs(fixture, browser).await?;
            browser
                .send("DELETE", &format!("{DASHBOARDS}/{dashboard}"), None)
                .await?;
            ("POST", format!("{DASHBOARDS}/{dashboard}/restore"), None)
        }
        "copyDashboard" => {
            let (dashboard, _) = theirs(fixture, browser).await?;
            (
                "POST",
                format!("{DASHBOARDS}/{dashboard}/copy"),
                Some(json!({})),
            )
        }
        "shareDashboard" => {
            let (dashboard, _) = theirs(fixture, browser).await?;
            (
                "POST",
                format!("{DASHBOARDS}/{dashboard}/shares"),
                Some(json!({"kind": "project", "id": target})),
            )
        }
        "stopSharingDashboard" => {
            let (dashboard, _) = theirs(fixture, browser).await?;
            (
                "DELETE",
                format!("{DASHBOARDS}/{dashboard}/shares/project/{target}"),
                None,
            )
        }
        _ => return Ok(None),
    }))
}
