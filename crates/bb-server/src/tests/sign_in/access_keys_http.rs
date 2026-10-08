//! Access keys over HTTP (AUTH-07, SEC-06, SEC-14): what a script can send and what a page of
//! another website cannot, the limit of one key, the same answer for every key that does not work,
//! and the routes of the owner's own sign-in data that no key opens.

use axum::http::{Method, StatusCode, header};
use bb_api::LimitsConfig;
use bb_api::openapi::operations;
use bb_api::permissions::Declared;
use bb_app::access::Permission;
use bb_app::auth::{DeleteAccessKey, MakeAccessKey};
use bb_auth::AccessKeySecret;
use bb_core::access_key::KeyChoice;
use bb_core::sign_in::SecurityAction;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, called, example_path};
use super::policy_rig::Org;
use super::rig::{PASSWORD, Rig};
use super::status::{app_with_probe, app_with_probe_and_limits};

/// A route that a key may use (it needs *See status*, a permission keys may hold) ...
const KEY_ROUTE: &str = "/api/v1/admin/status";
/// ... and one that no key may use (a change of organization settings): a request that reaches it
/// has passed the origin and token checks, and is turned away for what it asks.
const CHANGE_ROUTE: &str = "/api/v1/auth/sign-up-policy";

/// The same answer, whatever made it: the status, the code and the message, not the request ID.
fn alike(reply: &super::http::Reply) -> (u16, String, String) {
    (
        reply.status.as_u16(),
        reply.code().to_owned(),
        reply.body["message_key"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
    )
}

#[tokio::test]
async fn auth_07_a_script_with_a_key_needs_no_origin_no_token_and_no_cookie() -> TestResult {
    each_database("auth_07_script_http", async |db| {
        let rig = Rig::start(db, 50).await?;
        let org = Org::start(&rig).await?;
        let key = super::access_keys::key_of(&rig, &org.admin, KeyChoice::Everything).await?;
        let app = app_with_probe(&rig, None);
        let mut script = Browser::new(&app, &called());
        // A change from a program: only the header. A page needs an origin and a token. The key
        // gets past both and is turned away for what it asks: no key changes the settings of the
        // organization.
        let made = script
            .script(
                "PUT",
                CHANGE_ROUTE,
                Some(json!({"enabled": false, "domains": []})),
                &key,
            )
            .await?;
        assert_eq!(made.status, StatusCode::FORBIDDEN, "{:?}", made.body);
        assert_eq!(made.code(), "access.forbidden");
        assert!(made.cookies().is_empty(), "a key never sets a cookie");
        let listed = script.script("GET", KEY_ROUTE, None, &key).await?;
        assert_eq!(listed.status, StatusCode::OK);
        assert!(listed.body.get("version").is_some());
        // Without a key the same request from a program is a change that comes from nowhere
        // (no key, so no way around the origin check).
        let nothing = script
            .finish(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri(CHANGE_ROUTE)
                    .header(header::HOST, "bugaboo.test"),
                "PUT",
                CHANGE_ROUTE,
                Some(json!({"enabled": false, "domains": []})),
            )
            .await?;
        assert_eq!(nothing.code(), "request.wrong_origin");
        // A browser with a session: its cookie rules apply even when it sends a key too. No
        // token, no change.
        let mut page = Browser::new(&app, &called());
        page.send(
            "POST",
            "/api/v1/auth/sign-in",
            Some(super::http::sign_in_body("ada@example.test", PASSWORD)),
        )
        .await?;
        page.set_header("authorization", &format!("Bearer {key}"));
        let no_token = page
            .send_without_token(
                "PUT",
                CHANGE_ROUTE,
                json!({"enabled": false, "domains": []}),
            )
            .await?;
        assert_eq!(no_token.code(), "request.wrong_token");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_a_page_of_another_website_cannot_send_a_key() -> TestResult {
    each_database("sec_06_no_cors_for_keys", async |db| {
        let rig = Rig::start(db, 50).await?;
        let org = Org::start(&rig).await?;
        let key = super::access_keys::key_of(&rig, &org.admin, KeyChoice::Everything).await?;
        let app = app_with_probe(&rig, None);
        let mut page = Browser::new(&app, &called());
        // A script on another website can only send the header after a preflight, and a preflight
        // is answered without a single CORS header, so the browser never sends the request.
        let preflight = page
            .finish(
                axum::http::Request::builder()
                    .method("OPTIONS")
                    .uri(CHANGE_ROUTE)
                    .header(header::HOST, "bugaboo.test")
                    .header(header::ORIGIN, "https://evil.test")
                    .header("access-control-request-method", "POST")
                    .header("access-control-request-headers", "authorization"),
                "OPTIONS",
                CHANGE_ROUTE,
                None,
            )
            .await?;
        for name in reply_headers(&preflight) {
            assert!(!name.starts_with("access-control-"), "{name}");
        }
        // Something that is not a key in the header does not make a request a script's: the
        // origin check judges it as before.
        let not_a_key = page
            .finish(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri(CHANGE_ROUTE)
                    .header(header::HOST, "bugaboo.test")
                    .header(header::AUTHORIZATION, "Bearer not-a-key")
                    .header(header::ORIGIN, "https://evil.test"),
                "PUT",
                CHANGE_ROUTE,
                Some(json!({"enabled": false, "domains": []})),
            )
            .await?;
        assert_eq!(not_a_key.code(), "request.wrong_origin");
        // And a different scheme is not a key either.
        let basic = page
            .finish(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri(CHANGE_ROUTE)
                    .header(header::HOST, "bugaboo.test")
                    .header(header::AUTHORIZATION, format!("Basic {key}")),
                "PUT",
                CHANGE_ROUTE,
                Some(json!({"enabled": false, "domains": []})),
            )
            .await?;
        assert_eq!(basic.code(), "request.wrong_origin");
        Ok(())
    })
    .await
}

/// The names of the headers of an answer.
fn reply_headers(reply: &super::http::Reply) -> Vec<String> {
    reply
        .headers
        .keys()
        .map(|name| name.as_str().to_owned())
        .collect()
}

#[tokio::test]
async fn sec_14_a_key_has_its_own_limit() -> TestResult {
    each_database("sec_14_key_limit", async |db| {
        let rig = Rig::start(db, 50).await?;
        let org = Org::start(&rig).await?;
        let first = super::access_keys::key_of(&rig, &org.admin, KeyChoice::Everything).await?;
        let second = super::access_keys::key_of(&rig, &org.admin, KeyChoice::Everything).await?;
        // Three requests per key and minute.
        let limits = LimitsConfig {
            access_key_requests_per_minute: 3,
            ..LimitsConfig::default()
        };
        let app = app_with_probe_and_limits(&rig, None, limits);
        let mut script = Browser::new(&app, &called());
        for _ in 0..3 {
            let reply = script.script("GET", KEY_ROUTE, None, &first).await?;
            assert_eq!(reply.status, StatusCode::OK);
        }
        let limited = script.script("GET", KEY_ROUTE, None, &first).await?;
        assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(limited.code(), "rate_limit.exceeded");
        assert!(limited.header("retry-after").parse::<u64>()? >= 1);
        // Another key from the same network, and the person's own session, are not affected.
        let other = script.script("GET", KEY_ROUTE, None, &second).await?;
        assert_eq!(other.status, StatusCode::OK);
        let mut ada = Browser::new(&app, &called());
        ada.send(
            "POST",
            "/api/v1/auth/sign-in",
            Some(super::http::sign_in_body("ada@example.test", PASSWORD)),
        )
        .await?;
        for _ in 0..5 {
            assert_eq!(
                ada.send("GET", KEY_ROUTE, None).await?.status,
                StatusCode::OK
            );
        }
        // Keys that are not keys share one counter and cannot use up the limit of a key that is.
        let third = super::access_keys::key_of(&rig, &org.admin, KeyChoice::Everything).await?;
        for _ in 0..8 {
            script.script("GET", KEY_ROUTE, None, "bb_nonsense").await?;
        }
        let fine = script.script("GET", KEY_ROUTE, None, &third).await?;
        assert_eq!(fine.status, StatusCode::OK);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_an_expired_a_deleted_and_an_unknown_key_get_the_same_answer_over_http()
-> TestResult {
    each_database("auth_07_same_answer_http", async |db| {
        let rig = Rig::start(db, 50).await?;
        let org = Org::start(&rig).await?;
        let runs_out = super::access_keys::made(
            &rig,
            &org.admin,
            KeyChoice::Everything,
            Some(super::access_keys::later(rig.clock_now(), 2 * 24 * 3600).to_string()),
        )
        .await?;
        let deleted =
            super::access_keys::made(&rig, &org.admin, KeyChoice::Everything, None).await?;
        rig.api()
            .delete_access_key(
                Rig::info(),
                DeleteAccessKey {
                    session_token: org.admin.session.clone(),
                    id: deleted.key.id,
                },
            )
            .await?;
        let app = app_with_probe(&rig, None);
        let mut script = Browser::new(&app, &called());
        let works = script
            .script("GET", KEY_ROUTE, None, &runs_out.secret)
            .await?;
        assert_eq!(works.status, StatusCode::OK);
        rig.clock.advance(3 * 24 * 3600);
        let unknown = AccessKeySecret::generate()?.encode().as_str().to_owned();
        let expired = script
            .script("GET", KEY_ROUTE, None, &runs_out.secret)
            .await?;
        let gone = script
            .script("GET", KEY_ROUTE, None, &deleted.secret)
            .await?;
        let never = script.script("GET", KEY_ROUTE, None, &unknown).await?;
        let garbage = script.script("GET", KEY_ROUTE, None, "bb_garbage").await?;
        assert_eq!(
            alike(&expired),
            (401, "auth.not_signed_in".to_owned(), alike(&expired).2)
        );
        for reply in [&gone, &never, &garbage] {
            assert_eq!(alike(reply), alike(&expired));
            // A key has no cookie, so nothing is cleared.
            assert!(reply.cookies().is_empty());
            assert_eq!(reply.header("clear-site-data"), "");
        }
        Ok(())
    })
    .await
}

/// The routes of *My settings*, which keys may use.
const MY_SETTINGS: [&str; 3] = ["listMySettings", "changeMySetting", "resetMySetting"];

#[tokio::test]
async fn auth_07_no_route_of_the_owners_sign_in_data_opens_for_a_key() -> TestResult {
    each_database("auth_07_sign_in_data_http", async |db| {
        let rig = Rig::start(db, 50).await?;
        let org = Org::start(&rig).await?;
        let key = super::access_keys::key_of(&rig, &org.admin, KeyChoice::Everything).await?;
        let app = app_with_probe(&rig, None);
        let mut script = Browser::new(&app, &called());
        let mut tried = 0;
        for operation in operations() {
            if !matches!(operation.needs, Declared::OwnAccount | Declared::SignedIn) {
                continue;
            }
            // *My settings* are the person's choices, not their sign-in data: a key acts as
            // the person, reads them, and writes them if it may change (tested in
            // `settings_keys`).
            if MY_SETTINGS.contains(&operation.id.as_str()) {
                continue;
            }
            let (method, template) = (&operation.method, &operation.path);
            let reads_or_deletes = *method == Method::GET || *method == Method::DELETE;
            let body = (!reads_or_deletes).then(|| json!({}));
            let reply = script
                .script(method.as_str(), &example_path(template), body, &key)
                .await?;
            // Never a success, and a request that gets as far as the use case is turned away
            // as a key where no key may go.
            assert!(
                reply.status.is_client_error(),
                "{method} {template}: {}",
                reply.status
            );
            assert_ne!(reply.code(), "request.wrong_token", "{method} {template}");
            if reads_or_deletes {
                assert_eq!(
                    reply.code(),
                    "auth.access_key_not_allowed",
                    "{method} {template}"
                );
                assert_eq!(reply.status, StatusCode::FORBIDDEN);
            }
            tried += 1;
        }
        assert!(tried >= 15, "{tried}");
        // The key still works where a key may go, and the owner's own data was not touched.
        let listed = script.script("GET", KEY_ROUTE, None, &key).await?;
        assert_eq!(listed.status, StatusCode::OK);
        let mut ada = Browser::new(&app, &called());
        ada.send(
            "POST",
            "/api/v1/auth/sign-in",
            Some(super::http::sign_in_body("ada@example.test", PASSWORD)),
        )
        .await?;
        let me: Value = ada.send("GET", "/api/v1/auth/me", None).await?.body;
        assert_eq!(me["email"], "ada@example.test");
        Ok(())
    })
    .await
}

/// Ada, signed in in a browser of her own.
async fn ada_in_a_browser(app: &axum::Router) -> Result<Browser, Failure> {
    let mut ada = Browser::new(app, &called());
    ada.send(
        "POST",
        "/api/v1/auth/sign-in",
        Some(super::http::sign_in_body("ada@example.test", PASSWORD)),
    )
    .await?;
    Ok(ada)
}

/// The rules of this organization for a key and the projects to choose from, as the page asks.
async fn rules_and_projects(ada: &mut Browser) -> Result<(), Failure> {
    let rules = ada.send("GET", "/api/v1/access-keys/rules", None).await?;
    assert_eq!(rules.body["default_days"], 90);
    assert_eq!(rules.body["longest_days"], 365);
    assert_eq!(rules.body["permissions"][0]["key"], "see-projects");
    assert_eq!(rules.body["permissions"][0]["topic"], "projects");
    let projects = ada
        .send("GET", "/api/v1/access-keys/projects", None)
        .await?;
    assert_eq!(projects.body["items"], json!([]));
    assert_eq!(projects.body["next_cursor"], Value::Null);
    Ok(())
}

#[tokio::test]
async fn auth_07_a_person_makes_lists_and_deletes_keys_over_http() -> TestResult {
    each_database("auth_07_keys_over_http", async |db| {
        let rig = Rig::start(db, 50).await?;
        Org::start(&rig).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = ada_in_a_browser(&app).await?;
        let make = |name: &str| json!({"password": PASSWORD, "name": name, "choice": "see-only"});
        // The answer that makes a key holds its secret once, and it is never stored.
        let first = ada
            .send("POST", "/api/v1/access-keys", Some(make("First")))
            .await?;
        assert_eq!(first.status, StatusCode::OK, "{:?}", first.body);
        let secret = first.body["secret"].as_str().ok_or("no secret")?.to_owned();
        assert!(secret.starts_with("bb_") && secret.len() == 64);
        assert_eq!(first.header("cache-control"), "no-store");
        assert_eq!(first.body["key"]["prefix"], secret[3..11]);
        assert_eq!(first.body["key"]["choice"], "see-only");
        for name in ["Second", "Third"] {
            // Each key is made a moment after the one before, so "newest first" has an answer.
            rig.clock.advance(1);
            let reply = ada
                .send("POST", "/api/v1/access-keys", Some(make(name)))
                .await?;
            assert_eq!(reply.status, StatusCode::OK);
        }
        // A bad body is refused before anything is made; so is a wrong password.
        let bad = ada
            .send("POST", "/api/v1/access-keys", Some(json!({"name": "x"})))
            .await?;
        assert_eq!(bad.code(), "request.invalid");
        let wrong = ada
            .send(
                "POST",
                "/api/v1/access-keys",
                Some(json!({"password": "Wrong-Password-1", "name": "x", "choice": "see-only"})),
            )
            .await?;
        assert_eq!(wrong.code(), "auth.wrong_credentials");
        // The list pages newest first, takes the fields asked for and never holds the secret.
        let page = ada
            .send(
                "GET",
                "/api/v1/access-keys?limit=2&fields=id,name,last_used_at",
                None,
            )
            .await?;
        assert_eq!(page.body["items"].as_array().map(Vec::len), Some(2));
        assert_eq!(page.body["items"][0]["name"], "Third");
        assert!(page.body["items"][0].get("prefix").is_none());
        let cursor = page.body["next_cursor"]
            .as_str()
            .ok_or("no cursor")?
            .to_owned();
        let rest = ada
            .send(
                "GET",
                &format!("/api/v1/access-keys?limit=2&cursor={cursor}"),
                None,
            )
            .await?;
        assert_eq!(rest.body["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(rest.body["items"][0]["name"], "First");
        assert_eq!(rest.body["next_cursor"], Value::Null);
        assert!(!rest.body.to_string().contains(&secret[12..]));
        let unknown = ada
            .send("GET", "/api/v1/access-keys?fields=secret", None)
            .await?;
        assert_eq!(unknown.code(), "request.unknown_field");
        let bad_cursor = ada
            .send("GET", "/api/v1/access-keys?cursor=nonsense", None)
            .await?;
        assert_eq!(bad_cursor.code(), "request.bad_cursor");
        // Deleting a key: it stops working at once, and a second delete finds nothing.
        rules_and_projects(&mut ada).await?;
        let id = first.body["key"]["id"].as_str().ok_or("no id")?.to_owned();
        let mut script = Browser::new(&app, &called());
        assert_eq!(
            script
                .script("GET", CHANGE_ROUTE, None, &secret)
                .await?
                .status,
            StatusCode::FORBIDDEN,
            "a key that may only see is not let into a route that needs more"
        );
        let deleted = ada
            .send("DELETE", &format!("/api/v1/access-keys/{id}"), None)
            .await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        assert_eq!(
            script.script("GET", KEY_ROUTE, None, &secret).await?.code(),
            "auth.not_signed_in"
        );
        let again = ada
            .send("DELETE", &format!("/api/v1/access-keys/{id}"), None)
            .await?;
        assert_eq!(again.code(), "resource.not_found");
        let nonsense = ada
            .send("DELETE", "/api/v1/access-keys/not-an-id", None)
            .await?;
        assert_eq!(nonsense.code(), "resource.not_found");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_a_key_is_judged_before_the_body_or_the_address_of_the_request_is_read()
-> TestResult {
    each_database("auth_07_actor_before_body", async |db| {
        let rig = Rig::start(db, 50).await?;
        let org = Org::start(&rig).await?;
        let key = super::access_keys::key_of(&rig, &org.admin, KeyChoice::Everything).await?;
        let app = app_with_probe(&rig, None);
        let mut script = Browser::new(&app, &called());
        let before = rig.database.security_log_entries(0, 1000).await?.len();
        // A body that is not valid, and an address that is not a key: a key gets the answer of a
        // key where no key may go, not a 400 or a 404 that depends on what it sent.
        let bad_body = script
            .script("POST", "/api/v1/access-keys", Some(json!({})), &key)
            .await?;
        assert_eq!(bad_body.status, StatusCode::FORBIDDEN);
        assert_eq!(bad_body.code(), "auth.access_key_not_allowed");
        let bad_address = script
            .script("DELETE", "/api/v1/access-keys/not-an-id", None, &key)
            .await?;
        assert_eq!(bad_address.status, StatusCode::FORBIDDEN);
        assert_eq!(bad_address.code(), "auth.access_key_not_allowed");
        // Both are written into the security log.
        let entries = rig.database.security_log_entries(0, 1000).await?;
        let refused = entries
            .iter()
            .skip(before)
            .filter(|entry| entry.action.as_str() == SecurityAction::AccessKeyRefused.code())
            .count();
        assert_eq!(refused, 2);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_a_key_that_may_see_the_status_reads_it_and_no_other_key_does() -> TestResult {
    each_database("auth_07_status_by_key", async |db| {
        let rig = Rig::start(db, 50).await?;
        let org = Org::start(&rig).await?;
        let app = app_with_probe(&rig, None);
        let mut script = Browser::new(&app, &called());
        let only = |permission: Permission| {
            let rig = &rig;
            let org = &org;
            async move {
                rig.api()
                    .make_access_key(
                        Rig::info(),
                        MakeAccessKey {
                            session_token: org.admin.session.clone(),
                            password: PASSWORD.to_owned(),
                            name: "Status".to_owned(),
                            choice: KeyChoice::Permissions,
                            permissions: vec![permission],
                            projects: Vec::new(),
                            expires_at: None,
                        },
                    )
                    .await
                    .map(|created| created.into_value().secret)
            }
        };
        let status = only(Permission::SeeStatus).await?;
        let reply = script
            .script("GET", "/api/v1/admin/status", None, &status)
            .await?;
        assert_eq!(reply.status, StatusCode::OK, "{:?}", reply.body);
        assert!(reply.body.get("version").is_some());
        let see_only = super::access_keys::key_of(&rig, &org.admin, KeyChoice::SeeOnly).await?;
        let reply = script
            .script("GET", "/api/v1/admin/status", None, &see_only)
            .await?;
        assert_eq!(reply.status, StatusCode::OK);
        // A key without *See status* is turned away, and no key at all is not signed in.
        let other = only(Permission::CheckAccess).await?;
        let reply = script
            .script("GET", "/api/v1/admin/status", None, &other)
            .await?;
        assert_eq!(reply.status, StatusCode::FORBIDDEN);
        let nobody = script
            .script("GET", "/api/v1/admin/status", None, "bb_nonsense")
            .await?;
        assert_eq!(nobody.status, StatusCode::UNAUTHORIZED);
        Ok(())
    })
    .await
}
