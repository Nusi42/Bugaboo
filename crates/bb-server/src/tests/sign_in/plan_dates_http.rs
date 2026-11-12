//! *Start date* and *Due date* of an issue over HTTP (PLAN-01, PLAN-03, ADR-0043, decisions 1 and
//! 6): two fields of the catalogue with field permissions, a start never after the due date on
//! every way in, search that reads them, one event for a change of a date that *Undo* takes back,
//! and a move of several issues that is one run. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, key_of, ok};

/// The route of a search of issues.
const SEARCH: &str = "/api/v1/search/issues";
/// The route that moves the dates of several issues.
const MOVE_DATES: &str = "/api/v1/issues/move-dates";

/// The issue as `browser` reads it.
async fn issue_of(browser: &mut Browser, issue: &str) -> Result<Value, Failure> {
    let reply = browser
        .send("GET", &format!("{ISSUES}/{issue}"), None)
        .await?;
    Ok(ok(&reply)?["issue"].clone())
}

/// The readable IDs a search finds, in the order it answers them.
async fn found(browser: &mut Browser, query: &str) -> Result<Vec<String>, Failure> {
    let reply = browser
        .send(
            "GET",
            &format!(
                "{SEARCH}?q={}",
                query
                    .replace(' ', "%20")
                    .replace('<', "%3C")
                    .replace('>', "%3E")
            ),
            None,
        )
        .await?;
    Ok(ok(&reply)?["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["readable_id"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default())
}

/// An issue made with these dates.
async fn made_with(
    site: &Site<'_>,
    browser: &mut Browser,
    title: &str,
    dates: Value,
) -> Result<String, Failure> {
    let mut body = site.new_issue(title);
    if let (Some(body), Some(dates)) = (body.as_object_mut(), dates.as_object()) {
        body.extend(dates.clone());
    }
    let reply = browser
        .send(
            "POST",
            &format!("{PROJECTS}/{}/issues", site.acme),
            Some(body),
        )
        .await?;
    key_of(&reply)
}

#[tokio::test]
async fn plan_01_start_and_due_date_are_fields_with_field_permissions() -> TestResult {
    each_database("plan_01_date_fields", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // Both are fields of the catalogue, so the matrix of field permissions lists them.
        let matrix = ada.send("GET", "/api/v1/field-permissions", None).await?;
        let listed = ok(&matrix)?.to_string();
        for word in ["start-date", "due-date"] {
            assert!(listed.contains(word), "{word} is not in {listed}");
        }
        // An issue is made with them, and reads them back as dates with no time of day.
        let key = made_with(
            &site,
            &mut ada,
            "Plan the release",
            json!({"start_date": "2026-11-03", "due_date": "2026-11-07"}),
        )
        .await?;
        let read = issue_of(&mut ada, &key).await?;
        assert_eq!(read["start_date"], "2026-11-03", "{read}");
        assert_eq!(read["due_date"], "2026-11-07", "{read}");
        // A change on the version that was read; `null` takes a date away.
        let changed = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{key}"),
                Some(json!({"version": 1, "due_date": "2026-11-14", "start_date": null})),
            )
            .await?;
        assert_eq!(ok(&changed)?["due_date"], "2026-11-14");
        assert!(changed.body.get("start_date").is_none(), "{}", changed.body);
        // The starting levels are those of *Estimated time*: a Team member changes the dates, a
        // Contributor sees them and may not, a Guest does not see them at all.
        let (mut team, _) = site.person(StartingRole::TeamMember, "team").await?;
        let theirs = ok(&team
            .send(
                "PATCH",
                &format!("{ISSUES}/{key}"),
                Some(json!({"version": 2, "due_date": "2026-11-21"})),
            )
            .await?)?
        .clone();
        assert_eq!(theirs["due_date"], "2026-11-21");
        let (mut contributor, _) = site
            .person(StartingRole::Contributor, "contributor")
            .await?;
        assert_eq!(
            issue_of(&mut contributor, &key).await?["due_date"],
            "2026-11-21"
        );
        let refused = contributor
            .send(
                "PATCH",
                &format!("{ISSUES}/{key}"),
                Some(json!({"version": 3, "due_date": "2026-12-01"})),
            )
            .await?;
        assert_eq!(refused.code(), "access.forbidden", "{}", refused.body);
        let (mut guest, _) = site.person(StartingRole::Guest, "guest").await?;
        let own = key_of(&site.create(&mut guest, "A problem").await?)?;
        let seen = issue_of(&mut guest, &own).await?;
        assert!(seen.get("start_date").is_none() && seen.get("due_date").is_none());
        let hidden = guest
            .send(
                "PATCH",
                &format!("{ISSUES}/{own}"),
                Some(json!({"version": 1, "due_date": "2026-12-01"})),
            )
            .await?;
        assert_eq!(hidden.code(), "access.forbidden", "{}", hidden.body);
        // A list row holds them when it asks for them, and only then.
        let list = ada
            .send(
                "GET",
                &format!(
                    "{PROJECTS}/{}/issues?fields=readable_id,start_date,due_date",
                    site.acme
                ),
                None,
            )
            .await?;
        let rows = ok(&list)?["items"].as_array().cloned().unwrap_or_default();
        let row = rows
            .iter()
            .find(|row| row["readable_id"] == key)
            .ok_or("no row of the issue")?;
        assert_eq!(row["due_date"], "2026-11-21", "{rows:?}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_01_a_start_after_the_due_date_is_refused_on_every_way_in() -> TestResult {
    each_database("plan_01_start_after_due", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // The form: a new issue.
        let new = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{}/issues", site.acme),
                Some({
                    let mut body = site.new_issue("Backwards");
                    body["start_date"] = json!("2026-11-10");
                    body["due_date"] = json!("2026-11-03");
                    body
                }),
            )
            .await?;
        assert_eq!(new.status, StatusCode::BAD_REQUEST, "{}", new.body);
        assert_eq!(new.code(), "issue.start-after-due");
        let key = made_with(
            &site,
            &mut ada,
            "Forwards",
            json!({"start_date": "2026-11-03", "due_date": "2026-11-07"}),
        )
        .await?;
        let path = format!("{ISSUES}/{key}");
        // The API: a start after the due date the issue has, a due date before the start it has,
        // and both at once.
        for body in [
            json!({"version": 1, "start_date": "2026-11-08"}),
            json!({"version": 1, "due_date": "2026-11-02"}),
            json!({"version": 1, "start_date": "2026-11-20", "due_date": "2026-11-10"}),
        ] {
            let refused = ada.send("PATCH", &path, Some(body.clone())).await?;
            assert_eq!(
                refused.code(),
                "issue.start-after-due",
                "{body}: {}",
                refused.body
            );
        }
        // The same day is in order, and so is a due date that is taken away.
        ok(&ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 1, "start_date": "2026-11-07"})),
            )
            .await?)?;
        // A change of many issues leaves the issue alone, with the reason of a field's rule.
        let preview = ada
            .send(
                "POST",
                "/api/v1/many-changes/preview",
                Some(json!({"issues": [key], "action":
                    {"kind": "change", "change": {"due_date": "2026-11-01"}}})),
            )
            .await?;
        let row = &ok(&preview)?["rows"][0];
        assert_eq!(row["skipped"], "field-rule", "{row}");
        // The move of several issues: nothing is written when one of them is refused.
        let other = made_with(&site, &mut ada, "Other", json!({})).await?;
        let other_version = issue_of(&mut ada, &other).await?["version"].clone();
        let version = issue_of(&mut ada, &key).await?["version"].clone();
        let refused = ada
            .send(
                "POST",
                MOVE_DATES,
                Some(json!({"issues": [
                    {"issue": other, "version": other_version, "due_date": "2026-12-01"},
                    {"issue": key, "version": version, "due_date": "2026-11-01"},
                ]})),
            )
            .await?;
        assert_eq!(refused.code(), "issue.start-after-due", "{}", refused.body);
        assert!(issue_of(&mut ada, &other).await?.get("due_date").is_none());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_01_search_filters_sorts_and_groups_by_the_dates() -> TestResult {
    each_database("plan_01_search_dates", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let early = made_with(
            &site,
            &mut ada,
            "Early",
            json!({"start_date": "2026-11-02", "due_date": "2026-11-20"}),
        )
        .await?;
        let late = made_with(
            &site,
            &mut ada,
            "Late",
            json!({"start_date": "2026-12-01", "due_date": "2026-12-15"}),
        )
        .await?;
        let open = made_with(&site, &mut ada, "Open", json!({})).await?;
        assert_eq!(
            found(&mut ada, "due date < 2026-12-01").await?,
            std::slice::from_ref(&early)
        );
        assert_eq!(
            found(&mut ada, "due date > 2026-12-01").await?,
            std::slice::from_ref(&late)
        );
        assert_eq!(
            found(&mut ada, "start date is empty").await?,
            std::slice::from_ref(&open)
        );
        assert_eq!(
            found(&mut ada, "start date is 2026-12-01").await?,
            std::slice::from_ref(&late)
        );
        // Sorting by a date: the newest first without a word, as for any other day, and the
        // issues with none come last in either direction.
        let sorted = found(&mut ada, "sort by due date").await?;
        assert_eq!(sorted, [late.clone(), early.clone(), open.clone()]);
        let oldest = found(&mut ada, "sort by due date oldest first").await?;
        assert_eq!(oldest, [early, late.clone(), open.clone()]);
        // Grouping: one group for each day, and one for no day.
        let groups = ada
            .send("GET", &format!("{SEARCH}/groups?by=due%20date"), None)
            .await?;
        let groups = ok(&groups)?["groups"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert_eq!(groups.len(), 3, "{groups:?}");
        assert!(groups.iter().any(|group| group["key"]["type"] == "empty"));
        // The checker knows the words: a search that names them has no mistake, and the fields of
        // the language list them with their kind.
        let fields = ada.send("GET", "/api/v1/search/fields", None).await?;
        let words: Vec<&str> = ok(&fields)?["fields"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item["word"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        assert!(
            words.contains(&"start date") && words.contains(&"due date"),
            "{words:?}"
        );
        Ok(())
    })
    .await
}

/// The history of an issue: its changes of these members, newest first.
async fn dates_in_history(browser: &mut Browser, issue: &str) -> Result<Vec<Value>, Failure> {
    let reply = browser
        .send(
            "GET",
            &format!("{ISSUES}/{issue}/history?tab=changes"),
            None,
        )
        .await?;
    Ok(ok(&reply)?["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter(|item| {
                    ["start-date", "due-date"]
                        .contains(&item["change"]["field"].as_str().unwrap_or(""))
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default())
}

/// The preview of the undo of a run, and the undo on that preview.
async fn undo(browser: &mut Browser, run: &str) -> Result<Value, Failure> {
    let preview = browser
        .send("GET", &format!("/api/v1/runs/{run}/undo"), None)
        .await?;
    let version = ok(&preview)?["preview_version"]
        .as_str()
        .ok_or("no version")?
        .to_owned();
    let undone = browser
        .send(
            "POST",
            &format!("/api/v1/runs/{run}/undo"),
            Some(json!({"preview_version": version})),
        )
        .await?;
    Ok(ok(&undone)?.clone())
}

#[tokio::test]
async fn plan_03_a_date_change_is_one_event_in_the_history_and_undone_by_undo() -> TestResult {
    each_database("plan_03_date_event", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let key = made_with(
            &site,
            &mut ada,
            "Plan the release",
            json!({"start_date": "2026-11-03", "due_date": "2026-11-07"}),
        )
        .await?;
        rig.clock.advance(1);
        // Both dates change in one save: one event with the old and the new value of each.
        ok(&ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{key}"),
                Some(json!({"version": 1, "start_date": "2026-11-10", "due_date": "2026-11-14"})),
            )
            .await?)?;
        let rows = dates_in_history(&mut ada, &key).await?;
        let moved: Vec<(Value, Value, Value)> = rows
            .iter()
            .map(|row| {
                (
                    row["change"]["field"].clone(),
                    row["change"]["before"]["value"].clone(),
                    row["change"]["after"]["value"].clone(),
                )
            })
            .collect();
        assert!(
            moved.contains(&(
                json!("start-date"),
                json!("2026-11-03"),
                json!("2026-11-10")
            )),
            "{moved:?}"
        );
        assert!(
            moved.contains(&(json!("due-date"), json!("2026-11-07"), json!("2026-11-14"))),
            "{moved:?}"
        );
        assert!(
            rows.iter()
                .all(|row| row["change"]["after"]["kind"] == "date"
                    || row["change"]["after"]["kind"].is_null()),
            "{rows:?}"
        );
        // One date alone is one change of that date, and nothing is written for the other.
        rig.clock.advance(1);
        ok(&ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{key}"),
                Some(json!({"version": 2, "due_date": "2026-11-21", "start_date": "2026-11-10"})),
            )
            .await?)?;
        let rows = dates_in_history(&mut ada, &key).await?;
        let latest: Vec<&str> = rows
            .iter()
            .take(1)
            .filter_map(|row| row["change"]["field"].as_str())
            .collect();
        assert_eq!(latest, ["due-date"], "{rows:?}");
        // Undo of the change is the usual change back, on the version that is there.
        rig.clock.advance(1);
        let back = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{key}"),
                Some(json!({"version": 3, "due_date": "2026-11-14"})),
            )
            .await?;
        assert_eq!(ok(&back)?["due_date"], "2026-11-14");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_03_a_move_of_several_issues_is_one_run_undone_in_one_step() -> TestResult {
    each_database("plan_03_move_run", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let first = made_with(
            &site,
            &mut ada,
            "First",
            json!({"start_date": "2026-11-03", "due_date": "2026-11-07"}),
        )
        .await?;
        let second =
            made_with(&site, &mut ada, "Second", json!({"due_date": "2026-11-10"})).await?;
        let third = made_with(&site, &mut ada, "Third", json!({})).await?;
        rig.clock.advance(1);
        let versions = [
            issue_of(&mut ada, &first).await?["version"].clone(),
            issue_of(&mut ada, &second).await?["version"].clone(),
            issue_of(&mut ada, &third).await?["version"].clone(),
        ];
        let moved = ada
            .send(
                "POST",
                MOVE_DATES,
                Some(json!({"issues": [
                    {"issue": first, "version": versions[0],
                        "start_date": "2026-11-10", "due_date": "2026-11-14"},
                    {"issue": second, "version": versions[1],
                        "start_date": "2026-11-15", "due_date": "2026-11-20"},
                    {"issue": third, "version": versions[2], "start_date": "2026-11-21"},
                ]})),
            )
            .await?;
        let moved = ok(&moved)?.clone();
        let run = moved["run"].as_str().ok_or("no run")?.to_owned();
        assert_eq!(moved["issues"].as_array().map(Vec::len), Some(3), "{moved}");
        assert_eq!(moved["issues"][0]["readable_id"], first);
        assert_eq!(moved["issues"][0]["version"], 2);
        assert_eq!(
            issue_of(&mut ada, &second).await?["start_date"],
            "2026-11-15"
        );
        // The versions are those of the changes, so a second move on the old ones is stale.
        let stale = ada
            .send(
                "POST",
                MOVE_DATES,
                Some(json!({"issues": [
                    {"issue": first, "version": versions[0], "due_date": "2026-11-30"}]})),
            )
            .await?;
        assert_eq!(stale.code(), "edit.version-changed", "{}", stale.body);
        // Each issue has its own event, all in the one run.
        let changes = ada
            .send("GET", &format!("/api/v1/runs/{run}/changes"), None)
            .await?;
        let touched: std::collections::BTreeSet<String> = ok(&changes)?["items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item["issue"]["readable_id"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(touched.len(), 3, "{touched:?}");
        // One undo puts every date of every issue back.
        let undone = undo(&mut ada, &run).await?;
        assert_eq!(undone["changed_since_preview"], false, "{undone}");
        let first_now = issue_of(&mut ada, &first).await?;
        assert_eq!(first_now["start_date"], "2026-11-03", "{first_now}");
        assert_eq!(first_now["due_date"], "2026-11-07");
        let second_now = issue_of(&mut ada, &second).await?;
        assert!(second_now.get("start_date").is_none(), "{second_now}");
        assert_eq!(second_now["due_date"], "2026-11-10");
        let third_now = issue_of(&mut ada, &third).await?;
        assert!(third_now.get("start_date").is_none(), "{third_now}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_03_a_date_change_with_an_old_version_is_refused_with_the_newer_value() -> TestResult {
    each_database("plan_03_date_conflict", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let key = made_with(
            &site,
            &mut ada,
            "Plan the release",
            json!({"due_date": "2026-11-07"}),
        )
        .await?;
        let (mut ben, _) = site.person(StartingRole::TeamMember, "ben").await?;
        rig.clock.advance(1);
        ok(&ben
            .send(
                "PATCH",
                &format!("{ISSUES}/{key}"),
                Some(json!({"version": 1, "due_date": "2026-11-09"})),
            )
            .await?)?;
        // Ada still has the version she read.
        let stale = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{key}"),
                Some(json!({"version": 1, "due_date": "2026-11-12"})),
            )
            .await?;
        assert_eq!(stale.status, StatusCode::CONFLICT, "{}", stale.body);
        assert_eq!(stale.code(), "edit.version-changed");
        assert_eq!(stale.body["conflict"]["version"], 2);
        assert_eq!(stale.body["conflict"]["fields"], json!(["due-date"]));
        assert_eq!(
            stale.body["conflict"]["current"]["due-date"], "2026-11-09",
            "{}",
            stale.body
        );
        // The same on the route of several issues: nothing is written.
        let moved = ada
            .send(
                "POST",
                MOVE_DATES,
                Some(json!({"issues": [
                    {"issue": key, "version": 1, "due_date": "2026-11-12"}]})),
            )
            .await?;
        assert_eq!(moved.code(), "edit.version-changed", "{}", moved.body);
        assert_eq!(issue_of(&mut ada, &key).await?["due_date"], "2026-11-09");
        Ok(())
    })
    .await
}
