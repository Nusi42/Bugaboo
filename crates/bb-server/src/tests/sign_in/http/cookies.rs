//! The cookies of a session, the token against forged requests (SEC-12) and the limits of failed sign-ins (SEC-14).

use super::*;

#[tokio::test]
async fn sec_12_signing_in_sets_the_session_cookie_and_the_csrf_cookie() -> TestResult {
    each_database("sec_12_http_cookies", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let mut browser = Browser::new(&app, &called());
        let reply = browser.set_up(&rig).await?;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.body["person"]["email"], ADA_EMAIL);
        assert!(
            reply.body["person"].get("administrator").is_none(),
            "the flag is gone"
        );
        assert_eq!(reply.body["second_step"], Value::Null);
        assert_eq!(reply.header("cache-control"), "no-store");
        let cookies = reply.cookies();
        assert_eq!(cookies.len(), 2);
        let session = cookies
            .iter()
            .find(|cookie| cookie.starts_with(SESSION))
            .ok_or("no session cookie")?;
        let csrf = cookies
            .iter()
            .find(|cookie| cookie.starts_with(CSRF))
            .ok_or("no csrf cookie")?;
        // The session cookie is shut to scripts and lives as long as a session may (24 hours).
        assert!(session.contains("; Path=/") && session.contains("; Secure"));
        assert!(session.contains("; HttpOnly") && session.contains("; SameSite=Lax"));
        assert!(session.contains("Max-Age=86400"), "{session}");
        // The token cookie is read by the app, so scripts may see it; it is strict.
        assert!(csrf.contains("; Secure") && csrf.contains("; SameSite=Strict"));
        assert!(!csrf.contains("HttpOnly"));
        // Neither value is the other, and the session secret is 256 bits.
        assert_ne!(browser.jar[SESSION], browser.jar[CSRF]);
        assert_eq!(browser.jar[SESSION].len(), 43);
        // Both cookies come again with the next sign-in, new.
        let mut second = Browser::new(&app, &called());
        let again = second.sign_in("ADA@example.test", PASSWORD).await?;
        assert_eq!(again.status, StatusCode::OK);
        assert_ne!(second.jar[SESSION], browser.jar[SESSION]);
        assert_ne!(second.jar[CSRF], browser.jar[CSRF]);
        // The cookie is bound to the session and the browser asks for who it is with it.
        let me = browser.send("GET", "/api/v1/auth/me", None).await?;
        assert_eq!(me.status, StatusCode::OK);
        assert_eq!(me.body["email"], ADA_EMAIL);
        assert_eq!(me.header("cache-control"), "no-store");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_a_change_needs_the_token_of_its_own_session() -> TestResult {
    each_database("sec_12_http_csrf", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let mut browser = Browser::signed_in(&app, &rig).await?;
        let mut other = Browser::new(&app, &called());
        other.sign_in(ADA_EMAIL, PASSWORD).await?;
        let others_token = other.jar[CSRF].clone();
        let own_token = browser.jar[CSRF].clone();
        let origin = "http://bugaboo.test";

        // Reads need no token. Changes without the right one are refused, whatever the reason.
        for (what, token) in [
            ("no token", Token::Nothing),
            ("another session's token", Token::Is(&others_token)),
            ("a part of the token", Token::Is(&own_token[..10])),
            ("an empty token", Token::Is("")),
        ] {
            let reply = browser
                .send_as("POST", "/api/v1/auth/sign-out", None, token, origin)
                .await?;
            assert_eq!(reply.status, StatusCode::FORBIDDEN, "{what}");
            assert_eq!(reply.code(), "request.wrong_token", "{what}");
        }
        assert_eq!(
            browser.send("GET", "/api/v1/auth/me", None).await?.status,
            StatusCode::OK
        );

        // The origin is judged first.
        let reply = browser
            .send_as(
                "POST",
                "/api/v1/auth/sign-out",
                None,
                Token::FromCookie,
                "https://evil.test",
            )
            .await?;
        assert_eq!(reply.code(), "request.wrong_origin");
        // Without a session cookie there is nobody to protect: not signed in.
        let mut stranger = Browser::new(&app, &called());
        let reply = stranger
            .send("POST", "/api/v1/auth/two-step/code-app", None)
            .await?;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        assert_eq!(reply.code(), "auth.not_signed_in");

        // The right token does it, and both cookies are cleared.
        let reply = browser.send("POST", "/api/v1/auth/sign-out", None).await?;
        assert_eq!(reply.status, StatusCode::NO_CONTENT);
        let cleared = reply.cookies();
        assert_eq!(cleared.len(), 2);
        assert!(cleared.iter().all(|cookie| cookie.contains("Max-Age=0")));
        assert!(browser.jar.is_empty());
        let reply = browser.send("GET", "/api/v1/auth/me", None).await?;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        // The other session is still there.
        assert_eq!(
            other.send("GET", "/api/v1/auth/me", None).await?.status,
            StatusCode::OK
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_every_route_that_changes_something_checks_the_token_except_signing_in_and_up()
-> TestResult {
    each_database("sec_12_http_every_route", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let mut browser = Browser::signed_in(&app, &rig).await?;
        let mut checked = 0;
        for operation in operations().iter().filter(|operation| operation.changes()) {
            let reply = browser
                .send_as(
                    operation.method.as_str(),
                    &example_path(&operation.path),
                    Some(json!({})),
                    Token::Nothing,
                    "http://bugaboo.test",
                )
                .await?;
            let (method, path) = (&operation.method, &operation.path);
            // The routes people use before they have a session: no token can exist yet.
            if operation.needs == Declared::Public {
                assert_ne!(reply.code(), "request.wrong_token", "{method} {path}");
            } else {
                assert_eq!(reply.status, StatusCode::FORBIDDEN, "{method} {path}");
                assert_eq!(reply.code(), "request.wrong_token", "{method} {path}");
                checked += 1;
            }
        }
        assert!(checked >= 15, "{checked}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_failed_sign_ins_are_turned_away_with_a_wait_and_the_answers_look_alike()
-> TestResult {
    each_database("sec_14_http_limit", async |db| {
        let (rig, app) = rig_and_app_with(db, Tries::Few).await?;
        let _browser = Browser::signed_in(&app, &rig).await?;
        // Every try comes from another network, so only the account is limited here.
        let fresh = || Browser::new(&app, &called());
        let wrong = fresh().sign_in(ADA_EMAIL, "Wrong-Password-1").await?;
        let unknown = fresh()
            .sign_in("nobody@example.test", "Wrong-Password-1")
            .await?;
        // Same status, same code, same message: nothing says whether the account exists.
        assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
        assert_eq!(unknown.status, wrong.status);
        for field in ["type", "title", "status", "code", "message_key"] {
            assert_eq!(wrong.body[field], unknown.body[field], "{field}");
        }
        assert_eq!(wrong.code(), "auth.wrong_credentials");

        // Ada's account has two more tries; then even her right password waits.
        for _ in 0..2 {
            let reply = fresh().sign_in(ADA_EMAIL, "Wrong-Password-1").await?;
            assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        }
        let reply = fresh().sign_in(ADA_EMAIL, PASSWORD).await?;
        assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(reply.code(), "rate_limit.exceeded");
        let wait: u64 = reply.header("retry-after").parse()?;
        assert!((1..=60).contains(&wait), "{wait}");
        assert_eq!(reply.header("content-type"), "application/problem+json");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_failures_of_one_network_are_limited_whatever_the_account() -> TestResult {
    each_database("sec_14_http_network_failures", async |db| {
        let (rig, app) = rig_and_app_with(db, Tries::Few).await?;
        Browser::new(&app, &called()).set_up(&rig).await?;
        let mut guesser = Browser::new(&app, &called());
        // Three wrong tries at three different accounts use up the network.
        for number in 0..3 {
            let reply = guesser
                .sign_in(&format!("person{number}@example.test"), "Wrong-Password-1")
                .await?;
            assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        }
        // Now even a right password from this network waits ...
        let reply = guesser.sign_in(ADA_EMAIL, PASSWORD).await?;
        assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
        // ... and another network is not affected.
        let mut other = Browser::new(&app, &called());
        let reply = other.sign_in(ADA_EMAIL, PASSWORD).await?;
        assert_eq!(reply.status, StatusCode::OK);
        // Many successes from one network never count: they give their try back.
        for _ in 0..5 {
            let reply = other.sign_in(ADA_EMAIL, PASSWORD).await?;
            assert_eq!(reply.status, StatusCode::OK);
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_one_network_is_limited_in_its_tries_whatever_the_account() -> TestResult {
    each_database("sec_14_http_network_limit", async |db| {
        let rig = Rig::start_with(db, Tries::Unreached).await?;
        let app = router(options(
            &rig,
            LimitsConfig {
                requests_per_minute: 1000,
                sign_in_attempts_per_minute: 4,
                ..LimitsConfig::default()
            },
        ));
        let mut browser = Browser::new(&app, &called());
        let mut statuses = Vec::new();
        for number in 0..6 {
            let reply = browser
                .sign_in(&format!("person{number}@example.test"), PASSWORD)
                .await?;
            statuses.push(reply.status);
        }
        assert_eq!(
            statuses,
            [
                StatusCode::UNAUTHORIZED,
                StatusCode::UNAUTHORIZED,
                StatusCode::UNAUTHORIZED,
                StatusCode::UNAUTHORIZED,
                StatusCode::TOO_MANY_REQUESTS,
                StatusCode::TOO_MANY_REQUESTS
            ]
        );
        // Reading who is signed in is not a try.
        assert_eq!(
            browser
                .send("GET", "/api/v1/auth/status", None)
                .await?
                .status,
            StatusCode::OK
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_01_without_a_database_the_sign_in_routes_say_so_and_nothing_else_breaks() -> TestResult
{
    each_database("auth_01_http_unavailable", async |_db| {
        let app = router(Options::for_tests());
        let mut browser = Browser::new(&app, &called());
        let reply = browser.send("GET", "/api/v1/auth/status", None).await?;
        assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(reply.code(), "server.unavailable");
        assert_eq!(
            browser.send("GET", "/api/v1/status", None).await?.status,
            StatusCode::OK
        );
        Ok(())
    })
    .await
}
