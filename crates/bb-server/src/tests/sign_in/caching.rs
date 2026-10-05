//! Entity tags, `304` and what is never stored (PERF-03), over every route of the API description.
//!
//! The rules for every single request (what may be kept, what never is) are also checked on all
//! requests of all tests that use a `Browser` (`http::assert_cache_rules`).

use axum::http::StatusCode;
use bb_api::openapi;
use bb_testing::each_database;
use serde_json::json;

use super::http::{Browser, called, setup_body, totp};
use super::rig::{Outcome, PASSWORD, Rig};
use super::status::app_with_probe;

/// The addresses of every `GET` route in the API description.
fn get_routes() -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let document = serde_json::to_value(openapi::document())?;
    let mut routes = Vec::new();
    for (path, item) in document["paths"].as_object().ok_or("no paths")? {
        if item.get("get").is_some() {
            routes.push(path.clone());
        }
    }
    Ok(routes)
}

#[tokio::test]
async fn perf_03_every_get_answers_an_etag_and_304_on_a_match() -> Outcome {
    each_database("perf_03_http_etags", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;

        let routes = get_routes()?;
        assert!(routes.len() >= 8, "{routes:?}");
        for path in &routes {
            // A route with a part to fill in needs an example address here; fail loudly so a
            // new route is never left out of this test.
            assert!(
                !path.contains('{'),
                "{path} has a part to fill in: add an example address to this test"
            );
            ada.clear_headers();
            let first = ada.send("GET", path, None).await?;
            assert_eq!(first.status, StatusCode::OK, "{path}");
            let tag = first.header("etag").to_owned();
            assert!(tag.starts_with("W/\""), "{path}: {tag}");
            assert_eq!(first.header("cache-control"), "private, no-cache", "{path}");

            // The tag it gave: no body, the same tag.
            ada.set_header("if-none-match", &tag);
            let again = ada.send("GET", path, None).await?;
            if again.status == StatusCode::NOT_MODIFIED {
                assert_eq!(again.header("etag"), tag, "{path}");
                assert!(again.body.is_null(), "{path}");
            } else {
                // Only an answer that holds a moving number (the uptime on the status page) may
                // have changed between the two requests; then it must have a new tag.
                assert_eq!(again.status, StatusCode::OK, "{path}");
                assert_ne!(again.header("etag"), tag, "{path}");
            }

            // Some other tag: the answer in full.
            ada.set_header("if-none-match", "W/\"another\"");
            let other = ada.send("GET", path, None).await?;
            assert_eq!(other.status, StatusCode::OK, "{path}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_03_a_read_asked_about_again_is_not_modified_when_nothing_changed() -> Outcome {
    each_database("perf_03_http_not_modified", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let path = "/api/v1/auth/sign-up-policy";
        let first = ada.send("GET", path, None).await?;
        let tag = first.header("etag").to_owned();
        ada.set_header("if-none-match", &tag);
        let same = ada.send("GET", path, None).await?;
        assert_eq!(same.status, StatusCode::NOT_MODIFIED);
        assert!(same.body.is_null());

        // The answer changes: the old tag no longer matches, and the new answer has a new one.
        let changed = ada
            .send(
                "PUT",
                path,
                Some(json!({"enabled": true, "domains": ["example.test"]})),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::OK);
        let after = ada.send("GET", path, None).await?;
        assert_eq!(after.status, StatusCode::OK);
        assert_ne!(after.header("etag"), tag);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_03_two_people_never_share_a_tag_for_different_answers() -> Outcome {
    each_database("perf_03_http_tags_per_person", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let invitation = ada
            .send(
                "POST",
                "/api/v1/auth/invitations",
                Some(json!({"email": "bob@example.test"})),
            )
            .await?;
        let secret = invitation.body["link_path"]
            .as_str()
            .and_then(|path| path.split_once('#'))
            .map(|(_, secret)| secret.to_owned())
            .ok_or("no link")?;
        let mut bob = Browser::new(&app, &called());
        bob.send(
            "POST",
            "/api/v1/auth/invitation/accept",
            Some(json!({"secret": secret, "display_name": "Bob", "password": PASSWORD})),
        )
        .await?;
        // The same address, two people, two answers: two tags. Bob's tag does not make Ada's
        // answer come back as "not modified".
        let ada_me = ada.send("GET", "/api/v1/auth/me", None).await?;
        let bob_me = bob.send("GET", "/api/v1/auth/me", None).await?;
        assert_ne!(ada_me.header("etag"), bob_me.header("etag"));
        ada.set_header("if-none-match", bob_me.header("etag"));
        let reply = ada.send("GET", "/api/v1/auth/me", None).await?;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.body["email"], "ada@example.test");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_03_answers_with_a_secret_are_not_stored_or_compressed() -> Outcome {
    each_database("perf_03_http_secret_answers", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::new(&app, &called());
        // The browser accepts every kind of compression it knows.
        ada.set_header("accept-encoding", "br, gzip, deflate, zstd");

        let secret_answers = [
            ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
                .await?,
            ada.send(
                "POST",
                "/api/v1/auth/invitations",
                Some(json!({"email": "bob@example.test"})),
            )
            .await?,
            ada.send(
                "POST",
                "/api/v1/auth/two-step/code-app",
                Some(json!({"password": PASSWORD})),
            )
            .await?,
            ada.send(
                "POST",
                "/api/v1/auth/two-step/code-app/confirm",
                Some(json!({
                    "code": totp(&rig, "ada@example.test").await?,
                    "password": PASSWORD,
                })),
            )
            .await?,
            ada.send(
                "POST",
                "/api/v1/auth/two-step/backup-codes",
                Some(json!({"password": PASSWORD})),
            )
            .await?,
        ];
        for reply in &secret_answers {
            assert_eq!(reply.status, StatusCode::OK);
            assert_eq!(reply.header("cache-control"), "no-store");
            assert_eq!(reply.header("etag"), "");
            assert_eq!(reply.header("content-encoding"), "");
        }
        Ok(())
    })
    .await
}
