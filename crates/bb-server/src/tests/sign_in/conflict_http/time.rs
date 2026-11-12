//! The stale saves of work days, the timer and time entries: the rows of the table of `conflict_http` for these routes.

use bb_testing::Failure;
use serde_json::json;

use super::{Browser, Save, made, read};
use crate::sign_in_tests::issues_site::{Site, ok, text_in};
use crate::sign_in_tests::time_http::{MY_ENTRIES, TIMER};
use bb_app::access::StartingRole;

/// The stale changes of a work day of Ada's: a day typed afterwards, or the day that runs.
pub(super) async fn work_day(
    id: &str,
    site: &Site<'_>,
    ada: &mut Browser,
) -> Result<Save, Failure> {
    // At most one change of a person's days a second.
    site.org.rig.clock.advance(1);
    let base = "/api/v1/me/work-days".to_owned();
    let times = |date: &str| {
        json!({
            "periods": [{"start": format!("{date}T08:00:00Z"), "end": format!("{date}T16:00:00Z")}],
            "breaks": [],
            "reason": "Typed in afterwards",
        })
    };
    let first = |date: &str| {
        let mut body = times(date);
        body["version"] = json!(0);
        body
    };
    if id == "changeMyWorkDay" {
        let date = "2027-01-10";
        let path = format!("{base}/{date}");
        let day = ok(&ada.send("PATCH", &path, Some(first(date))).await?)?.clone();
        return Save::new("PATCH", path, times(date), &day["version"]);
    }
    if id == "changeWorkDayOfPerson" {
        // Bob's day, which Ada changes once she holds *HR*.
        let date = "2027-01-09";
        let (mut bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob-days", "Bob Baker")
            .await?;
        let day = ok(&bob
            .send("PATCH", &format!("{base}/{date}"), Some(first(date)))
            .await?)?
        .clone();
        let hr = site.org.starting_roles().await?[StartingRole::Hr.code()];
        site.org.give_everywhere(&site.org.admin, hr).await?;
        return Save::new(
            "PATCH",
            format!("/api/v1/people/{bob_id}/work-days/{date}"),
            times(date),
            &day["version"],
        );
    }
    // The day that runs: started once, and read as it is now.
    ada.send("POST", "/api/v1/me/work-days/start", Some(json!({})))
        .await?;
    let today = read(ada, "/api/v1/me/work-days?from=2027-01-15&to=2027-01-15").await?;
    let tap = match id {
        "startBreak" => "break/start",
        "endBreak" => "break/end",
        _ => "end",
    };
    Save::new(
        "POST",
        format!("{base}/{tap}"),
        json!({}),
        &today["days"][0]["version"],
    )
}

/// The usual work days of Ada or of Bob, changed once so that they have a version, and a stale
/// change of them.
pub(super) async fn usual_days(
    id: &str,
    site: &Site<'_>,
    ada: &mut Browser,
) -> Result<Save, Failure> {
    let week = |minutes: u16| -> serde_json::Value {
        json!({"days": (0..7).map(|at| json!({"works": at < 4, "minutes": minutes})).collect::<Vec<_>>()})
    };
    let (path, read_path) = if id == "changeMyUsualDays" {
        (
            "/api/v1/me/work-times/usual-days".to_owned(),
            "/api/v1/me/work-times?from=2027-01-01&to=2027-01-02".to_owned(),
        )
    } else {
        let (_bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob-usual", "Bob Baker")
            .await?;
        (
            format!("/api/v1/people/{bob_id}/work-times/usual-days"),
            format!("/api/v1/people/{bob_id}/work-times?from=2027-01-01&to=2027-01-02"),
        )
    };
    let mut first = week(360);
    first["version"] = json!(0);
    ok(&ada.send("PUT", &path, Some(first)).await?)?;
    let now = read(ada, &read_path).await?;
    Save::new("PUT", path, week(300), &now["usual_version"])
}

/// An away time of Ada's, and a stale change or removal of it.
pub(super) async fn away_time(id: &str, ada: &mut Browser) -> Result<Save, Failure> {
    let made = made(
        ada,
        "/api/v1/me/away-times",
        json!({"first": "2027-02-01", "last": "2027-02-03"}),
    )
    .await?;
    let path = format!("/api/v1/away-times/{}", text_in(&made, "id")?);
    if id == "removeAwayTime" {
        return Ok(Save::new("DELETE", path, json!({}), &made["version"])?.in_the_query());
    }
    Save::new(
        "PATCH",
        path,
        json!({"days": {"first": "2027-02-01", "last": "2027-02-04"}}),
        &made["version"],
    )
}

/// The person's running timer (started on a new issue when it stands still), or one of their
/// entries, and a change of it.
pub(super) async fn entries(id: &str, site: &Site<'_>, ada: &mut Browser) -> Result<Save, Failure> {
    let issue = text_in(ok(&site.create(ada, "Timed").await?)?, "readable_id")?;
    if let "changeTimeEntry" | "deleteTimeEntry" = id {
        let entry = made(
            ada,
            MY_ENTRIES,
            json!({"issue": issue, "date": "2027-01-14", "minutes": 30}),
        )
        .await?;
        let path = format!("{MY_ENTRIES}/{}", text_in(&entry, "id")?);
        return if id == "changeTimeEntry" {
            Save::new("PATCH", path, json!({"minutes": 45}), &entry["version"])
        } else {
            Ok(Save::new("DELETE", path, json!({}), &entry["version"])?.in_the_query())
        };
    }
    let mut timer = read(ada, TIMER).await?;
    if timer.get("running").is_none() {
        timer = made(ada, TIMER, json!({"issue": issue})).await?;
    }
    let (path, body) = match id {
        "stopTimer" => ("stop", json!({})),
        "switchTimer" => ("switch", json!({"issue": issue})),
        "changeTimerStart" => (
            "start-time",
            json!({"started_at": timer["running"]["started_at"]}),
        ),
        _ => ("throw-away", json!({})),
    };
    Save::new("POST", format!("{TIMER}/{path}"), body, &timer["version"])
}
