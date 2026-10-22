//! The exit review of M4 (WP-12, M2), probing for people by their handle (ADR-0033 §6, *Who sees
//! people*): the rule of the days is judged before anything is said about a handle, and a person
//! who keeps asking for handles that are taken has to wait. Every flow runs on SQLite and
//! PostgreSQL.

use axum::http::StatusCode;
use bb_api::DEFAULT_SIGN_IN_FAILURES_PER_MINUTE;
use bb_app::access::StartingRole;
use bb_app::notifications::TAKEN_ANSWERS_PER_DAY;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::http::{Browser, rig_and_app, rig_and_app_with};
use super::issues_site::Site;
use super::rig::Tries;

const HANDLE: &str = "/api/v1/me/handle";

#[tokio::test]
async fn sec_m2_a_person_who_changed_lately_gets_the_same_answer_for_every_handle() -> TestResult {
    each_database("sec_m2_handle_days_first", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        site.person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let mut bob = Browser::signed_in_as(&app, "bob@example.test").await?;
        let first = bob
            .send("PUT", HANDLE, Some(json!({"handle": "bobby"})))
            .await?;
        assert_eq!(first.status, StatusCode::OK, "{}", first.body);
        // A handle that is taken, one that is free and the handle of a role: one answer.
        for guess in ["bob.baker", "nobody.here", "administrator"] {
            let reply = bob
                .send("PUT", HANDLE, Some(json!({"handle": guess})))
                .await?;
            assert_eq!(
                (reply.status, reply.code()),
                (StatusCode::TOO_MANY_REQUESTS, "handle.changed_lately"),
                "{guess}"
            );
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_m2_a_person_who_keeps_asking_for_taken_handles_has_to_wait() -> TestResult {
    each_database("sec_m2_handle_limit", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        site.person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        site.person_called(StartingRole::TeamMember, "eve", "Eve Evans")
            .await?;
        let mut eve = Browser::signed_in_as(&app, "eve@example.test").await?;
        for number in 0..DEFAULT_SIGN_IN_FAILURES_PER_MINUTE {
            let reply = eve
                .send("PUT", HANDLE, Some(json!({"handle": "bob.baker"})))
                .await?;
            assert_eq!(
                (reply.status, reply.code()),
                (StatusCode::CONFLICT, "name.taken"),
                "try {number}"
            );
        }
        let over = eve
            .send("PUT", HANDLE, Some(json!({"handle": "bob.baker"})))
            .await?;
        assert_eq!(over.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(over.code(), "rate_limit.exceeded");
        // Whatever she asks for now waits, a free handle too; another person is not held up.
        let free = eve
            .send("PUT", HANDLE, Some(json!({"handle": "eve.free"})))
            .await?;
        assert_eq!(free.code(), "rate_limit.exceeded");
        let mut bob = Browser::signed_in_as(&app, "bob@example.test").await?;
        let own = bob
            .send("PUT", HANDLE, Some(json!({"handle": "bobby"})))
            .await?;
        assert_eq!(own.status, StatusCode::OK, "{}", own.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_m2_the_answers_taken_run_out_for_the_day_and_then_every_handle_gets_one_answer()
-> TestResult {
    each_database("sec_m2_handle_day", async |db| {
        // The tries of a minute are out of the way: the allowance of the day is what stops here.
        let (rig, app) = rig_and_app_with(db, Tries::Roomy).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        site.person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        site.person_called(StartingRole::TeamMember, "eve", "Eve Evans")
            .await?;
        let mut eve = Browser::signed_in_as(&app, "eve@example.test").await?;
        for number in 0..TAKEN_ANSWERS_PER_DAY {
            let reply = eve
                .send("PUT", HANDLE, Some(json!({"handle": "bob.baker"})))
                .await?;
            assert_eq!(
                (reply.status, reply.code()),
                (StatusCode::CONFLICT, "name.taken"),
                "try {number}"
            );
        }
        // The day is used up: a taken handle, a free one and the handle of a role get one answer.
        for guess in ["bob.baker", "eve.free", "administrator"] {
            let reply = eve
                .send("PUT", HANDLE, Some(json!({"handle": guess})))
                .await?;
            assert_eq!(
                (reply.status, reply.code()),
                (StatusCode::TOO_MANY_REQUESTS, "rate_limit.exceeded"),
                "{guess}"
            );
        }
        // Another person has a day of their own.
        let mut bob = Browser::signed_in_as(&app, "bob@example.test").await?;
        let own = bob
            .send("PUT", HANDLE, Some(json!({"handle": "bobby"})))
            .await?;
        assert_eq!(own.status, StatusCode::OK, "{}", own.body);
        Ok(())
    })
    .await
}
