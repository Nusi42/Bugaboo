//! The example requests of the routes of work times (PLAN-11, PLAN-15, PRIV-09, SEC-MATRIX).
//!
//! One's own work times need nobody else: every actor reads and changes their own. Another
//! person's are those of the administrator of the matrix. An away time to change or remove is one
//! the actor entered for themselves a moment before, so the call runs on a real record (a key
//! never reaches work times, so it names one nobody made). A share is set in the target project.

use bb_testing::Failure;
use jiff::Span;
use jiff::civil::Date;
use serde_json::{Value, json};

use super::examples::Request;
use super::routes::Fixture;
use crate::sign_in_tests::http::Browser;

const MINE: &str = "/api/v1/me/work-times";
const RANGE: &str = "from=2026-01-01&to=2027-12-31";
/// An away time nobody entered.
const NOBODYS: &str = "01a3185c-5000-73b2-8f37-20757db0778e";

/// Seven usual work days, Monday to Thursday with 6 hours.
fn a_week() -> Value {
    json!({"days": (0..7).map(|at| json!({"works": at < 4, "minutes": 360})).collect::<Vec<_>>()})
}

/// The days of an away time, a few days of its own for each call so the calls do not meet.
fn some_days(fixture: &Fixture<'_>) -> Result<Value, Failure> {
    let first = Date::new(2026, 1, 1)?
        .checked_add(Span::new().days(i64::from(fixture.next_number()) * 3))?;
    let last = first.checked_add(Span::new().days(1))?;
    Ok(json!({"first": first.to_string(), "last": last.to_string()}))
}

/// The ID and the version of an away time the actor enters for themselves; a record nobody made
/// where the actor cannot enter one.
async fn own_away_time(
    fixture: &Fixture<'_>,
    browser: &mut Browser,
) -> Result<(String, i64), Failure> {
    let made = browser
        .send("POST", "/api/v1/me/away-times", Some(some_days(fixture)?))
        .await?;
    Ok(
        match (made.body["id"].as_str(), made.body["version"].as_i64()) {
            (Some(id), Some(version)) => (id.to_owned(), version),
            _ => (NOBODYS.to_owned(), 1),
        },
    )
}

/// The request of a route of work times, if the operation is one.
pub(super) async fn work_times_request(
    fixture: &Fixture<'_>,
    operation: &str,
    browser: &mut Browser,
) -> Result<Option<Request>, Failure> {
    let others = format!("/api/v1/people/{}", fixture.admin_id());
    let target = fixture.project(super::actors::Project::Target);
    Ok(Some(match operation {
        "getMyWorkTimes" => ("GET", format!("{MINE}?{RANGE}"), None),
        "getWorkTimesOfPerson" => ("GET", format!("{others}/work-times?{RANGE}"), None),
        "changeMyUsualDays" => ("PUT", format!("{MINE}/usual-days"), Some(a_week())),
        "changeUsualDaysOfPerson" => (
            "PUT",
            format!("{others}/work-times/usual-days"),
            Some(a_week()),
        ),
        "enterMyAwayTime" => (
            "POST",
            "/api/v1/me/away-times".into(),
            Some(some_days(fixture)?),
        ),
        "enterAwayTimeForPerson" => (
            "POST",
            format!("{others}/away-times"),
            Some(some_days(fixture)?),
        ),
        "changeAwayTime" => {
            let (id, version) = own_away_time(fixture, browser).await?;
            let change = json!({"version": version, "days": some_days(fixture)?});
            ("PATCH", format!("/api/v1/away-times/{id}"), Some(change))
        }
        "removeAwayTime" => {
            let (id, version) = own_away_time(fixture, browser).await?;
            (
                "DELETE",
                format!("/api/v1/away-times/{id}?version={version}"),
                None,
            )
        }
        "listProjectShares" => ("GET", format!("/api/v1/projects/{target}/shares"), None),
        "setShare" => (
            "PUT",
            format!("/api/v1/projects/{target}/shares/{}", fixture.admin_id()),
            Some(json!({"percent": 60})),
        ),
        _ => return Ok(None),
    }))
}
