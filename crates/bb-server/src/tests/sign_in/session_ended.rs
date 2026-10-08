//! A `401` for a session that has ended clears the session cookie and what the browser cached
//! (M2-F06, SEC-12), but not for a session that is only waiting for its second step.

use axum::http::StatusCode;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::http::{Browser, CSRF, SESSION, called, totp};
use super::rig::{PASSWORD, Rig};
use super::status::app_with_probe;

/// The answer clears both cookies and tells the browser to forget what it cached.
fn assert_cleared(reply: &super::http::Reply, what: &str) {
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{what}");
    assert_eq!(reply.code(), "auth.not_signed_in", "{what}");
    let cookies = reply.cookies();
    for name in [SESSION, CSRF] {
        assert!(
            cookies
                .iter()
                .any(|cookie| cookie.starts_with(&format!("{name}="))
                    && cookie.contains("Max-Age=0")),
            "{what}: {name} is not cleared in {cookies:?}"
        );
    }
    assert_eq!(reply.header("clear-site-data"), "\"cache\"", "{what}");
    assert_eq!(reply.header("cache-control"), "no-store", "{what}");
}

/// The answer leaves the cookies and the cache alone.
fn assert_untouched(reply: &super::http::Reply, what: &str) {
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{what}");
    assert!(reply.cookies().is_empty(), "{what}: {:?}", reply.cookies());
    assert_eq!(reply.header("clear-site-data"), "", "{what}");
}

#[tokio::test]
async fn m2_f06_a_session_that_ran_out_is_answered_with_the_cookies_cleared() -> TestResult {
    each_database("m2_f06_session_ran_out", async |db| {
        let rig = Rig::start(db).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::signed_in(&app, &rig).await?;
        assert_eq!(
            ada.send("GET", "/api/v1/auth/me", None).await?.status,
            StatusCode::OK
        );
        // Idle for longer than the idle time (30 minutes): the session is over.
        rig.clock.advance(31 * 60);
        let ran_out = ada.send("GET", "/api/v1/auth/me", None).await?;
        assert_cleared(&ran_out, "a session that was idle too long");
        // The browser forgot the cookies, so the next try has none and has nothing to clear.
        let nothing = ada.send("GET", "/api/v1/auth/me", None).await?;
        assert_untouched(&nothing, "no cookie at all");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn m2_f06_a_cookie_that_opens_no_session_is_cleared_too() -> TestResult {
    each_database("m2_f06_cookie_opens_nothing", async |db| {
        let rig = Rig::start(db).await?;
        let app = app_with_probe(&rig, None);
        // A secret that looks like one and belongs to no session (ended, or never was).
        let mut stale = Browser::new(&app, &called());
        stale.set_cookie(SESSION, &"A".repeat(43));
        let reply = stale.send("GET", "/api/v1/auth/me", None).await?;
        assert_cleared(&reply, "a secret that belongs to no session");
        // Something that cannot be a secret at all.
        let mut garbage = Browser::new(&app, &called());
        garbage.set_cookie(SESSION, "garbage");
        let reply = garbage.send("GET", "/api/v1/auth/me", None).await?;
        assert_cleared(&reply, "a cookie that is no secret");
        // A session that ended by signing out elsewhere is the same.
        let mut ada = Browser::signed_in(&app, &rig).await?;
        let mut other = Browser::new(&app, &called());
        other.sign_in("ada@example.test", PASSWORD).await?;
        ada.send("DELETE", "/api/v1/auth/sessions", None).await?;
        let reply = other.send("GET", "/api/v1/auth/me", None).await?;
        assert_cleared(&reply, "a session that was ended from another device");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn m2_f06_a_session_that_waits_for_its_second_step_is_not_cleared() -> TestResult {
    each_database("m2_f06_pending_is_kept", async |db| {
        let rig = Rig::start(db).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::signed_in(&app, &rig).await?;
        ada.send(
            "POST",
            "/api/v1/auth/two-step/code-app",
            Some(json!({"password": PASSWORD})),
        )
        .await?;
        ada.send(
            "POST",
            "/api/v1/auth/two-step/code-app/confirm",
            Some(json!({"code": totp(&rig, "ada@example.test").await?, "password": PASSWORD})),
        )
        .await?;
        rig.clock.advance(30);
        let mut laptop = Browser::new(&app, &called());
        laptop.sign_in("ada@example.test", PASSWORD).await?;
        // The password step is not a session yet: asking who is signed in is turned away, and
        // the cookie is kept, or the person could never finish signing in.
        let asked = laptop.send("GET", "/api/v1/auth/me", None).await?;
        assert_untouched(
            &asked,
            "the pending session on a route for signed-in people",
        );
        let finished = laptop
            .send(
                "POST",
                "/api/v1/auth/second-step",
                Some(json!({"code": totp(&rig, "ada@example.test").await?})),
            )
            .await?;
        assert_eq!(finished.status, StatusCode::OK);
        assert_eq!(
            laptop.send("GET", "/api/v1/auth/me", None).await?.status,
            StatusCode::OK
        );
        // The other way round: a finished session on the route of the second step is turned away
        // and kept.
        let wrong_place = laptop
            .send(
                "POST",
                "/api/v1/auth/second-step",
                Some(json!({"code": "123456"})),
            )
            .await?;
        assert_untouched(
            &wrong_place,
            "a signed-in session on the route of the second step",
        );
        assert_eq!(
            laptop.send("GET", "/api/v1/auth/me", None).await?.status,
            StatusCode::OK
        );
        Ok(())
    })
    .await
}
