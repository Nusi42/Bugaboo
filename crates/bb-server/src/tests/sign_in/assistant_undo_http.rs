//! What the assistant says about taking an applied change back, over HTTP on both databases (AI-05):
//! the version an *Apply* leaves the issue at, so *Undo* of fields never overwrites a change made
//! meanwhile; the way back of a comment, a time entry and a new issue, and only for what the person may
//! take away; and no way back for stopping the timer, whose recorded time is never erased.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_app::ai::{AiAnswer, AiToolCall};
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, id_of, key_of, version_of};
use super::rig::{PASSWORD, Rig};

const AI: &str = "/api/v1/ai";

/// A model on the organization's own network, switched on.
fn local_model_on() -> Value {
    json!({
        "enabled": true,
        "provider": "openai-compatible",
        "address": "http://models.lan/v1",
        "model": "local-test",
        "password": PASSWORD,
    })
}

/// The model asks for one tool and then says it proposed it.
fn will_propose(rig: &Rig, tool: &str, arguments: Value) {
    rig.ai.will_answer(AiAnswer {
        tool_calls: vec![AiToolCall {
            id: "call-1".to_owned(),
            name: tool.to_owned(),
            arguments,
        }],
        ..AiAnswer::default()
    });
    rig.ai.will_answer(AiAnswer {
        text: "Proposed.".to_owned(),
        ..AiAnswer::default()
    });
}

/// Asks the question as `who` and applies the first proposal; the answer of *Apply*.
async fn ask_and_apply(
    rig: &Rig,
    who: &mut Browser,
    text: &str,
    tool: &str,
    input: Value,
) -> Result<Value, Failure> {
    will_propose(rig, tool, input);
    let asked = who
        .send(
            "POST",
            "/api/v1/assistant/questions",
            Some(json!({"text": text})),
        )
        .await?;
    assert_eq!(asked.status, StatusCode::OK, "{}", asked.body);
    let proposal = asked.body["proposals"][0].clone();
    assert!(
        proposal.is_object(),
        "no proposal for {tool}: {} {:?}",
        asked.body,
        rig.ai
            .calls()
            .last()
            .map(|call| format!("{:?}", call.request.turns))
    );
    let applied = who
        .send(
            "POST",
            "/api/v1/assistant/apply",
            Some(json!({
                "reference": proposal["reference"],
                "tool": proposal["tool"],
                "input": proposal["input"],
            })),
        )
        .await?;
    assert_eq!(applied.status, StatusCode::OK, "{}", applied.body);
    Ok(applied.body)
}

#[tokio::test]
async fn ai_05_undo_of_fields_is_made_on_the_version_apply_left_and_refuses_a_newer_one()
-> TestResult {
    each_database("ai_05_undo_version_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let set = ada.send("PUT", AI, Some(local_model_on())).await?;
        assert_eq!(set.status, StatusCode::OK, "{}", set.body);
        let key = key_of(&site.create(&mut ada, "Login").await?)?;
        let before = version_of(&mut ada, &key).await?;
        let applied = ask_and_apply(
            &rig,
            &mut ada,
            "Make it high",
            "update_issue",
            json!({"issue": key, "version": before, "priority": "high"}),
        )
        .await?;
        let left = version_of(&mut ada, &key).await?;
        assert_eq!(applied["version"], json!(left));
        assert!(applied.get("undo").is_none(), "{applied}");
        // Somebody else changes the issue before the person takes the change back.
        let (mut bob, _) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        let theirs = bob
            .send(
                "PATCH",
                &format!("/api/v1/issues/{key}"),
                Some(json!({"version": left, "title": "Bob changed this"})),
            )
            .await?;
        assert_eq!(theirs.status, StatusCode::OK, "{}", theirs.body);
        let undo = ada
            .send(
                "PATCH",
                &format!("/api/v1/issues/{key}"),
                Some(json!({"version": left, "priority": "normal"})),
            )
            .await?;
        assert_eq!(undo.status, StatusCode::CONFLICT, "{}", undo.body);
        let kept = ada
            .send("GET", &format!("/api/v1/issues/{key}"), None)
            .await?;
        assert_eq!(kept.body["issue"]["title"], json!("Bob changed this"));
        assert_eq!(kept.body["issue"]["priority"], json!("high"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_05_what_an_apply_made_says_how_to_take_it_away_and_stopping_the_timer_has_no_way_back()
-> TestResult {
    each_database("ai_05_undo_made_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let set = ada.send("PUT", AI, Some(local_model_on())).await?;
        assert_eq!(set.status, StatusCode::OK, "{}", set.body);
        let key = key_of(&site.create(&mut ada, "Login").await?)?;
        // A comment is put into the trash by its ID.
        let comment = ask_and_apply(
            &rig,
            &mut ada,
            "Comment that it looks fine",
            "add_comment",
            json!({"issue": key, "text": "Looks fine."}),
        )
        .await?;
        assert_eq!(comment["undo"]["kind"], json!("comment"));
        let id = comment["undo"]["id"].as_str().ok_or("no id")?;
        let gone = ada
            .send("DELETE", &format!("/api/v1/comments/{id}"), None)
            .await?;
        assert_eq!(gone.status, StatusCode::NO_CONTENT, "{}", gone.body);
        // A new issue goes to the trash for the person who made it, at the version it was made at.
        let made = ask_and_apply(
            &rig,
            &mut ada,
            "Create an issue called Fresh",
            "create_issue",
            json!({"project": "ACME", "issue_type": site.kind, "title": "Fresh"}),
        )
        .await?;
        assert_eq!(made["undo"]["kind"], json!("issue"));
        assert_eq!(made["undo"]["version"], json!(1));
        // Recorded time: the way back is the entry that was booked, on the version it was made at.
        let booked = ask_and_apply(
            &rig,
            &mut ada,
            "Book 30 minutes on it",
            "log_time",
            json!({"issue": key, "date": "2027-01-15", "minutes": 30}),
        )
        .await?;
        assert_eq!(booked["undo"]["kind"], json!("time-entry"));
        let entry = booked["undo"]["id"].as_str().ok_or("no entry")?;
        let version = booked["undo"]["version"].as_i64().ok_or("no version")?;
        let removed = ada
            .send(
                "DELETE",
                &format!("/api/v1/me/time-entries/{entry}?version={version}"),
                None,
            )
            .await?;
        assert_eq!(removed.status, StatusCode::NO_CONTENT, "{}", removed.body);
        // Stopping the timer records working time, and nothing takes it away again.
        let started = ada
            .send(
                "POST",
                "/api/v1/me/timer",
                Some(json!({"issue": key, "version": 0})),
            )
            .await?;
        assert!(started.status.is_success(), "{}", started.body);
        rig.clock.advance(120);
        let timer = ada.send("GET", "/api/v1/me/timer", None).await?;
        let on = timer.body["version"].as_i64().ok_or("no timer version")?;
        let stopped = ask_and_apply(
            &rig,
            &mut ada,
            "Stop my timer",
            "stop_timer",
            json!({"version": on}),
        )
        .await?;
        assert!(stopped.get("undo").is_none(), "{stopped}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_05_a_new_issue_has_a_way_back_exactly_when_the_person_may_delete_it() -> TestResult {
    each_database("ai_05_undo_delete_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let set = ada.send("PUT", AI, Some(local_model_on())).await?;
        assert_eq!(set.status, StatusCode::OK, "{}", set.body);
        for (role, name) in [
            (StartingRole::TeamMember, "pat"),
            (StartingRole::Contributor, "quin"),
        ] {
            let (mut person, _) = site.person_in_acme(role, name).await?;
            // Whether the person may delete an issue they reported, asked the way the app asks.
            let own = id_of(&site.create(&mut person, "Own").await?)?;
            let may = person
                .send("DELETE", &format!("/api/v1/issues/{own}"), None)
                .await?
                .status
                .is_success();
            let made = ask_and_apply(
                &rig,
                &mut person,
                "Create an issue called Mine",
                "create_issue",
                json!({"project": "ACME", "issue_type": site.kind, "title": "Mine"}),
            )
            .await?;
            assert_eq!(made.get("undo").is_some(), may, "{role:?}: {made}");
        }
        Ok(())
    })
    .await
}
