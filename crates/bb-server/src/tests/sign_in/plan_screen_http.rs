//! What the screen of the timeline reads from a page besides the bars (PLAN-03, PLAN-04, ADR-0043):
//! whether a bar may be dragged, the goals with their due dates and the span of their sub-issues,
//! and rows grouped by an own field of choices. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::create_field;
use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, ok};
use super::plan_site::{MONDAY, ask, id_of_key, issue, page, planned, rows_of};

/// Puts `child` under `parent`, as `browser`.
async fn under(browser: &mut Browser, child: &str, parent: &str) -> Result<(), Failure> {
    let version = issue(browser, child).await?["version"].clone();
    let reply = browser
        .send(
            "PUT",
            &format!("{ISSUES}/{child}/parent"),
            Some(json!({"version": version, "parent": parent})),
        )
        .await?;
    ok(&reply)?;
    Ok(())
}

/// The goal of a page with this internal ID.
fn goal_of<'a>(page: &'a Value, id: &str) -> Option<&'a Value> {
    page["goals"]
        .as_array()?
        .iter()
        .find(|goal| goal["id"] == id)
}

#[tokio::test]
async fn plan_03_a_row_says_whether_the_reader_may_move_its_dates() -> TestResult {
    each_database("plan_03_may_move", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let key = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": MONDAY, "due_date": "2027-03-05"}),
        )
        .await?;
        let undated = planned(&site, &mut ada, "An idea", json!({})).await?;
        let body = ask(&site, MONDAY, "2027-03-31", json!({}));
        let may_move = |page: &Value, key: &str| rows_of(page, key)[0]["may_move"].clone();
        // Ada may change the issue and both of its dates.
        let theirs = page(&mut ada, body.clone()).await?;
        assert_eq!(may_move(&theirs, &key), json!(true), "{theirs}");
        let mut tray = body.clone();
        tray["part"] = json!("not-scheduled");
        let waiting = page(&mut ada, tray.clone()).await?;
        assert_eq!(may_move(&waiting, &undated), json!(true), "{waiting}");
        // A Viewer sees the bar but may not drag it, nor give dates to an issue of the tray.
        let (mut viewer, _) = site.person_in_acme(StartingRole::Viewer, "viewer").await?;
        let seen = page(&mut viewer, body).await?;
        assert_eq!(may_move(&seen, &key), json!(false), "{seen}");
        let waiting = page(&mut viewer, tray).await?;
        assert_eq!(may_move(&waiting, &undated), json!(false), "{waiting}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_04_a_goal_has_its_due_date_and_the_span_of_its_sub_issues() -> TestResult {
    each_database("plan_04_goal_span", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let goal = planned(
            &site,
            &mut ada,
            "Mobile app 2.0",
            json!({"due_date": "2027-03-12"}),
        )
        .await?;
        let early = planned(
            &site,
            &mut ada,
            "API",
            json!({"start_date": "2027-03-02", "due_date": "2027-03-04"}),
        )
        .await?;
        let late = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": "2027-03-08", "due_date": "2027-03-10"}),
        )
        .await?;
        let alone = planned(&site, &mut ada, "Alone", json!({"due_date": "2027-03-03"})).await?;
        under(&mut ada, &early, &goal).await?;
        under(&mut ada, &late, &goal).await?;
        let goal_id = id_of_key(&mut ada, &goal).await?;
        let read = page(&mut ada, ask(&site, MONDAY, "2027-03-31", json!({}))).await?;
        let found = goal_of(&read, &goal_id).ok_or("no goal")?;
        assert_eq!(found["due"], "2027-03-12", "{read}");
        assert_eq!(
            found["span"],
            json!({"first": "2027-03-02", "last": "2027-03-10"})
        );
        // An issue without sub-issues is no goal.
        let alone_id = id_of_key(&mut ada, &alone).await?;
        assert!(goal_of(&read, &alone_id).is_none(), "{read}");
        assert_eq!(read["goals"].as_array().map(Vec::len), Some(1));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_04_rows_group_by_an_own_field_of_choices() -> TestResult {
    each_database("plan_04_group_choice", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let made = create_field(
            &mut ada,
            json!({
                "name": "Severity",
                "kind": "pick-one",
                "applies_to": "issues",
                "choices": [{"name": "Minor"}, {"name": "Major"}],
            }),
        )
        .await?;
        let field = made["id"].as_str().ok_or("no field ID")?.to_owned();
        let major = made["choices"][1]["id"]
            .as_str()
            .ok_or("no choice ID")?
            .to_owned();
        let dates = json!({"start_date": MONDAY, "due_date": "2027-03-05"});
        let mut with_major = dates.clone();
        with_major["fields"] = json!({ &field: &major });
        let severe = planned(&site, &mut ada, "Severe", with_major).await?;
        let plain = planned(&site, &mut ada, "Plain", dates).await?;
        let read = page(
            &mut ada,
            ask(&site, MONDAY, "2027-03-31", json!({"group_by": "severity"})),
        )
        .await?;
        assert_eq!(
            rows_of(&read, &severe)[0]["group"],
            json!({"type": "choice", "value": major}),
            "{read}"
        );
        assert_eq!(rows_of(&read, &plain)[0]["group"], json!({"type": "empty"}));
        // A field that is no field of choices is still no grouping of the timeline.
        let refused = ada
            .send(
                "POST",
                super::plan_site::TIMELINE,
                Some(ask(
                    &site,
                    MONDAY,
                    "2027-03-31",
                    json!({"group_by": "title"}),
                )),
            )
            .await?;
        assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{}", refused.body);
        Ok(())
    })
    .await
}
