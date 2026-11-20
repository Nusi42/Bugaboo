//! The example requests of the routes of working hours (TIME-08, TIME-12, PRIV-09, SEC-MATRIX).
//!
//! One's own days need nobody else: every actor starts, taps and changes their own. Another
//! person's days are those of the administrator of the matrix, on a date of their own for each
//! call, so a change never meets the change of another actor. The days older than the keep period
//! are counted and deleted for everybody.

use bb_testing::Failure;
use jiff::Span;
use jiff::civil::Date;
use serde_json::{Value, json};

use super::examples::Request;
use super::routes::Fixture;

const MINE: &str = "/api/v1/me/work-days";
const OLD: &str = "/api/v1/work-days/old";
const KEEP_RULES: &str = "/api/v1/admin/keep-rules";

/// A date of its own for each call: one day further from the first of 2026 every time.
fn a_date(fixture: &Fixture<'_>) -> Result<String, Failure> {
    let first = Date::new(2026, 1, 1)?;
    Ok(first
        .checked_add(Span::new().days(i64::from(fixture.next_number())))?
        .to_string())
}

/// New times of a day of `date`, with a reason.
fn a_change(date: &str) -> Value {
    json!({
        "version": 0,
        "periods": [{"start": format!("{date}T08:00:00Z"), "end": format!("{date}T16:00:00Z")}],
        "breaks": [],
        "reason": "Typed in afterwards",
    })
}

/// The request of a route of working hours or of the keep rules, if the operation is one.
pub(super) fn working_hours_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    let date = a_date(fixture)?;
    let others = format!("/api/v1/people/{}/work-days", fixture.admin_id());
    let tap = Some(json!({"version": 1}));
    Ok(Some(match operation {
        "listMyWorkDays" => ("GET", format!("{MINE}?from={date}&to={date}"), None),
        "startWorkDay" => ("POST", format!("{MINE}/start"), Some(json!({}))),
        "startBreak" => ("POST", format!("{MINE}/break/start"), tap),
        "endBreak" => ("POST", format!("{MINE}/break/end"), tap),
        "endWorkDay" => ("POST", format!("{MINE}/end"), tap),
        "changeMyWorkDay" => ("PATCH", format!("{MINE}/{date}"), Some(a_change(&date))),
        "getMyWorkDayHistory" => ("GET", format!("{MINE}/{date}/history"), None),
        "listPeopleOfWorkDays" => ("GET", "/api/v1/work-days/people?search=a".into(), None),
        "getMyBookedWeek" => ("GET", format!("{MINE}/booked?date={date}"), None),
        "listWorkDaysOfPerson" => ("GET", format!("{others}?from={date}&to={date}"), None),
        "changeWorkDayOfPerson" => ("PATCH", format!("{others}/{date}"), Some(a_change(&date))),
        "getWorkDayHistoryOfPerson" => ("GET", format!("{others}/{date}/history"), None),
        "countOldWorkDays" => ("GET", OLD.into(), None),
        // A date that is not the one the count gives: refused for an administrator, as a deletion
        // that does not delete what they counted (`keeping.past-changed`).
        "deleteOldWorkDays" => ("DELETE", format!("{OLD}?before={date}"), None),
        // The keep rules, whose kinds an administrator deletes are work days and time entries.
        "listKeepRules" => ("GET", KEEP_RULES.into(), None),
        "countPastKeepPeriod" => ("GET", format!("{KEEP_RULES}/time-entries/past"), None),
        "countWouldDelete" => (
            "GET",
            format!("{KEEP_RULES}/notifications/would-delete?count=30"),
            None,
        ),
        "deletePastKeepPeriod" => (
            "POST",
            format!("{KEEP_RULES}/time-entries/delete-past?before={date}"),
            None,
        ),
        _ => return Ok(None),
    }))
}
