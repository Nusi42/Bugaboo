//! Entity tags, `304` and what is never stored (PERF-03), over every route of the API description.
//!
//! The rules for every single request (what may be kept, what never is) are also checked on all
//! requests of all tests that use a `Browser` (`http::assert_cache_rules`).

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use bb_api::openapi;
use bb_app::access::Permission;
use bb_app::roles::MakeRole;
use bb_testing::each_database;
use serde_json::json;
use tower::ServiceExt as _;

use super::http::{Browser, Failure, called, never_stored_reads, setup_body, totp};
use super::rig::{Outcome, PASSWORD, Rig};
use super::status::app_with_probe;

/// The addresses of every `GET` route in the API description.
fn get_routes() -> Result<Vec<String>, Failure> {
    let document = serde_json::to_value(openapi::document())?;
    let mut routes = Vec::new();
    for (path, item) in document["paths"].as_object().ok_or("no paths")? {
        if item.get("get").is_some() {
            routes.push(path.clone());
        }
    }
    Ok(routes)
}

/// A second person (not an administrator): invited by `ada`, signed in.
pub(super) async fn invited_person(
    ada: &mut Browser,
    app: &Router,
    email: &str,
) -> Result<Browser, Failure> {
    let invitation = ada
        .send(
            "POST",
            "/api/v1/auth/invitations",
            Some(json!({ "email": email })),
        )
        .await?;
    let secret = invitation.body["link_path"]
        .as_str()
        .and_then(|path| path.split_once('#'))
        .map(|(_, secret)| secret.to_owned())
        .ok_or("no link")?;
    let mut person = Browser::new(app, &called());
    let accepted = person
        .send(
            "POST",
            "/api/v1/auth/invitation/accept",
            Some(json!({"secret": secret, "display_name": "Bob", "password": PASSWORD})),
        )
        .await?;
    assert_eq!(accepted.status, StatusCode::OK);
    Ok(person)
}

#[tokio::test]
async fn perf_03_every_get_answers_an_etag_and_304_on_a_match() -> Outcome {
    each_database("perf_03_http_etags", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;

        // A role and a project for the routes that name one.
        let admin = rig
            .sign_in("ada@example.test", PASSWORD)
            .await?
            .session_token;
        let role = rig
            .api()
            .make_role(
                Rig::info(),
                MakeRole {
                    session_token: admin,
                    name: "Checker".to_owned(),
                    permissions: vec![Permission::SeeProjects],
                    two_step: false,
                },
            )
            .await?
            .into_value();
        let made = ada
            .send(
                "POST",
                "/api/v1/projects",
                Some(json!({"short_name": "EXAMPLE", "name": "Example"})),
            )
            .await?;
        assert_eq!(made.status, StatusCode::OK);
        // A group and the person themselves, for the routes of people and groups.
        let group = ada
            .send("POST", "/api/v1/groups", Some(json!({"name": "Checkers"})))
            .await?;
        assert_eq!(group.status, StatusCode::OK);
        let (group_id, project_id, person_id) = (
            group.body["id"].as_str().ok_or("no group")?.to_owned(),
            made.body["id"].as_str().ok_or("no project")?.to_owned(),
            ada.send("GET", "/api/v1/auth/me", None).await?.body["id"]
                .as_str()
                .ok_or("no person")?
                .to_owned(),
        );
        let routes = get_routes()?;
        assert!(routes.len() >= 9, "{routes:?}");
        let (mut kept, mut never_stored) = (0, 0);
        for template in &routes {
            // A route with a part to fill in needs an example address here; fail loudly so a
            // new route is never left out of this test.
            // The routes of *Who has which role* take the ID of a project, not a short name.
            let project = if template.contains("assignment") {
                &project_id
            } else {
                "EXAMPLE"
            };
            let filled = template
                .replace("{role_id}", &role.id.to_string())
                .replace("{group_id}", &group_id)
                .replace("{person_id}", &person_id)
                .replace("{project}", project);
            let path = &filled;
            assert!(
                !path.contains('{'),
                "{path} has a part to fill in: add an example address to this test"
            );
            ada.clear_headers();
            let first = ada.send("GET", path, None).await?;
            assert_eq!(first.status, StatusCode::OK, "{path}");

            if never_stored_reads().contains(template) {
                // Personal data: no tag to ask with, and a tag or `*` changes nothing.
                never_stored += 1;
                assert_eq!(first.header("cache-control"), "no-store", "{path}");
                assert_eq!(first.header("etag"), "", "{path}");
                ada.set_header("if-none-match", "*");
                let asked = ada.send("GET", path, None).await?;
                assert_eq!(asked.status, StatusCode::OK, "{path}");
                continue;
            }

            kept += 1;
            let tag = first.header("etag").to_owned();
            assert!(tag.starts_with("W/\""), "{path}: {tag}");
            assert_eq!(first.header("cache-control"), "private, no-cache", "{path}");

            // The tag it gave: no body, the same tag.
            ada.set_header("if-none-match", &tag);
            let again = ada.send("GET", path, None).await?;
            assert_eq!(again.status, StatusCode::NOT_MODIFIED, "{path}");
            assert_eq!(again.header("etag"), tag, "{path}");
            assert!(again.body.is_null(), "{path}");

            // Some other tag: the answer in full.
            ada.set_header("if-none-match", "W/\"another\"");
            let other = ada.send("GET", path, None).await?;
            assert_eq!(other.status, StatusCode::OK, "{path}");
        }
        assert!(
            kept >= 4 && never_stored >= 5,
            "{kept} kept, {never_stored} never stored"
        );
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
async fn perf_03_answers_with_personal_data_are_never_stored_for_anybody() -> Outcome {
    each_database("perf_03_http_personal_data", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let mut bob = invited_person(&mut ada, &app, "bob@example.test").await?;
        for person in [&mut ada, &mut bob] {
            for path in [
                "/api/v1/auth/me",
                "/api/v1/auth/sessions",
                "/api/v1/auth/two-step",
            ] {
                let reply = person.send("GET", path, None).await?;
                assert_eq!(reply.status, StatusCode::OK, "{path}");
                assert_eq!(reply.header("cache-control"), "no-store", "{path}");
                assert_eq!(reply.header("etag"), "", "{path}");
            }
        }
        // Different people, different answers, and neither is kept for the next person at the
        // same computer.
        let ada_me = ada.send("GET", "/api/v1/auth/me", None).await?;
        let bob_me = bob.send("GET", "/api/v1/auth/me", None).await?;
        assert_ne!(ada_me.body["email"], bob_me.body["email"]);
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
        // An answer that sets a cookie is never compressed either (signing in has no secret
        // field, but the session it starts is one).
        let mut again = Browser::new(&app, &called());
        again.set_header("accept-encoding", "br, gzip, deflate, zstd");
        let signed_in = again
            .send(
                "POST",
                "/api/v1/auth/sign-in",
                Some(json!({"email": "ada@example.test", "password": PASSWORD})),
            )
            .await?;
        assert!(!signed_in.cookies().is_empty());
        assert_eq!(signed_in.header("content-encoding"), "");
        // The control: this very server does compress a big answer that holds no secret (the API
        // description), so the plain answers above are plain by the rule and not by chance.
        let request = Request::get("/api/v1/openapi.json")
            .header("accept-encoding", "br, gzip")
            .body(Body::empty())?;
        let big = app.clone().oneshot(request).await?;
        assert_eq!(big.headers()["content-encoding"], "br");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_signing_out_clears_what_the_browser_cached() -> Outcome {
    each_database("sec_12_http_clear_cache", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::new(&app, &called());
        let signed_in = ada
            .send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        // Signing in does not clear anything.
        assert_eq!(signed_in.header("clear-site-data"), "");

        // Signing out ends the session, forgets the cookies and clears the cache.
        let out = ada.send("POST", "/api/v1/auth/sign-out", None).await?;
        assert_eq!(out.status, StatusCode::NO_CONTENT);
        assert_eq!(out.header("clear-site-data"), "\"cache\"");
        assert_eq!(out.header("cache-control"), "no-store");

        // The same for signing out everywhere.
        let mut again = Browser::new(&app, &called());
        again
            .send(
                "POST",
                "/api/v1/auth/sign-in",
                Some(json!({"email": "ada@example.test", "password": PASSWORD})),
            )
            .await?;
        let everywhere = again.send("DELETE", "/api/v1/auth/sessions", None).await?;
        assert_eq!(everywhere.status, StatusCode::NO_CONTENT);
        assert_eq!(everywhere.header("clear-site-data"), "\"cache\"");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_ending_the_own_session_is_signing_out_but_ending_another_is_not() -> Outcome {
    each_database("sec_12_http_end_own_session", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = app_with_probe(&rig, None);
        let mut laptop = Browser::new(&app, &called());
        laptop
            .send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let mut phone = Browser::new(&app, &called());
        phone
            .send(
                "POST",
                "/api/v1/auth/sign-in",
                Some(json!({"email": "ada@example.test", "password": PASSWORD})),
            )
            .await?;
        let listed = laptop.send("GET", "/api/v1/auth/sessions", None).await?;
        let sessions = listed.body["sessions"].as_array().ok_or("no sessions")?;
        let id_of = |current: bool| {
            sessions
                .iter()
                .find(|session| session["current"] == current)
                .and_then(|session| session["id"].as_str())
                .map(str::to_owned)
        };
        let own = id_of(true).ok_or("no current session")?;
        let other = id_of(false).ok_or("no other session")?;

        // Another session: ended, and this browser keeps its cookies and its cache.
        let ended = laptop
            .send("DELETE", &format!("/api/v1/auth/sessions/{other}"), None)
            .await?;
        assert_eq!(ended.status, StatusCode::NO_CONTENT);
        assert_eq!(ended.header("clear-site-data"), "");
        assert_eq!(
            laptop.send("GET", "/api/v1/auth/me", None).await?.status,
            StatusCode::OK
        );

        // The own session: the same as signing out, cookies and cache.
        let own_end = laptop
            .send("DELETE", &format!("/api/v1/auth/sessions/{own}"), None)
            .await?;
        assert_eq!(own_end.status, StatusCode::NO_CONTENT);
        assert_eq!(own_end.header("clear-site-data"), "\"cache\"");
        let after = laptop.send("GET", "/api/v1/auth/me", None).await?;
        assert_eq!(after.status, StatusCode::UNAUTHORIZED);
        Ok(())
    })
    .await
}
