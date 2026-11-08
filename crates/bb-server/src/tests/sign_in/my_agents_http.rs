//! *My agents* over HTTP (AI-37, ADR-0029): the list of the work of a person's own agents, and when
//! an agent last signed in. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, key_of, ok};
use super::rig::PASSWORD;

#[tokio::test]
async fn ai_37_my_agents_lists_the_work_of_my_agents_and_nobody_elses() -> TestResult {
    each_database("ai_37_my_agents_work", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tess, _) = site.person(StartingRole::TeamMember, "tess").await?;
        let (mut other, _) = site.person(StartingRole::TeamMember, "olga").await?;
        let agent = tess
            .send("POST", "/api/v1/agents", Some(json!({"name": "Fixer"})))
            .await?;
        let agent = ok(&agent)?["id"].as_str().ok_or("no agent ID")?.to_owned();
        let issue = key_of(&site.create(&mut ada, "Hand this over").await?)?;
        let handed = tess
            .send(
                "POST",
                "/api/v1/hand-overs",
                Some(json!({"agent": agent, "issues": [issue], "note": "Please fix"})),
            )
            .await?;
        ok(&handed)?;
        let mine = tess.send("GET", "/api/v1/me/agents/work", None).await?;
        let body = ok(&mine)?;
        let items = body["items"].as_array().ok_or("no items")?;
        assert_eq!(items.len(), 1, "{body}");
        assert_eq!(items[0]["issue"]["readable_id"], json!(issue), "{body}");
        assert_eq!(items[0]["work"]["state"], json!("waiting-for-the-agent"));
        assert_eq!(items[0]["work"]["agent"], json!(agent));
        assert!(items[0]["waits_for"].is_null(), "{body}");
        assert_eq!(body["truncated"], json!(false));
        // Another person with no agent sees an empty list, whatever else exists.
        let nobody = other.send("GET", "/api/v1/me/agents/work", None).await?;
        assert_eq!(
            ok(&nobody)?["items"].as_array().map(Vec::len),
            Some(0),
            "{}",
            nobody.body
        );
        // The list needs a session.
        let mut stranger = Browser::new(&app, &super::http::called());
        let refused = stranger.send("GET", "/api/v1/me/agents/work", None).await?;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{}", refused.body);
        Ok::<(), Failure>(())
    })
    .await
}

#[tokio::test]
async fn ai_30_the_list_of_the_organization_says_when_an_agent_last_signed_in() -> TestResult {
    each_database("ai_30_agents_last_use", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tess, _) = site.person(StartingRole::TeamMember, "tess").await?;
        let made = tess
            .send("POST", "/api/v1/agents", Some(json!({"name": "Fixer"})))
            .await?;
        let agent = ok(&made)?["id"].as_str().ok_or("no agent ID")?.to_owned();
        let key = tess
            .send(
                "POST",
                &format!("/api/v1/agents/{agent}/keys"),
                Some(json!({"password": PASSWORD, "name": "Key of Fixer"})),
            )
            .await?;
        let key = ok(&key)?["secret"].as_str().ok_or("no secret")?.to_owned();
        let asked = "/api/v1/organization/agents?fields=id,last_used_at";
        let before = ada.send("GET", asked, None).await?;
        assert!(ok(&before)?["items"][0].get("last_used_at").is_none());
        let mut script = Browser::new(&app, &super::http::called());
        let own = script
            .script("GET", &format!("/api/v1/agents/{agent}"), None, &key)
            .await?;
        assert_eq!(own.status, StatusCode::OK, "{}", own.body);
        let after = ada.send("GET", asked, None).await?;
        let item = &ok(&after)?["items"][0];
        assert!(item["last_used_at"].is_string(), "{item}");
        Ok::<(), Failure>(())
    })
    .await
}

#[tokio::test]
async fn ai_30_an_agent_handed_to_a_person_is_listed_for_them_until_they_accept() -> TestResult {
    each_database("ai_30_agent_offered", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let (mut tess, _) = site.person(StartingRole::TeamMember, "tess").await?;
        let (mut olga, olga_id) = site.person(StartingRole::TeamMember, "olga").await?;
        let made = tess
            .send("POST", "/api/v1/agents", Some(json!({"name": "Fixer"})))
            .await?;
        let made = ok(&made)?;
        let agent = made["id"].as_str().ok_or("no agent ID")?.to_owned();
        let handed = tess
            .send(
                "POST",
                &format!("/api/v1/agents/{agent}/hand-over"),
                Some(json!({"to": olga_id.to_string(), "version": made["version"]})),
            )
            .await?;
        assert_eq!(handed.status, StatusCode::NO_CONTENT, "{}", handed.body);
        let before = olga.send("GET", "/api/v1/me/agents/work", None).await?;
        let offered = &ok(&before)?["offered"];
        assert_eq!(offered.as_array().map(Vec::len), Some(1), "{offered}");
        assert_eq!(offered[0]["id"], json!(agent));
        assert_eq!(offered[0]["name"], json!("Fixer"));
        // The person who hands it over does not get it listed as offered to them.
        let own = tess.send("GET", "/api/v1/me/agents/work", None).await?;
        assert_eq!(ok(&own)?["offered"].as_array().map(Vec::len), Some(0));
        let accepted = olga
            .send("POST", &format!("/api/v1/agents/{agent}/accept"), None)
            .await?;
        assert_eq!(accepted.status, StatusCode::NO_CONTENT, "{}", accepted.body);
        let after = olga.send("GET", "/api/v1/me/agents/work", None).await?;
        assert_eq!(ok(&after)?["offered"].as_array().map(Vec::len), Some(0));
        Ok::<(), Failure>(())
    })
    .await
}

#[tokio::test]
async fn ai_37_a_person_changes_the_limits_of_an_agent_over_http() -> TestResult {
    each_database("ai_37_agent_limits", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let (mut tess, _) = site.person(StartingRole::TeamMember, "tess").await?;
        let made = tess
            .send("POST", "/api/v1/agents", Some(json!({"name": "Fixer"})))
            .await?;
        let made = ok(&made)?;
        let agent = made["id"].as_str().ok_or("no agent ID")?.to_owned();
        let most = made["maximums"].clone();
        assert!(most["changes_a_minute"].as_i64().is_some_and(|n| n >= 1), "{made}");
        let path = format!("/api/v1/agents/{agent}");
        let changed = tess
            .send(
                "PATCH",
                &path,
                Some(json!({
                    "version": made["version"],
                    "limits": {"changes_a_minute": 10, "issues_at_once": 2, "attempts_on_one_issue": 3},
                })),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        let list = tess.send("GET", "/api/v1/agents", None).await?;
        let item = &ok(&list)?["items"][0];
        assert_eq!(item["changes_a_minute"], json!(10));
        assert_eq!(item["issues_at_once"], json!(2));
        // More than the organization allows is refused, and so is asking for two changes at once.
        let too_much = tess
            .send(
                "PATCH",
                &path,
                Some(json!({
                    "version": item["version"],
                    "limits": {
                        "changes_a_minute": most["changes_a_minute"].as_i64().unwrap_or(0) + 1,
                        "issues_at_once": 2,
                        "attempts_on_one_issue": 3,
                    },
                })),
            )
            .await?;
        assert_eq!(too_much.code(), "request.invalid", "{}", too_much.body);
        Ok::<(), Failure>(())
    })
    .await
}
