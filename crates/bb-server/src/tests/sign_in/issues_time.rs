//! *Estimated time* and *Time left* of an issue over HTTP (TIME-03, ADR-0035, decision 4): they are
//! changed like any field, with a version, an event and a row of the history, one issue or many
//! at once, and taking a typed *Time left* away makes it worked out again. Both databases.

use axum::http::StatusCode;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, key_of, ok};

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
        // Taking the typed value away leaves the estimate: time left is worked out again.
        let cleared = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": 3, "time_left": null})),
            )
            .await?;
        let cleared = ok(&cleared)?;
        assert!(cleared.get("time_left").is_none(), "{cleared}");
        assert_eq!(cleared["estimated_time"], 240);

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
        }
        Ok(())
    })
    .await
}
