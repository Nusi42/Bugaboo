//! The example requests of the routes of the timeline, the board and the calendar (PLAN-01,
//! PLAN-06, PLAN-09, PLAN-13, PLAN-14, SEC-MATRIX): a page of the timeline, the board and the
//! calendar of everything the actor sees, and the preview of *Move it after*, the accepted warning
//! and the time factor of an issue of the target project with a start date and the administrator as
//! its person on the bar.

use bb_testing::Failure;
use jiff::civil::Date;
use serde_json::json;

use super::examples::Request;
use super::examples_issues::a_planned_issue;
use super::routes::Fixture;

/// A Monday with no public holiday.
const MONDAY: &str = "2026-03-02";

/// An issue of the target project the timeline gives a bar, planned for the administrator.
async fn a_bar(fixture: &Fixture<'_>) -> Result<String, Failure> {
    let start: Date = MONDAY.parse()?;
    a_planned_issue(fixture, Some(start), Some(vec![fixture.admin_id()])).await
}

/// The request of a route of the timeline, the board or the calendar, if the operation is one. An
/// access key reads the timeline like a search, but no work times: its bars keep the dates people
/// set, and it gets no hatched day and no warning about what a person can work (architecture §12).
pub(super) async fn timeline_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    Ok(Some(match operation {
        "getTimeline" => (
            "POST",
            "/api/v1/timeline".into(),
            Some(json!({
                "search_text": "",
                "timeline": {"zoom": "week", "plan_by_effort": true},
                "first": MONDAY,
                "last": "2026-03-31",
            })),
        ),
        "previewMoveAfter" => (
            "POST",
            "/api/v1/timeline/move-after".into(),
            Some(json!({
                "issue": a_bar(fixture).await?,
                "after": "2026-03-04",
                "timeline": {"zoom": "week"},
            })),
        ),
        "acceptWarning" => (
            "POST",
            "/api/v1/timeline/accepted-warnings".into(),
            // Taken as it is said: *Too much at once* depends on the other work on the screen.
            Some(json!({
                "issue": a_bar(fixture).await?,
                "version": 1,
                "kind": "too-much-at-once",
                "timeline": {"zoom": "week"},
            })),
        ),
        "getTimeFactor" => (
            "GET",
            format!("/api/v1/issues/{}/time-factor", a_bar(fixture).await?),
            None,
        ),
        "setTimeFactor" => (
            "PUT",
            format!("/api/v1/issues/{}/time-factor", a_bar(fixture).await?),
            Some(json!({"version": 1, "percent": 150})),
        ),
        // The board and the calendar of everything the actor sees: like a page of the timeline,
        // whoever is signed in is let in and shown what they read, a key as its person.
        "getBoard" => (
            "POST",
            "/api/v1/board".into(),
            Some(json!({"search_text": "", "board": {"rows": "priority"}})),
        ),
        "getCalendar" => (
            "POST",
            "/api/v1/calendar".into(),
            Some(json!({
                "search_text": "",
                "calendar": {"dates_field": null, "span": "month"},
                "first": MONDAY,
                "last": "2026-03-31",
            })),
        ),
        _ => return Ok(None),
    }))
}
