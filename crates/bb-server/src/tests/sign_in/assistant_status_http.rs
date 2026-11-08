//! Whether the assistant is there, over HTTP on both databases (AI-02, AI-03, PRIV-08, COMP-08): off at
//! the start, on for the organization and off for one project, a person who does not see a project,
//! a project kept from AI, and whether names of people go with the answers.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::http::rig_and_app;
use super::issues_site::Site;
use super::rig::PASSWORD;

const AI: &str = "/api/v1/ai";
const STATUS: &str = "/api/v1/assistant";

/// A model on the organization's own network, switched on.
fn local_model_on() -> serde_json::Value {
    json!({
        "enabled": true,
        "provider": "openai-compatible",
        "address": "http://models.lan/v1",
        "model": "local-test",
        "password": PASSWORD,
    })
}

#[tokio::test]
async fn ai_02_the_status_follows_the_switch_of_the_organization_and_of_the_project() -> TestResult
{
    each_database("ai_02_status_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        // Off at the start, for everybody, and nothing is sent.
        for who in [&mut ada, &mut bob] {
            let found = who.send("GET", STATUS, None).await?;
            assert_eq!(found.status, StatusCode::OK, "{}", found.body);
            assert_eq!(found.body["available"], json!(false));
            assert!(found.body.get("destination").is_none(), "{}", found.body);
        }
        let set = ada.send("PUT", AI, Some(local_model_on())).await?;
        assert_eq!(set.status, StatusCode::OK, "{}", set.body);
        let found = bob.send("GET", STATUS, None).await?;
        assert_eq!(found.body["available"], json!(true));
        assert_eq!(found.body["destination"], json!("local-model"));
        // A project that switched AI off has no assistant; the organization still has.
        let off = ada
            .send(
                "PUT",
                &format!("/api/v1/projects/ACME/ai"),
                Some(json!({"enabled": false})),
            )
            .await?;
        assert_eq!(off.status, StatusCode::OK, "{}", off.body);
        let in_project = bob
            .send("GET", &format!("{STATUS}?project=ACME"), None)
            .await?;
        assert_eq!(in_project.body["available"], json!(false));
        let elsewhere = bob.send("GET", STATUS, None).await?;
        assert_eq!(elsewhere.body["available"], json!(true));
        assert!(rig.ai.calls().is_empty(), "the status sends nothing");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_03_a_project_the_person_cannot_see_or_that_is_kept_from_ai_is_not_found() -> TestResult
{
    each_database("ai_03_status_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let other = ada
            .send(
                "POST",
                "/api/v1/projects",
                Some(json!({"short_name": "OTHER", "name": "Other"})),
            )
            .await?;
        assert_eq!(other.status, StatusCode::OK, "{}", other.body);
        let (mut bob, _) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        let set = ada.send("PUT", AI, Some(local_model_on())).await?;
        assert_eq!(set.status, StatusCode::OK, "{}", set.body);
        // A project Bob does not see answers as one that does not exist.
        let unseen = bob
            .send("GET", &format!("{STATUS}?project=OTHER"), None)
            .await?;
        assert_eq!(unseen.code(), "resource.not-found");
        let missing = bob
            .send("GET", &format!("{STATUS}?project=NOPE"), None)
            .await?;
        assert_eq!(missing.code(), "resource.not-found");
        let seen = bob
            .send("GET", &format!("{STATUS}?project=ACME"), None)
            .await?;
        assert_eq!(seen.body["available"], json!(true));
        // A project kept from AI is not there for the assistant either.
        let marks = ada.send("GET", "/api/v1/never-sent-to-ai", None).await?;
        assert_eq!(marks.status, StatusCode::OK);
        let marked = ada
            .send(
                "PUT",
                "/api/v1/never-sent-to-ai/projects/ACME",
                Some(json!({"marked": true, "version": 0})),
            )
            .await?;
        assert_eq!(marked.status, StatusCode::OK, "{}", marked.body);
        let kept = bob
            .send("GET", &format!("{STATUS}?project=ACME"), None)
            .await?;
        assert_eq!(kept.code(), "resource.not-found");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn comp_08_the_status_says_where_the_answers_come_from_and_whether_names_go() -> TestResult {
    each_database("comp_08_status_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        // A model of the organization's own: names go with it, nothing leaves.
        let own = ada.send("PUT", AI, Some(local_model_on())).await?;
        assert_eq!(own.status, StatusCode::OK, "{}", own.body);
        let found = bob.send("GET", STATUS, None).await?;
        assert_eq!(found.body["destination"], json!("local-model"));
        assert_eq!(found.body["names_sent"], json!(true));
        // An outside provider, after its agreement: people go as their role until names are switched on.
        let outside = ada
            .send(
                "PUT",
                AI,
                Some(json!({"enabled": false, "provider": "anthropic", "address": "", "model": "claude-test", "password": PASSWORD})),
            )
            .await?;
        assert_eq!(outside.status, StatusCode::OK, "{}", outside.body);
        let agreed = ada
            .send("POST", "/api/v1/ai/agreement", Some(json!({"destination": "anthropic"})))
            .await?;
        assert_eq!(agreed.status, StatusCode::OK, "{}", agreed.body);
        let on = ada.send("PUT", AI, Some(json!({"enabled": true}))).await?;
        assert_eq!(on.status, StatusCode::OK, "{}", on.body);
        let found = bob.send("GET", STATUS, None).await?;
        assert_eq!(found.body["destination"], json!("anthropic"));
        assert_eq!(found.body["names_sent"], json!(false));
        let names = ada.send("PUT", AI, Some(json!({"send_names": true}))).await?;
        assert_eq!(names.status, StatusCode::OK, "{}", names.body);
        let found = bob.send("GET", STATUS, None).await?;
        assert_eq!(found.body["names_sent"], json!(true));
        Ok(())
    })
    .await
}
