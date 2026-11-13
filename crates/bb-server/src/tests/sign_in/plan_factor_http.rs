//! The time factor over HTTP (PLAN-14, ADR-0030, decision 4 and amendment M8): the estimate of the
//! person on the bar, set by them always, back to ×1 when the first assigned person changes, never
//! searched, sorted or grouped by, and told to the person on the bar when somebody else changes it.
//! Every flow runs on SQLite and PostgreSQL.

use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{ISSUES, Site, ok};
use super::plan_privacy_http::field_level;
use super::plan_site::{MONDAY, by_effort, change, issue, page, planned, rows_of, work_times};

/// The route of the time factor of an issue.
fn factor_of(key: &str) -> String {
    format!("{ISSUES}/{key}/time-factor")
}

/// Sets the time factor of an issue as `browser`, on the version the issue has: the answer.
async fn set_factor(browser: &mut Browser, key: &str, given: Value) -> Result<Reply, Failure> {
    let version = browser.send("GET", &factor_of(key), None).await?.body["version"].clone();
    let mut body = json!({"version": version});
    if let (Some(body), Some(given)) = (body.as_object_mut(), given.as_object()) {
        body.extend(given.clone());
    }
    browser.send("PUT", &factor_of(key), Some(body)).await
}

/// The percent of the time factor of an issue, as `browser` reads it.
async fn percent(browser: &mut Browser, key: &str) -> Result<Value, Failure> {
    let read = browser.send("GET", &factor_of(key), None).await?;
    Ok(ok(&read)?["percent"].clone())
}

#[tokio::test]
async fn plan_14_the_time_factor_resets_when_the_person_changes() -> TestResult {
    each_database("plan_14_reset", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut lea, lea_id) = site.person(StartingRole::TeamMember, "lea").await?;
        let (_ben, ben_id) = site.person(StartingRole::TeamMember, "ben").await?;
        let (_cy, cy_id) = site.person(StartingRole::TeamMember, "cy").await?;
        let (lea_text, ben_text, cy_text) =
            (lea_id.to_string(), ben_id.to_string(), cy_id.to_string());
        let key = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": MONDAY, "estimated_time": 480, "assigned_to": [lea_text, ben_text]}),
        )
        .await?;
        ok(&set_factor(&mut lea, &key, json!({"percent": 150})).await?)?;
        assert_eq!(percent(&mut ada, &key).await?, 150);
        // Another person after the first, or one taken off after the first: it stays.
        ok(&change(&mut ada, &key, json!({"assigned_to": [lea_text, ben_text, cy_text]})).await?)?;
        ok(&change(&mut ada, &key, json!({"assigned_to": [lea_text, cy_text]})).await?)?;
        assert_eq!(percent(&mut ada, &key).await?, 150);
        // Another person first: back to ×1, in the same change, and the history says so.
        ok(&change(&mut ada, &key, json!({"assigned_to": [cy_text, lea_text]})).await?)?;
        let reset = ada.send("GET", &factor_of(&key), None).await?;
        assert_eq!(ok(&reset)?["percent"], 100);
        assert_eq!(reset.body["set_by"], Value::Null);
        let history = ada
            .send("GET", &format!("{ISSUES}/{key}/history"), None)
            .await?;
        let text = ok(&history)?.to_string();
        assert!(text.contains("time-factor"), "{text}");
        // The history gives the factor as a factor in percent, for the screen to write ×1.5.
        assert!(
            text.contains(&json!({"shown": "time-factor", "percent": 150}).to_string()),
            "{text}"
        );
        // The first taken off: back to ×1 as well.
        ok(&set_factor(&mut ada, &key, json!({"percent": 80})).await?)?;
        ok(&change(&mut ada, &key, json!({"assigned_to": [lea_text]})).await?)?;
        assert_eq!(percent(&mut ada, &key).await?, 100);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_14_the_person_on_the_bar_may_always_set_the_time_factor() -> TestResult {
    each_database("plan_14_person_on_bar", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // A Contributor may see the time factor but not change it, unless the issue is theirs.
        let (mut lea, lea_id) = site
            .person_called(StartingRole::Contributor, "lea", "Lea Lang")
            .await?;
        let (mut other, _) = site.person(StartingRole::Contributor, "other").await?;
        work_times(&site, &mut ada, lea_id, 480, 100).await?;
        let key = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": MONDAY, "estimated_time": 3840, "assigned_to": [lea_id.to_string()]}),
        )
        .await?;
        // *Takes me 10 days* against an estimate of 8 days: ×1.25.
        let set = set_factor(&mut lea, &key, json!({"takes_minutes": 4800})).await?;
        let shown = ok(&set)?.clone();
        assert_eq!(shown["percent"], 125, "{shown}");
        assert_eq!(shown["own_estimate"], true);
        let refused = set_factor(&mut other, &key, json!({"percent": 200})).await?;
        assert_eq!(refused.code(), "access.forbidden", "{}", refused.body);
        // The bar plans with it: 8 days times 1.25 are 10 work days.
        let found = page(&mut ada, by_effort(&site, MONDAY, "2027-03-31")).await?;
        let bar = &rows_of(&found, &key)[0]["bar"];
        assert_eq!(bar["days"]["last"], "2027-03-12", "{bar}");
        assert_eq!(bar["time_factor"], json!({"percent": 125, "own_estimate": true}));
        // Somebody else changes it: the person on the bar is told, and who set it is shown.
        let by_ada = set_factor(&mut ada, &key, json!({"percent": 150})).await?;
        assert_eq!(ok(&by_ada)?["own_estimate"], false);
        let told = lea.send("GET", "/api/v1/notifications", None).await?;
        let kinds: Vec<Value> = ok(&told)?["items"]
            .as_array()
            .map(|items| items.iter().map(|item| item["kind"].clone()).collect())
            .unwrap_or_default();
        assert!(kinds.contains(&json!("time-factor-changed")), "{kinds:?}");
        // Without an estimate a length is no request, and neither is a factor out of range.
        let plain = planned(
            &site,
            &mut ada,
            "No estimate",
            json!({"assigned_to": [lea_id.to_string()]}),
        )
        .await?;
        for (on, given) in [
            (&plain, json!({"takes_minutes": 480})),
            (&key, json!({"percent": 5})),
            (&key, json!({"percent": 120, "takes_minutes": 480})),
        ] {
            let bad = set_factor(&mut lea, on, given).await?;
            assert_eq!(bad.code(), "request.invalid", "{}", bad.body);
        }
        // An issue with nobody on the bar has no time factor to set.
        let nobody = planned(&site, &mut ada, "Nobody", json!({"estimated_time": 60})).await?;
        let none = set_factor(&mut ada, &nobody, json!({"percent": 120})).await?;
        assert_eq!(none.code(), "request.invalid", "{}", none.body);
        // A Guest does not see the field at all.
        let (mut guest, _) = site.person(StartingRole::Guest, "guest").await?;
        let own = super::issues_site::key_of(&site.create(&mut guest, "Mine").await?)?;
        let read = guest.send("GET", &factor_of(&own), None).await?;
        assert!(ok(&read)?.get("percent").is_none(), "{}", read.body);
        let _ = issue(&mut ada, &key).await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_14_no_report_or_search_sorts_by_time_factor() -> TestResult {
    each_database("plan_14_no_search", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // The search language has no word for it: no filter, no sort.
        for query in ["time%20factor%20%3E%201", "sort%20by%20time%20factor"] {
            let refused = ada
                .send("GET", &format!("/api/v1/search/issues?q={query}"), None)
                .await?;
            assert_eq!(
                refused.status,
                axum::http::StatusCode::BAD_REQUEST,
                "{query}: {}",
                refused.body
            );
        }
        // It is no field a search, a list column or a group can name.
        let fields = ada.send("GET", "/api/v1/search/fields", None).await?;
        assert!(
            !ok(&fields)?.to_string().contains("time factor"),
            "{}",
            fields.body
        );
        let grouped = ada
            .send(
                "GET",
                "/api/v1/search/issues/groups?q=&by=time%20factor",
                None,
            )
            .await?;
        assert_eq!(
            grouped.status,
            axum::http::StatusCode::BAD_REQUEST,
            "{}",
            grouped.body
        );
        let timeline = ada
            .send(
                "POST",
                super::plan_site::TIMELINE,
                Some(super::plan_site::ask(
                    &site,
                    MONDAY,
                    "2027-03-31",
                    json!({"group_by": "time factor"}),
                )),
            )
            .await?;
        assert_eq!(timeline.code(), "request.invalid", "{}", timeline.body);
        // A saved search cannot show it as a column.
        let saved = ada
            .send(
                "POST",
                "/api/v1/saved-searches",
                Some(json!({
                    "name": "By factor",
                    "search_text": "",
                    "presentation": {"view": "list", "columns": [{"field": "time factor", "width": null}], "group_by": null},
                })),
            )
            .await?;
        assert_eq!(
            saved.status,
            axum::http::StatusCode::BAD_REQUEST,
            "{}",
            saved.body
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_14_the_time_factor_says_who_may_change_it() -> TestResult {
    each_database("plan_14_may_change", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        super::ai_mcp_rig::ai_on(&rig).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // Lea is a Contributor, whose role may not change the field, and the person on the bar.
        let (mut lea, lea_id) = site
            .person_called(StartingRole::Contributor, "lea", "Lea Lang")
            .await?;
        let (mut vera, _) = site.person(StartingRole::Viewer, "vera").await?;
        let key = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"estimated_time": 480, "assigned_to": [lea_id.to_string()]}),
        )
        .await?;
        let route = factor_of(&key);
        // The person on the bar may, and so may somebody who may change the issue and the field.
        let own = lea.send("GET", &route, None).await?;
        assert_eq!(ok(&own)?["may_change"], true, "{}", own.body);
        let by_ada = ada.send("GET", &route, None).await?;
        assert_eq!(ok(&by_ada)?["may_change"], true, "{}", by_ada.body);
        // A Viewer may not.
        let by_vera = vera.send("GET", &route, None).await?;
        assert_eq!(ok(&by_vera)?["may_change"], false, "{}", by_vera.body);
        // The person on the bar with a key that may only look may not.
        let secret = super::plan_privacy_http::see_only_key(&mut lea).await?;
        let looking = lea.script("GET", &route, None, &secret).await?;
        assert_eq!(ok(&looking)?["may_change"], false, "{}", looking.body);
        // An agent never may, whatever its person may.
        let (_, agent) =
            super::ai_mcp_rig::an_agent_with_a_key(&mut ada, "Fixer", Some(&key)).await?;
        let by_agent = ada.script("GET", &route, None, &agent).await?;
        assert_eq!(ok(&by_agent)?["may_change"], false, "{}", by_agent.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_14_the_time_factor_names_no_person_where_assigned_to_is_hidden() -> TestResult {
    each_database("plan_14_factor_assigned_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut lea, lea_id) = site
            .person_called(StartingRole::TeamMember, "lea", "Lea Lang")
            .await?;
        let key = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": MONDAY, "estimated_time": 480, "assigned_to": [lea_id.to_string()]}),
        )
        .await?;
        ok(&set_factor(&mut lea, &key, json!({"percent": 150})).await?)?;
        // A reader whose field level hides *Assigned to* is told no person on the bar, by any
        // answer of the time factor.
        let (mut reader, _) = site.person(StartingRole::Contributor, "reader").await?;
        field_level(&site, &mut ada, StartingRole::Contributor, "assigned-to", "hidden").await?;
        let read = reader.send("GET", &factor_of(&key), None).await?;
        let read = ok(&read)?;
        assert_eq!(read["person"], Value::Null, "{read}");
        assert!(!read.to_string().contains("Lea Lang"), "{read}");
        Ok(())
    })
    .await
}
