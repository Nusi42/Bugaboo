//! The calendar over HTTP (PLAN-09, PRJ-06, ADR-0034, amendment of 2026-11-24): the issues of a
//! window of days by the chosen date field, the move of a date with its version check, and the due
//! dates of goals as marks. Every flow runs on SQLite and PostgreSQL.

use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, ok};
use super::perf_queries::{Counter, assert_as_many_statements};
use super::plan_site::{id_of_key, issue, under};
use super::views_site::{CALENDAR, issue_in};

/// The calendar of ACME by `dates_field` (none for *Due date*) over March 2027.
fn march(site: &Site<'_>, dates_field: Option<&str>) -> Value {
    json!({
        "search_text": "",
        "project": site.acme,
        "calendar": {"dates_field": dates_field, "span": "month"},
        "first": "2027-03-01",
        "last": "2027-03-31",
    })
}

/// The calendar of `body`, as `browser` reads it.
async fn calendar(browser: &mut Browser, body: Value) -> Result<Value, Failure> {
    let reply = browser.send("POST", CALENDAR, Some(body)).await?;
    Ok(ok(&reply)?.clone())
}

/// The readable IDs and days of the entries of a calendar, in order.
fn entries(read: &Value) -> Vec<(String, String)> {
    read["entries"]
        .as_array()
        .map(|all| {
            all.iter()
                .map(|entry| {
                    (
                        entry["issue"]["readable_id"]
                            .as_str()
                            .unwrap_or_default()
                            .to_owned(),
                        entry["day"].as_str().unwrap_or_default().to_owned(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn plan_09_the_calendar_answers_a_window_of_days_by_the_chosen_date_field() -> TestResult {
    each_database("plan_09_window", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let kind = site.kind.clone();
        let late = issue_in(
            &mut ada,
            (&site.acme, &kind),
            "Late",
            json!({"start_date": "2027-02-20", "due_date": "2027-03-20"}),
        )
        .await?;
        let early = issue_in(
            &mut ada,
            (&site.acme, &kind),
            "Early",
            json!({"start_date": "2027-03-02", "due_date": "2027-04-02"}),
        )
        .await?;
        issue_in(&mut ada, (&site.acme, &kind), "Undated", json!({})).await?;
        // By *Due date*: only the issue whose due date is in March, on that day.
        let by_due = calendar(&mut ada, march(&site, None)).await?;
        assert_eq!(
            entries(&by_due),
            [(late.clone(), "2027-03-20".to_owned())],
            "{by_due}"
        );
        assert_eq!(by_due["more"], false);
        assert_eq!(by_due["entries"][0]["may_move"], true);
        // By *Start date*: the other one.
        let by_start = calendar(&mut ada, march(&site, Some("start date"))).await?;
        assert_eq!(
            entries(&by_start),
            [(early, "2027-03-02".to_owned())],
            "{by_start}"
        );
        // A window longer than six weeks, a backwards one and a field that is no date are refused.
        for (first, last, field) in [
            ("2027-03-01", "2027-04-30", None),
            ("2027-03-31", "2027-03-01", None),
            ("2027-03-01", "2027-03-31", Some("title")),
        ] {
            let mut body = march(&site, field);
            body["first"] = json!(first);
            body["last"] = json!(last);
            let refused = ada.send("POST", CALENDAR, Some(body)).await?;
            assert_eq!(refused.code(), "request.invalid", "{}", refused.body);
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_09_moving_an_issue_changes_that_date_and_an_old_version_is_refused_with_the_newer_value()
-> TestResult {
    each_database("plan_09_move_date", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let key = issue_in(
            &mut ada,
            (&site.acme, &site.kind),
            "Movable",
            json!({"due_date": "2027-03-10"}),
        )
        .await?;
        let looked = issue(&mut ada, &key).await?["version"].clone();
        // Ben moves it first.
        let moved = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{key}"),
                Some(json!({"version": looked, "due_date": "2027-03-12"})),
            )
            .await?;
        ok(&moved)?;
        let read = calendar(&mut ada, march(&site, None)).await?;
        assert_eq!(entries(&read), [(key.clone(), "2027-03-12".to_owned())]);
        // The drag on the old version is refused with the day the issue has now.
        let stale = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{key}"),
                Some(json!({"version": looked, "due_date": "2027-03-15"})),
            )
            .await?;
        assert_eq!(stale.code(), "edit.version-changed", "{}", stale.body);
        assert_eq!(stale.body["conflict"]["current"]["due-date"], "2027-03-12");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_09_goals_and_releases_stand_on_their_dates_as_marks() -> TestResult {
    each_database("plan_09_marks", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let goal_type = super::views_site::type_named(&mut ada, "Goal").await?;
        let goal = issue_in(
            &mut ada,
            (&site.acme, &goal_type),
            "Launch",
            json!({"due_date": "2027-03-25"}),
        )
        .await?;
        let step = issue_in(
            &mut ada,
            (&site.acme, &site.kind),
            "Step",
            json!({"due_date": "2027-03-05"}),
        )
        .await?;
        under(&mut ada, &step, &goal).await?;
        let goal_id = id_of_key(&mut ada, &goal).await?;
        let read = calendar(
            &mut ada,
            json!({
                "search_text": "title is Step",
                "project": site.acme,
                "calendar": {"dates_field": null, "span": "month"},
                "first": "2027-03-01",
                "last": "2027-03-31",
            }),
        )
        .await?;
        // The goal of the issue shown stands on its due date as a mark, not as an entry.
        assert_eq!(entries(&read), [(step, "2027-03-05".to_owned())], "{read}");
        let marks = read["marks"].as_array().cloned().unwrap_or_default();
        assert_eq!(marks.len(), 1, "{read}");
        assert_eq!(marks[0]["issue"], goal_id.as_str());
        assert_eq!(marks[0]["readable_id"], goal.as_str());
        assert_eq!(marks[0]["day"], "2027-03-25");
        assert_eq!(marks[0]["title"], "Launch");
        // Outside the window it is no mark.
        let april = calendar(
            &mut ada,
            json!({
                "search_text": "",
                "project": site.acme,
                "calendar": {"dates_field": null, "span": "month"},
                "first": "2027-03-01",
                "last": "2027-03-10",
            }),
        )
        .await?;
        assert_eq!(april["marks"], json!([]), "{april}");
        Ok(())
    })
    .await
}

/// The most statements a calendar may run, whatever its issues: the session and the rights, the
/// search prepared once, the one statement of the window, what the issues show once for all of
/// them, and the goals around them: 55 measured on both databases.
const CALENDAR_AT_MOST: usize = 55;

#[tokio::test]
async fn plan_09_a_window_answers_at_most_two_hundred_issues_with_a_fixed_number_of_statements()
-> TestResult {
    each_database("plan_09_two_hundred", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let due = |at: usize| format!("2027-03-{:02}", 1 + at % 28);
        for at in 0..10 {
            issue_in(
                &mut ada,
                (&site.acme, &site.kind),
                &format!("Due {at}"),
                json!({"due_date": due(at)}),
            )
            .await?;
        }
        let body = march(&site, None);
        let mut counter = Counter::on(&rig.database);
        calendar(&mut ada, body.clone()).await?;
        counter.take();
        let ten = calendar(&mut ada, body.clone()).await?;
        let on_ten = counter.take();
        assert_eq!(entries(&ten).len(), 10);
        assert_eq!(ten["more"], false);
        for at in 10..201 {
            issue_in(
                &mut ada,
                (&site.acme, &site.kind),
                &format!("Due {at}"),
                json!({"due_date": due(at)}),
            )
            .await?;
        }
        counter.take();
        let full = calendar(&mut ada, body).await?;
        let on_full = counter.take();
        assert_eq!(entries(&full).len(), 200, "{}", full["more"]);
        assert_eq!(full["more"], true);
        assert_as_many_statements(on_ten, on_full, CALENDAR_AT_MOST, "a fuller window");
        Ok(())
    })
    .await
}
