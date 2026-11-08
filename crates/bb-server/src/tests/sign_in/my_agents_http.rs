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
