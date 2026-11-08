//! Every way back of an applied change, over HTTP on both databases (AI-05): what the browser sends
//! to take away what the assistant made (the timer, a new issue, a comment) or to put a field back
//! (status, *Assigned to*), and that each of them stops when somebody changed the thing since.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::assistant_undo_http::{AI, ask_and_apply, local_model_on};
use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, key_of, version_of};

/// The ID and the version a way back names.
fn made_of(applied: &Value) -> Result<(String, i64), Failure> {
    let id = applied["undo"]["id"].as_str().ok_or("no id")?.to_owned();
    let version = applied["undo"]["version"].as_i64().ok_or("no version")?;
    Ok((id, version))
}

/// Asks the assistant for a new issue called *Fresh* and applies it.
async fn fresh_issue(
    rig: &super::rig::Rig,
    who: &mut Browser,
    site: &Site<'_>,
) -> Result<Value, Failure> {
    ask_and_apply(
        rig,
        who,
        "Create an issue called Fresh",
        "create_issue",
        json!({"project": "ACME", "issue_type": site.kind, "title": "Fresh"}),
    )
    .await
}

#[tokio::test]
async fn ai_05_undo_after_start_timer_names_the_timer_and_throws_it_away() -> TestResult {
    each_database("ai_05_undo_timer_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let set = ada.send("PUT", AI, Some(local_model_on())).await?;
        assert_eq!(set.status, StatusCode::OK, "{}", set.body);
        let key = key_of(&site.create(&mut ada, "Login").await?)?;
        let started = ask_and_apply(
            &rig,
            &mut ada,
            "Start my timer",
            "start_timer",
            json!({"issue": key}),
        )
        .await?;
        assert_eq!(started["undo"]["kind"], json!("timer"), "{started}");
        let (_, version) = made_of(&started)?;
        let thrown = ada
            .send(
                "POST",
                "/api/v1/me/timer/throw-away",
                Some(json!({"version": version})),
            )
            .await?;
        assert!(thrown.status.is_success(), "{}", thrown.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_05_undo_of_the_timer_stops_when_the_timer_changed_since() -> TestResult {
    each_database("ai_05_undo_timer_stale_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let set = ada.send("PUT", AI, Some(local_model_on())).await?;
        assert_eq!(set.status, StatusCode::OK, "{}", set.body);
        let first = key_of(&site.create(&mut ada, "First").await?)?;
        let second = key_of(&site.create(&mut ada, "Second").await?)?;
        let started = ask_and_apply(
            &rig,
            &mut ada,
            "Start my timer",
            "start_timer",
            json!({"issue": first}),
        )
        .await?;
        let (_, version) = made_of(&started)?;
        // The person moves the timer to another issue: it is a new version of the timer.
        rig.clock.advance(5);
        let moved = ada
            .send(
                "POST",
                "/api/v1/me/timer/switch",
                Some(json!({"issue": second, "version": version})),
            )
            .await?;
        assert!(moved.status.is_success(), "{}", moved.body);
        let stale = ada
            .send(
                "POST",
                "/api/v1/me/timer/throw-away",
                Some(json!({"version": version})),
            )
            .await?;
        assert_eq!(stale.status, StatusCode::CONFLICT, "{}", stale.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_05_undo_of_a_new_issue_deletes_it_and_stops_when_it_changed_since() -> TestResult {
    each_database("ai_05_undo_issue_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let set = ada.send("PUT", AI, Some(local_model_on())).await?;
        assert_eq!(set.status, StatusCode::OK, "{}", set.body);
        let (mut bob, _) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        // Untouched since: the way back puts it into the trash.
        let (id, version) = made_of(&fresh_issue(&rig, &mut ada, &site).await?)?;
        let gone = ada
            .send(
                "DELETE",
                &format!("/api/v1/issues/{id}?version={version}"),
                None,
            )
            .await?;
        assert_eq!(gone.status, StatusCode::NO_CONTENT, "{}", gone.body);
        let looked = ada
            .send("GET", &format!("/api/v1/issues/{id}"), None)
            .await?;
        assert_eq!(looked.status, StatusCode::NOT_FOUND, "{}", looked.body);
        // Changed by somebody else since: the way back is refused and the issue stays.
        let (id, version) = made_of(&fresh_issue(&rig, &mut ada, &site).await?)?;
        let theirs = bob
            .send(
                "PATCH",
                &format!("/api/v1/issues/{id}"),
                Some(json!({"version": version, "title": "Bob changed this"})),
            )
            .await?;
        assert_eq!(theirs.status, StatusCode::OK, "{}", theirs.body);
        let refused = ada
            .send(
                "DELETE",
                &format!("/api/v1/issues/{id}?version={version}"),
                None,
            )
            .await?;
        assert_eq!(refused.status, StatusCode::CONFLICT, "{}", refused.body);
        let kept = ada
            .send("GET", &format!("/api/v1/issues/{id}"), None)
            .await?;
        assert_eq!(kept.status, StatusCode::OK, "{}", kept.body);
        assert_eq!(kept.body["issue"]["title"], json!("Bob changed this"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_05_undo_of_a_comment_stops_when_it_changed_since() -> TestResult {
    each_database("ai_05_undo_comment_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let set = ada.send("PUT", AI, Some(local_model_on())).await?;
        assert_eq!(set.status, StatusCode::OK, "{}", set.body);
        let key = key_of(&site.create(&mut ada, "Login").await?)?;
        let made = ask_and_apply(
            &rig,
            &mut ada,
            "Comment that it looks fine",
            "add_comment",
            json!({"issue": key, "text": "Looks fine."}),
        )
        .await?;
        let (id, version) = made_of(&made)?;
        // The comment changes for real (into the trash and out again), so its version moved on.
        let away = ada
            .send("DELETE", &format!("/api/v1/comments/{id}"), None)
            .await?;
        assert_eq!(away.status, StatusCode::NO_CONTENT, "{}", away.body);
        let back = ada
            .send("POST", &format!("/api/v1/comments/{id}/restore"), None)
            .await?;
        assert_eq!(back.status, StatusCode::OK, "{}", back.body);
        let stale = ada
            .send(
                "DELETE",
                &format!("/api/v1/comments/{id}?version={version}"),
                None,
            )
            .await?;
        assert_eq!(stale.status, StatusCode::CONFLICT, "{}", stale.body);
        let version = back.body["version"].as_i64().ok_or("no version")?;
        let gone = ada
            .send(
                "DELETE",
                &format!("/api/v1/comments/{id}?version={version}"),
                None,
            )
            .await?;
        assert_eq!(gone.status, StatusCode::NO_CONTENT, "{}", gone.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_05_undo_of_status_and_of_assigned_to_puts_the_old_value_back_on_the_version_left()
-> TestResult {
    each_database("ai_05_undo_fields_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let set = ada.send("PUT", AI, Some(local_model_on())).await?;
        assert_eq!(set.status, StatusCode::OK, "{}", set.body);
        let (_, bob) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        let key = key_of(&site.create(&mut ada, "Login").await?)?;
        let issue = format!("/api/v1/issues/{key}");
        let before = ada.send("GET", &issue, None).await?;
        let old_status = before.body["issue"]["status"]["key"].clone();
        let old_people = before.body["issue"]["assigned_to"].clone();
        // The status.
        let version = version_of(&mut ada, &key).await?;
        let moved = ask_and_apply(
            &rig,
            &mut ada,
            "Close it",
            "change_status",
            json!({"issue": key, "version": version, "status": "done"}),
        )
        .await?;
        let left = moved["version"].as_i64().ok_or("no version left")?;
        assert_eq!(left, version_of(&mut ada, &key).await?);
        let back = ada
            .send(
                "PATCH",
                &issue,
                Some(json!({"version": left, "status": old_status})),
            )
            .await?;
        assert_eq!(back.status, StatusCode::OK, "{}", back.body);
        let now = ada.send("GET", &issue, None).await?;
        assert_eq!(now.body["issue"]["status"]["key"], old_status);
        // Assigned to.
        let version = version_of(&mut ada, &key).await?;
        let assigned = ask_and_apply(
            &rig,
            &mut ada,
            "Assign it to Bob Builder",
            "update_issue",
            json!({"issue": key, "version": version, "assigned_to": [bob.to_string()]}),
        )
        .await?;
        let left = assigned["version"].as_i64().ok_or("no version left")?;
        assert_eq!(left, version_of(&mut ada, &key).await?);
        let ids: Vec<&str> = old_people
            .as_array()
            .map(|people| people.iter().filter_map(|one| one["id"].as_str()).collect())
            .unwrap_or_default();
        let back = ada
            .send(
                "PATCH",
                &issue,
                Some(json!({"version": left, "assigned_to": ids})),
            )
            .await?;
        assert_eq!(back.status, StatusCode::OK, "{}", back.body);
        let now = ada.send("GET", &issue, None).await?;
        assert_eq!(now.body["issue"]["assigned_to"], old_people);
        Ok(())
    })
    .await
}
