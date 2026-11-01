//! *Estimated time* and *Time left* of an issue over HTTP (TIME-03, ADR-0035, decision 4): they are
//! changed like any field, with a version, an event and a row of the history, one issue or many
//! at once, and taking a typed *Time left* away makes it worked out again. Both databases.

use axum::http::StatusCode;
use bb_app::access::Permission;
use bb_app::roles::SetFieldLevel;
use bb_core::access::FieldLevel;
use bb_core::fields::Field;
use bb_core::id::ProjectId;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, key_of, ok};
use super::policy_rig::Member;
use super::rig::Rig;

/// The changes of the history of an issue, as `browser` reads them.
async fn changes_of(browser: &mut Browser, issue: &str) -> Result<Vec<Value>, Failure> {
    let reply = browser
        .send(
            "GET",
            &format!("{ISSUES}/{issue}/history?tab=changes"),
            None,
        )
        .await?;
    Ok(ok(&reply)?["items"].as_array().cloned().unwrap_or_default())
}

/// The issue as `browser` reads it.
async fn issue_of(browser: &mut Browser, issue: &str) -> Result<Value, Failure> {
    let reply = browser
        .send("GET", &format!("{ISSUES}/{issue}"), None)
        .await?;
    Ok(ok(&reply)?["issue"].clone())
}

#[tokio::test]
async fn time_03_estimated_time_and_time_left_are_changed_like_any_field() -> TestResult {
    each_database("time_03_change_estimates", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Plan the release").await?)?;
        let path = format!("{ISSUES}/{issue}");
        let fresh = issue_of(&mut ada, &issue).await?;
        assert!(fresh.get("estimated_time").is_none(), "{fresh}");
        assert!(fresh.get("time_left").is_none(), "{fresh}");

        // An estimate, then a typed time left: each a change on the version that was read.
        let estimated = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 1, "estimated_time": 240})),
            )
            .await?;
        assert_eq!(ok(&estimated)?["estimated_time"], 240);
        assert_eq!(ok(&estimated)?["version"], 2);
        let typed = ada
            .send("PATCH", &path, Some(json!({"version": 2, "time_left": 90})))
            .await?;
        assert_eq!(ok(&typed)?["time_left"], 90);
        assert_eq!(ok(&typed)?["time_left_worked_out"], false);
        // A change made on an old version is refused.
        let stale = ada
            .send("PATCH", &path, Some(json!({"version": 2, "time_left": 30})))
            .await?;
        assert_eq!(stale.status, StatusCode::CONFLICT, "{}", stale.body);
        // A length below zero is no length.
        let negative = ada
            .send("PATCH", &path, Some(json!({"version": 3, "time_left": -5})))
            .await?;
        assert_eq!(
            negative.status,
            StatusCode::BAD_REQUEST,
            "{}",
            negative.body
        );
        // Taking the typed value away leaves the estimate: time left is worked out again, from
        // nothing spent.
        let cleared = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 3, "time_left": null})),
            )
            .await?;
        let cleared = ok(&cleared)?;
        // The answer of the change holds only a typed value; the issue's time works it out.
        assert!(cleared.get("time_left").is_none(), "{cleared}");
        assert_eq!(cleared["estimated_time"], 240);
        let time = ada.send("GET", &format!("{path}/time"), None).await?;
        assert_eq!(
            ok(&time)?["time_left"],
            json!({"minutes": 240, "worked_out": true})
        );

        // The history words each change as a length, in the seconds the screens get for one.
        let history = changes_of(&mut ada, &issue).await?;
        let lengths: Vec<(Value, Value, Value)> = history
            .iter()
            .filter(|item| {
                ["estimated-time", "time-left"]
                    .contains(&item["change"]["field"].as_str().unwrap_or(""))
            })
            .map(|item| {
                (
                    item["change"]["field"].clone(),
                    item["change"]["before"]["value"].clone(),
                    item["change"]["after"]["value"].clone(),
                )
            })
            .collect();
        assert_eq!(
            lengths,
            [
                (json!("time-left"), json!(5400), Value::Null),
                (json!("time-left"), Value::Null, json!(5400)),
                (json!("estimated-time"), Value::Null, json!(14_400)),
            ],
            "{history:?}"
        );
        assert!(
            history
                .iter()
                .filter(|item| item["change"]["field"] == "estimated-time")
                .all(|item| item["change"]["after"]["kind"] == "duration")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_03_many_issues_get_an_estimate_at_once() -> TestResult {
    each_database("time_03_many_estimates", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut issues = Vec::new();
        for title in ["One", "Two"] {
            issues.push(key_of(&site.create(&mut ada, title).await?)?);
        }
        let action =
            json!({"kind": "change", "change": {"estimated_time": 120, "time_left": null}});
        let preview = ada
            .send(
                "POST",
                "/api/v1/many-changes/preview",
                Some(json!({"issues": issues, "action": action})),
            )
            .await?;
        let preview = ok(&preview)?.clone();
        let targets: Vec<Value> = preview["rows"]
            .as_array()
            .ok_or("no rows")?
            .iter()
            .map(|row| json!({"issue": row["address"], "version": row["version"]}))
            .collect();
        let changed = ada
            .send(
                "POST",
                "/api/v1/many-changes",
                Some(json!({"issues": targets, "action": action})),
            )
            .await?;
        ok(&changed)?;
        for issue in &issues {
            let read = issue_of(&mut ada, issue).await?;
            assert_eq!(read["estimated_time"], 120, "{issue}: {read}");
            assert!(read.get("time_left").is_none(), "{issue}: {read}");
            let time = ada
                .send("GET", &format!("{ISSUES}/{issue}/time"), None)
                .await?;
            assert_eq!(
                ok(&time)?["time_left"],
                json!({"minutes": 120, "worked_out": true}),
                "{issue}"
            );
        }
        Ok(())
    })
    .await
}

/// Puts `child` under `parent`.
async fn put_under(browser: &mut Browser, child: &str, parent: &str) -> Result<(), Failure> {
    let version = issue_of(browser, child).await?["version"].clone();
    ok(&browser
        .send(
            "PUT",
            &format!("{ISSUES}/{child}/parent"),
            Some(json!({"version": version, "parent": parent})),
        )
        .await?)?;
    Ok(())
}

/// Books `minutes` on an issue, as the person of the browser.
async fn book(browser: &mut Browser, issue: &str, minutes: u32) -> Result<(), Failure> {
    ok(&browser
        .send(
            "POST",
            "/api/v1/me/time-entries",
            Some(json!({"issue": issue, "date": "2027-01-14", "minutes": minutes})),
        )
        .await?)?;
    Ok(())
}

/// Gives an issue an estimate.
async fn estimate(browser: &mut Browser, issue: &str, minutes: u32) -> Result<(), Failure> {
    let version = issue_of(browser, issue).await?["version"].clone();
    ok(&browser
        .send(
            "PATCH",
            &format!("{ISSUES}/{issue}"),
            Some(json!({"version": version, "estimated_time": minutes})),
        )
        .await?)?;
    Ok(())
}

#[tokio::test]
async fn time_03_the_time_of_an_issue_and_of_a_project_over_http() -> TestResult {
    each_database("time_03_time_routes", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let goal = key_of(&site.create(&mut ada, "Release").await?)?;
        let task = key_of(&site.create(&mut ada, "Write notes").await?)?;
        let step = key_of(&site.create(&mut ada, "Check links").await?)?;
        put_under(&mut ada, &task, &goal).await?;
        put_under(&mut ada, &step, &task).await?;
        estimate(&mut ada, &goal, 240).await?;
        estimate(&mut ada, &task, 120).await?;
        estimate(&mut ada, &step, 60).await?;
        book(&mut ada, &goal, 80).await?;
        book(&mut ada, &step, 90).await?;
        let time = ada
            .send("GET", &format!("{ISSUES}/{goal}/time"), None)
            .await?;
        let time = ok(&time)?;
        assert_eq!(time["estimated_time"], 240, "{time}");
        assert_eq!(
            time["time_left"],
            json!({"minutes": 160, "worked_out": true})
        );
        assert_eq!(time["time_spent"], 80);
        // Every level below: the step is under the task, which is under the goal.
        assert_eq!(
            time["with_sub_issues"],
            json!({"estimated_time": 420, "time_left": 160 + 120, "time_spent": 170})
        );
        let alone = ada
            .send("GET", &format!("{ISSUES}/{step}/time"), None)
            .await?;
        assert!(ok(&alone)?.get("with_sub_issues").is_none());
        let project = ada.send("GET", "/api/v1/projects/ACME/time", None).await?;
        assert_eq!(
            ok(&project)?,
            &json!({"estimated_time": 420, "time_left": 280, "time_spent": 170})
        );
        // A list row says the same, when it asks for it.
        let list = ada
            .send(
                "GET",
                "/api/v1/projects/ACME/issues?fields=readable_id,time_left,time_left_worked_out,time_spent",
                None,
            )
            .await?;
        let rows = ok(&list)?["items"].as_array().cloned().unwrap_or_default();
        let row = rows
            .iter()
            .find(|row| row["readable_id"] == json!(step))
            .ok_or("no row of the step")?;
        assert_eq!(row["time_spent"], 90);
        assert_eq!(row["time_left"], 0);
        assert_eq!(row["time_left_worked_out"], true);
        Ok(())
    })
    .await
}

/// A person who sees every issue and reads the three lengths of time everywhere but *Time spent*
/// in the project `hidden_in`.
async fn reader_without_spent_in(
    site: &Site<'_>,
    rig: &Rig,
    hidden_in: ProjectId,
) -> Result<Browser, Failure> {
    let role = site
        .org
        .role(
            "Time reader",
            &[Permission::SeeProjects, Permission::SeeIssues],
        )
        .await?;
    let levels = [
        (Field::Title, None, FieldLevel::CanSee),
        (Field::EstimatedTime, None, FieldLevel::CanSee),
        (Field::TimeLeft, None, FieldLevel::CanSee),
        (Field::TimeSpent, None, FieldLevel::CanSee),
        (Field::TimeSpent, Some(hidden_in), FieldLevel::Hidden),
    ];
    for (field, project, level) in levels {
        rig.api()
            .set_field_level(
                Rig::info(),
                SetFieldLevel {
                    session_token: site.org.admin.session.clone(),
                    role: role.id,
                    project,
                    issue_type: None,
                    field: field.into(),
                    level: Some(level),
                },
            )
            .await?;
    }
    let email = "times@example.test";
    let started = rig.person(&site.org.admin.session, email).await?;
    let member = Member {
        id: started.person.id,
        session: started.session_token,
    };
    site.org.give_everywhere(&member, role.id).await?;
    Browser::signed_in_as(site.app, email).await
}

#[tokio::test]
async fn time_03_sub_issues_count_in_every_project_and_a_hidden_total_is_left_out() -> TestResult {
    each_database("time_03_sub_issues_elsewhere", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let other = ada
            .send(
                "POST",
                "/api/v1/projects",
                Some(json!({"short_name": "SIDE", "name": "Side"})),
            )
            .await?;
        let other_id: ProjectId = ok(&other)?["id"].as_str().ok_or("no ID")?.parse()?;
        let goal = key_of(&site.create(&mut ada, "Release").await?)?;
        let elsewhere = ada
            .send(
                "POST",
                "/api/v1/projects/SIDE/issues",
                Some(site.new_issue("Translate the notes")),
            )
            .await?;
        let elsewhere = key_of(&elsewhere)?;
        put_under(&mut ada, &elsewhere, &goal).await?;
        estimate(&mut ada, &goal, 60).await?;
        estimate(&mut ada, &elsewhere, 120).await?;
        book(&mut ada, &goal, 80).await?;
        book(&mut ada, &elsewhere, 30).await?;
        // A sub-issue in a sibling project counts like one in the same project.
        let time = ada
            .send("GET", &format!("{ISSUES}/{goal}/time"), None)
            .await?;
        assert_eq!(
            ok(&time)?["with_sub_issues"],
            json!({"estimated_time": 180, "time_left": 90, "time_spent": 110})
        );
        // A reader who may not read Time spent where the sub-issue is gets no total of it: not
        // the issue's own 80 minutes standing for the total.
        let mut reader = reader_without_spent_in(&site, &rig, other_id).await?;
        let seen = reader
            .send("GET", &format!("{ISSUES}/{goal}/time"), None)
            .await?;
        let seen = ok(&seen)?;
        assert_eq!(seen["time_spent"], 80, "{seen}");
        assert!(
            seen["with_sub_issues"].get("time_spent").is_none(),
            "{seen}"
        );
        assert_eq!(seen["with_sub_issues"]["estimated_time"], 180, "{seen}");
        Ok(())
    })
    .await
}
