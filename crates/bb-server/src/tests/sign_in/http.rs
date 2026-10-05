//! Signing in over HTTP: the cookies, the token against forged requests (SEC-12), the limits
//! (SEC-14), the answers that never say whether an account exists, and every route once.

use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, StatusCode, header};
use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_api::{
    AuthHandle, ClientAddr, LimitsConfig, Options, ProbeHandle, WebSource, openapi, router,
};
use bb_app::auth::AuthStore as _;
use bb_app::context::Clock as _;
use bb_auth::TotpSecret;
use bb_testing::{TestDatabase, each_database};
use serde_json::{Value, json};
use tower::ServiceExt as _;
use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator as _};
use webauthn_authenticator_rs::softpasskey::SoftPasskey;

use super::rig::{OTHER_PASSWORD, Outcome, PASSWORD, PUBLIC_URL, Rig};

pub(super) type Failure = Box<dyn std::error::Error + Send + Sync>;

/// The third number of the next browser's address (a network is the first three numbers).
static NEXT_HOST: AtomicU8 = AtomicU8::new(1);

const SESSION: &str = "__Host-bb_session";
const CSRF: &str = "__Host-bb_csrf";

/// What the server answered.
pub(super) struct Reply {
    pub(super) status: StatusCode,
    headers: HeaderMap,
    pub(super) body: Value,
}

impl Reply {
    pub(super) fn code(&self) -> &str {
        self.body["code"].as_str().unwrap_or_default()
    }

    pub(super) fn header(&self, name: &str) -> &str {
        self.headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
    }

    fn cookies(&self) -> Vec<String> {
        self.headers
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .map(str::to_owned)
            .collect()
    }
}

/// How a request carries the token of the session.
enum Token<'a> {
    /// Like the generated client: the value of the `__Host-bb_csrf` cookie, if there is one.
    FromCookie,
    /// No token at all.
    Nothing,
    /// This value.
    Is(&'a str),
}

/// A browser: it keeps cookies and sends the token like the app does.
pub(super) struct Browser {
    app: Router,
    jar: BTreeMap<String, String>,
    called: Arc<Mutex<BTreeSet<String>>>,
    peer: SocketAddr,
    /// Headers every request of this browser carries besides the usual ones.
    headers: Vec<(&'static str, String)>,
}

pub(super) fn options(rig: &Rig, limits: LimitsConfig, auth: bool) -> Options {
    Options {
        web: WebSource::Folder("no/such/folder".into()),
        hsts_max_age: None,
        header_timeout: Duration::from_secs(30),
        data_folder: std::env::temp_dir(),
        trusted_proxies: vec![],
        limits,
        auth: if auth {
            AuthHandle::new(rig.services.clone())
        } else {
            AuthHandle::none()
        },
        probe: ProbeHandle::none(),
    }
}

impl Browser {
    pub(super) fn new(app: &Router, called: &Arc<Mutex<BTreeSet<String>>>) -> Self {
        Self {
            app: app.clone(),
            jar: BTreeMap::new(),
            called: Arc::clone(called),
            headers: Vec::new(),
            // Every browser is on a network of its own, so the limit per network of one test
            // browser does not use up another's.
            peer: SocketAddr::from((
                [203, 0, NEXT_HOST.fetch_add(1, Ordering::SeqCst), 9],
                50_000,
            )),
        }
    }

    /// From now on every request carries this header (`If-None-Match`, `Accept-Encoding`).
    pub(super) fn set_header(&mut self, name: &'static str, value: &str) {
        self.headers.retain(|(known, _)| *known != name);
        self.headers.push((name, value.to_owned()));
    }

    /// From now on no extra headers.
    pub(super) fn clear_headers(&mut self) {
        self.headers.clear();
    }

    pub(super) async fn send(
        &mut self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Reply, Failure> {
        self.send_as(method, path, body, Token::FromCookie, "http://bugaboo.test")
            .await
    }

    /// `template` is the address as the API description writes it (with `{…}`), for the
    /// bookkeeping of which routes were used.
    async fn route(
        &mut self,
        method: &str,
        template: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Reply, Failure> {
        self.called
            .lock()
            .map_err(|_| "poisoned")?
            .insert(format!("{method} {template}"));
        self.send(method, path, body).await
    }

    async fn send_as(
        &mut self,
        method: &str,
        path: &str,
        body: Option<Value>,
        token: Token<'_>,
        origin: &str,
    ) -> Result<Reply, Failure> {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::HOST, "bugaboo.test")
            .header(header::ORIGIN, origin)
            .header(
                header::USER_AGENT,
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:130.0) Gecko/20100101 Firefox/130.0",
            );
        for (name, value) in &self.headers {
            request = request.header(*name, value);
        }
        if !self.jar.is_empty() {
            let line = self
                .jar
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join("; ");
            request = request.header(header::COOKIE, line);
        }
        match token {
            Token::FromCookie => {
                if let Some(value) = self.jar.get(CSRF) {
                    request = request.header("x-csrf-token", value);
                }
            }
            Token::Nothing => {}
            Token::Is(value) => request = request.header("x-csrf-token", value),
        }
        let body = match body {
            Some(value) => {
                request = request.header(header::CONTENT_TYPE, "application/json");
                Body::from(serde_json::to_vec(&value)?)
            }
            None => Body::empty(),
        };
        let mut request = request.body(body)?;
        request
            .extensions_mut()
            .insert(ConnectInfo(ClientAddr(self.peer)));
        let response = self.app.clone().oneshot(request).await?;
        let (parts, body) = response.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX).await?;
        let reply = Reply {
            status: parts.status,
            headers: parts.headers,
            body: if bytes.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(&bytes)?
            },
        };
        assert_cache_rules(method, path, &reply);
        for cookie in reply.cookies() {
            let pair = cookie.split(';').next().unwrap_or_default();
            if let Some((name, value)) = pair.split_once('=') {
                if cookie.contains("Max-Age=0") {
                    self.jar.remove(name);
                } else {
                    self.jar.insert(name.to_owned(), value.to_owned());
                }
            }
        }
        Ok(reply)
    }
}

/// What every answer of the API says about keeping it (PERF-03, `bb-api` `etag`), checked on
/// every request of every test that goes through a [`Browser`]: a read that worked may be kept
/// and must be asked about again, and has a tag; everything else is never stored.
fn assert_cache_rules(method: &str, path: &str, reply: &Reply) {
    let kept = method == "GET" && reply.status == StatusCode::OK;
    let rule = reply.header("cache-control");
    if kept {
        assert_eq!(rule, "private, no-cache", "{method} {path}");
        assert!(reply.header("etag").starts_with("W/\""), "{method} {path}");
    } else if reply.status == StatusCode::NOT_MODIFIED {
        assert_eq!(rule, "private, no-cache", "{method} {path}");
    } else {
        assert_eq!(rule, "no-store", "{method} {path}");
        assert_eq!(reply.header("etag"), "", "{method} {path}");
    }
}

pub(super) fn setup_body(rig: &Rig) -> Value {
    json!({
        "code": rig.setup_code,
        "display_name": "Ada Admin",
        "email": "ada@example.test",
        "password": PASSWORD,
    })
}

pub(super) async fn rig_and_app(
    db: &TestDatabase,
    failures: u32,
) -> Result<(Rig, Router), Failure> {
    let rig = Rig::start(db, failures).await?;
    let app = router(options(&rig, LimitsConfig::default(), true));
    Ok((rig, app))
}

pub(super) fn called() -> Arc<Mutex<BTreeSet<String>>> {
    Arc::new(Mutex::new(BTreeSet::new()))
}

pub(super) fn sign_in_body(email: &str, password: &str) -> Value {
    json!({"email": email, "password": password})
}

#[tokio::test]
async fn sec_12_signing_in_sets_the_session_cookie_and_the_csrf_cookie() -> Outcome {
    each_database("sec_12_http_cookies", async |db| {
        let (rig, app) = rig_and_app(db, 10).await?;
        let mut browser = Browser::new(&app, &called());
        let reply = browser
            .send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.body["person"]["email"], "ada@example.test");
        assert_eq!(reply.body["person"]["administrator"], true);
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
        let again = second
            .send(
                "POST",
                "/api/v1/auth/sign-in",
                Some(sign_in_body("ADA@example.test", PASSWORD)),
            )
            .await?;
        assert_eq!(again.status, StatusCode::OK);
        assert_ne!(second.jar[SESSION], browser.jar[SESSION]);
        assert_ne!(second.jar[CSRF], browser.jar[CSRF]);
        // The cookie is bound to the session and the browser asks for who it is with it.
        let me = browser.send("GET", "/api/v1/auth/me", None).await?;
        assert_eq!(me.status, StatusCode::OK);
        assert_eq!(me.body["email"], "ada@example.test");
        assert_eq!(me.header("cache-control"), "private, no-cache");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_a_change_needs_the_token_of_its_own_session() -> Outcome {
    each_database("sec_12_http_csrf", async |db| {
        let (rig, app) = rig_and_app(db, 10).await?;
        let mut browser = Browser::new(&app, &called());
        browser
            .send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let mut other = Browser::new(&app, &called());
        other
            .send(
                "POST",
                "/api/v1/auth/sign-in",
                Some(sign_in_body("ada@example.test", PASSWORD)),
            )
            .await?;
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
-> Outcome {
    each_database("sec_12_http_every_route", async |db| {
        let (rig, app) = rig_and_app(db, 10).await?;
        let mut browser = Browser::new(&app, &called());
        browser
            .send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        // The routes people use before they have a session: no token can exist yet.
        let open = [
            "/api/v1/auth/setup",
            "/api/v1/auth/sign-in",
            "/api/v1/auth/sign-up",
            "/api/v1/auth/passkey/sign-in/options",
            "/api/v1/auth/passkey/sign-in",
            "/api/v1/auth/invitation/lookup",
            "/api/v1/auth/invitation/accept",
        ];
        let document = serde_json::to_value(openapi::document())?;
        let paths = document["paths"].as_object().ok_or("no paths")?;
        let id = "0197a5e2-7c1f-4b0a-9a3e-2f6d1c8b4a10";
        let mut checked = 0;
        for (template, item) in paths {
            for method in ["post", "put", "delete", "patch"] {
                if item.get(method).is_none() {
                    continue;
                }
                let path = template
                    .replace("{session_id}", id)
                    .replace("{passkey_id}", id)
                    .replace("{invitation_id}", id);
                let reply = browser
                    .send_as(
                        &method.to_uppercase(),
                        &path,
                        Some(json!({})),
                        Token::Nothing,
                        "http://bugaboo.test",
                    )
                    .await?;
                if open.contains(&template.as_str()) {
                    assert_ne!(reply.code(), "request.wrong_token", "{method} {template}");
                } else {
                    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{method} {template}");
                    assert_eq!(reply.code(), "request.wrong_token", "{method} {template}");
                    checked += 1;
                }
            }
        }
        assert!(checked >= 15, "{checked}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_failed_sign_ins_are_turned_away_with_a_wait_and_the_answers_look_alike() -> Outcome
{
    each_database("sec_14_http_limit", async |db| {
        let (rig, app) = rig_and_app(db, 3).await?;
        let mut browser = Browser::new(&app, &called());
        browser
            .send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        // Every try comes from another network, so only the account is limited here.
        let fresh = || Browser::new(&app, &called());
        let wrong = fresh()
            .send(
                "POST",
                "/api/v1/auth/sign-in",
                Some(sign_in_body("ada@example.test", "Wrong-Password-1")),
            )
            .await?;
        let unknown = fresh()
            .send(
                "POST",
                "/api/v1/auth/sign-in",
                Some(sign_in_body("nobody@example.test", "Wrong-Password-1")),
            )
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
            let reply = fresh()
                .send(
                    "POST",
                    "/api/v1/auth/sign-in",
                    Some(sign_in_body("ada@example.test", "Wrong-Password-1")),
                )
                .await?;
            assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        }
        let reply = fresh()
            .send(
                "POST",
                "/api/v1/auth/sign-in",
                Some(sign_in_body("ada@example.test", PASSWORD)),
            )
            .await?;
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
async fn sec_14_failures_of_one_network_are_limited_whatever_the_account() -> Outcome {
    each_database("sec_14_http_network_failures", async |db| {
        let (rig, app) = rig_and_app(db, 3).await?;
        Browser::new(&app, &called())
            .send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let mut guesser = Browser::new(&app, &called());
        // Three wrong tries at three different accounts use up the network.
        for number in 0..3 {
            let reply = guesser
                .send(
                    "POST",
                    "/api/v1/auth/sign-in",
                    Some(sign_in_body(
                        &format!("person{number}@example.test"),
                        "Wrong-Password-1",
                    )),
                )
                .await?;
            assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        }
        // Now even a right password from this network waits ...
        let reply = guesser
            .send(
                "POST",
                "/api/v1/auth/sign-in",
                Some(sign_in_body("ada@example.test", PASSWORD)),
            )
            .await?;
        assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
        // ... and another network is not affected.
        let mut other = Browser::new(&app, &called());
        let reply = other
            .send(
                "POST",
                "/api/v1/auth/sign-in",
                Some(sign_in_body("ada@example.test", PASSWORD)),
            )
            .await?;
        assert_eq!(reply.status, StatusCode::OK);
        // Many successes from one network never count: they give their try back.
        for _ in 0..5 {
            let reply = other
                .send(
                    "POST",
                    "/api/v1/auth/sign-in",
                    Some(sign_in_body("ada@example.test", PASSWORD)),
                )
                .await?;
            assert_eq!(reply.status, StatusCode::OK);
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_one_network_is_limited_in_its_tries_whatever_the_account() -> Outcome {
    each_database("sec_14_http_network_limit", async |db| {
        let rig = Rig::start(db, 100).await?;
        let app = router(options(
            &rig,
            LimitsConfig {
                requests_per_minute: 1000,
                sign_in_attempts_per_minute: 4,
                ..LimitsConfig::default()
            },
            true,
        ));
        let mut browser = Browser::new(&app, &called());
        let mut statuses = Vec::new();
        for number in 0..6 {
            let reply = browser
                .send(
                    "POST",
                    "/api/v1/auth/sign-in",
                    Some(sign_in_body(
                        &format!("person{number}@example.test"),
                        PASSWORD,
                    )),
                )
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
async fn auth_01_without_a_database_the_sign_in_routes_say_so_and_nothing_else_breaks() -> Outcome {
    each_database("auth_01_http_unavailable", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = router(options(&rig, LimitsConfig::default(), false));
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

fn origin() -> Result<Url, Failure> {
    Ok(Url::parse(PUBLIC_URL)?)
}

pub(super) async fn totp(rig: &Rig, email: &str) -> Result<String, Failure> {
    let person = rig.person_id(email).await?;
    let secret = rig.storage.totp_secret(person).await?.ok_or("no secret")?;
    Ok(TotpSecret::from_bytes(&secret).code_at(rig.clock.now())?)
}

fn ok(reply: &Reply) -> bool {
    reply.status == StatusCode::OK || reply.status == StatusCode::NO_CONTENT
}

/// A text of the answer, found by its JSON pointer.
fn text(reply: &Reply, pointer: &str) -> Result<String, Failure> {
    reply
        .body
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            format!(
                "no {pointer} in the answer {}: {}",
                reply.status, reply.body
            )
            .into()
        })
}

/// What the tour of the routes shares.
struct Tour {
    rig: Rig,
    app: Router,
    used: Arc<Mutex<BTreeSet<String>>>,
    ada: Browser,
}

const API: &str = "/api/v1/auth";

async fn first_start(tour: &mut Tour) -> Outcome {
    let rig = &tour.rig;
    let ada = &mut tour.ada;
    let api = API;
    // The first start.
    let reply = ada
        .route("GET", "/api/v1/auth/status", &format!("{api}/status"), None)
        .await?;
    assert_eq!(
        reply.body,
        json!({"setup_needed": true, "sign_up_open": false, "passkeys_available": true})
    );
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/setup",
            &format!("{api}/setup"),
            Some(setup_body(rig)),
        )
        .await?;
    assert!(ok(&reply));
    let reply = ada
        .route("GET", "/api/v1/auth/me", &format!("{api}/me"), None)
        .await?;
    assert_eq!(reply.body["display_name"], "Ada Admin");

    Ok(())
}

async fn invitations(tour: &mut Tour) -> Outcome {
    let app = &tour.app;
    let used = &tour.used;
    let ada = &mut tour.ada;
    let api = API;
    // Invitations: make, list, look up, accept, revoke.
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/invitations",
            &format!("{api}/invitations"),
            Some(json!({"email": "bob@example.test"})),
        )
        .await?;
    assert!(ok(&reply));
    let secret = text(&reply, "/link_path")?
        .split_once('#')
        .ok_or("no secret")?
        .1
        .to_owned();
    let reply = ada
        .route(
            "GET",
            "/api/v1/auth/invitations",
            &format!("{api}/invitations"),
            None,
        )
        .await?;
    assert_eq!(reply.body["items"].as_array().map(Vec::len), Some(1));
    let mut bob = Browser::new(app, used);
    let reply = bob
        .route(
            "POST",
            "/api/v1/auth/invitation/lookup",
            &format!("{api}/invitation/lookup"),
            Some(json!({"secret": secret})),
        )
        .await?;
    assert_eq!(reply.body["email"], "bob@example.test");
    let reply = bob
        .route(
            "POST",
            "/api/v1/auth/invitation/accept",
            &format!("{api}/invitation/accept"),
            Some(json!({"secret": secret, "display_name": "Bob", "password": PASSWORD})),
        )
        .await?;
    assert!(ok(&reply));
    let reply = ada
        .send(
            "POST",
            &format!("{api}/invitations"),
            Some(json!({"email": "eve@example.test"})),
        )
        .await?;
    assert!(ok(&reply));
    let listed = ada.send("GET", &format!("{api}/invitations"), None).await?;
    let id = text(&listed, "/items/0/id")?;
    let reply = ada
        .route(
            "DELETE",
            "/api/v1/auth/invitations/{invitation_id}",
            &format!("{api}/invitations/{id}"),
            None,
        )
        .await?;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);

    Ok(())
}

async fn sign_up(tour: &mut Tour) -> Outcome {
    let app = &tour.app;
    let used = &tour.used;
    let ada = &mut tour.ada;
    let api = API;
    // Signing up on one's own: the rule, then somebody who uses it.
    let reply = ada
        .route(
            "GET",
            "/api/v1/auth/sign-up-policy",
            &format!("{api}/sign-up-policy"),
            None,
        )
        .await?;
    assert_eq!(reply.body, json!({"enabled": false, "domains": []}));
    let reply = ada
        .route(
            "PUT",
            "/api/v1/auth/sign-up-policy",
            &format!("{api}/sign-up-policy"),
            Some(json!({"enabled": true, "domains": ["example.test"]})),
        )
        .await?;
    assert_eq!(
        reply.body,
        json!({"enabled": true, "domains": ["example.test"]})
    );
    let mut eve = Browser::new(app, used);
    let reply = eve
        .route(
            "POST",
            "/api/v1/auth/sign-up",
            &format!("{api}/sign-up"),
            Some(json!({"email": "eve@example.test", "display_name": "Eve", "password": PASSWORD})),
        )
        .await?;
    assert!(ok(&reply));
    let reply = eve
        .send(
            "POST",
            &format!("{api}/sign-up"),
            Some(json!({"email": "x@evil.test", "display_name": "X", "password": PASSWORD})),
        )
        .await?;
    assert_eq!(reply.code(), "auth.sign_up_refused");

    Ok(())
}

async fn code_app(tour: &mut Tour) -> Outcome {
    let rig = &tour.rig;
    let app = &tour.app;
    let used = &tour.used;
    let ada = &mut tour.ada;
    let api = API;
    // The code app: set up, switch on, sign in with it, new backup codes, switch off.
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/two-step/code-app",
            &format!("{api}/two-step/code-app"),
            Some(json!({"password": PASSWORD})),
        )
        .await?;
    assert!(
        reply.body["otpauth_url"]
            .as_str()
            .is_some_and(|url| url.starts_with("otpauth://"))
    );
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/two-step/code-app/confirm",
            &format!("{api}/two-step/code-app/confirm"),
            Some(json!({"code": totp(rig, "ada@example.test").await?, "password": PASSWORD})),
        )
        .await?;
    assert_eq!(reply.body["codes"].as_array().map(Vec::len), Some(10));
    let reply = ada
        .route(
            "GET",
            "/api/v1/auth/two-step",
            &format!("{api}/two-step"),
            None,
        )
        .await?;
    assert_eq!(reply.body["code_app"], true);
    assert_eq!(reply.body["backup_codes_left"], 10);
    rig.clock.advance(30);
    let mut laptop = Browser::new(app, used);
    let reply = laptop
        .send(
            "POST",
            &format!("{api}/sign-in"),
            Some(sign_in_body("ada@example.test", PASSWORD)),
        )
        .await?;
    assert_eq!(
        reply.body["second_step"],
        json!({"code_app": true, "passkey": false, "backup_code": true})
    );
    let reply = laptop.send("GET", &format!("{api}/me"), None).await?;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    let reply = laptop
        .route(
            "POST",
            "/api/v1/auth/second-step",
            &format!("{api}/second-step"),
            Some(json!({"code": totp(rig, "ada@example.test").await?})),
        )
        .await?;
    assert!(ok(&reply));
    assert_eq!(
        laptop.send("GET", &format!("{api}/me"), None).await?.status,
        StatusCode::OK
    );
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/two-step/backup-codes",
            &format!("{api}/two-step/backup-codes"),
            Some(json!({"password": PASSWORD})),
        )
        .await?;
    assert_eq!(reply.body["codes"].as_array().map(Vec::len), Some(10));
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/two-step/code-app/remove",
            &format!("{api}/two-step/code-app/remove"),
            Some(json!({"password": PASSWORD})),
        )
        .await?;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);

    Ok(())
}

async fn passkeys(tour: &mut Tour) -> Outcome {
    let ada = &mut tour.ada;
    let api = API;
    // Passkeys: add, use as the second step, sign in with one alone, remove.
    let mut device = SoftPasskey::new(true);
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/passkeys/options",
            &format!("{api}/passkeys/options"),
            Some(json!({"password": PASSWORD})),
        )
        .await?;
    let challenge = text(&reply, "/challenge_id")?;
    let answer = device.do_registration(
        origin()?,
        serde_json::from_value(reply.body["options"].clone())?,
    )?;
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/passkeys",
            &format!("{api}/passkeys"),
            Some(json!({"challenge_id": challenge, "credential": answer, "label": "Laptop", "password": PASSWORD})),
        )
        .await?;
    let passkey_id = text(&reply, "/id")?;
    use_passkeys(tour, &mut device, &passkey_id).await
}

/// A passkey as the second step, one alone, and removing it.
async fn use_passkeys(tour: &mut Tour, device: &mut SoftPasskey, passkey_id: &str) -> Outcome {
    let rig = &tour.rig;
    let app = &tour.app;
    let used = &tour.used;
    let ada = &mut tour.ada;
    let api = API;
    let mut phone = Browser::new(app, used);
    phone
        .send(
            "POST",
            &format!("{api}/sign-in"),
            Some(sign_in_body("ada@example.test", PASSWORD)),
        )
        .await?;
    let reply = phone
        .route(
            "POST",
            "/api/v1/auth/second-step/passkey/options",
            &format!("{api}/second-step/passkey/options"),
            None,
        )
        .await?;
    let challenge = text(&reply, "/challenge_id")?;
    let answer = device.do_authentication(
        origin()?,
        serde_json::from_value(reply.body["options"].clone())?,
    )?;
    let reply = phone
        .route(
            "POST",
            "/api/v1/auth/second-step/passkey",
            &format!("{api}/second-step/passkey"),
            Some(json!({"challenge_id": challenge, "credential": answer})),
        )
        .await?;
    assert!(ok(&reply));
    assert_eq!(
        phone.send("GET", &format!("{api}/me"), None).await?.status,
        StatusCode::OK
    );
    let mut tablet = Browser::new(app, used);
    let reply = tablet
        .route(
            "POST",
            "/api/v1/auth/passkey/sign-in/options",
            &format!("{api}/passkey/sign-in/options"),
            None,
        )
        .await?;
    let challenge = text(&reply, "/challenge_id")?;
    let person = rig.person_id("ada@example.test").await?;
    let stored = rig.storage.passkeys_of(person).await?;
    let mut options = reply.body["options"].clone();
    options["publicKey"]["allowCredentials"] = json!([{
        "type": "public-key",
        "id": Base64UrlUnpadded::encode_string(&stored[0].credential_id),
    }]);
    let mut answer = serde_json::to_value(
        device.do_authentication(origin()?, serde_json::from_value(options)?)?,
    )?;
    answer["response"]["userHandle"] = json!(Base64UrlUnpadded::encode_string(&person.to_bytes()));
    let reply = tablet
        .route(
            "POST",
            "/api/v1/auth/passkey/sign-in",
            &format!("{api}/passkey/sign-in"),
            Some(json!({"challenge_id": challenge, "credential": answer})),
        )
        .await?;
    assert!(ok(&reply));
    assert_eq!(
        tablet.send("GET", &format!("{api}/me"), None).await?.status,
        StatusCode::OK
    );
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/passkeys/{passkey_id}/remove",
            &format!("{api}/passkeys/{passkey_id}/remove"),
            Some(json!({"password": PASSWORD})),
        )
        .await?;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);

    Ok(())
}

async fn sessions_and_password(tour: &mut Tour) -> Outcome {
    let app = &tour.app;
    let used = &tour.used;
    let ada = &mut tour.ada;
    let api = API;
    // Sessions and the password: list, end one, change the password, end all, sign out.
    let reply = ada
        .route(
            "GET",
            "/api/v1/auth/sessions",
            &format!("{api}/sessions"),
            None,
        )
        .await?;
    let sessions = reply.body["sessions"].as_array().ok_or("no sessions")?;
    assert!(sessions.iter().any(|session| session["current"] == true));
    let other = sessions
        .iter()
        .find(|session| session["current"] == false)
        .and_then(|session| session["id"].as_str())
        .map(str::to_owned)
        .ok_or("no other session")?;
    let reply = ada
        .route(
            "DELETE",
            "/api/v1/auth/sessions/{session_id}",
            &format!("{api}/sessions/{other}"),
            None,
        )
        .await?;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/password",
            &format!("{api}/password"),
            Some(json!({"current_password": PASSWORD, "new_password": OTHER_PASSWORD})),
        )
        .await?;
    assert!(ok(&reply));
    let reply = ada
        .route(
            "DELETE",
            "/api/v1/auth/sessions",
            &format!("{api}/sessions"),
            None,
        )
        .await?;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert!(ada.jar.is_empty());
    let mut again = Browser::new(app, used);
    let reply = again
        .route(
            "POST",
            "/api/v1/auth/sign-in",
            &format!("{api}/sign-in"),
            Some(sign_in_body("ada@example.test", OTHER_PASSWORD)),
        )
        .await?;
    assert!(ok(&reply));
    let reply = again
        .route(
            "POST",
            "/api/v1/auth/sign-out",
            &format!("{api}/sign-out"),
            None,
        )
        .await?;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);

    Ok(())
}

/// Every route of the API description under /auth was used.
fn every_route_was_used(tour: &Tour) -> Outcome {
    let used = &tour.used;
    // Every route of the API description under /auth was used.
    let document = serde_json::to_value(openapi::document())?;
    let mut documented = BTreeSet::new();
    for (path, item) in document["paths"].as_object().ok_or("no paths")? {
        if !path.starts_with("/api/v1/auth/") {
            continue;
        }
        for method in ["get", "post", "put", "delete"] {
            if item.get(method).is_some() {
                documented.insert(format!("{} {path}", method.to_uppercase()));
            }
        }
    }
    let used = used.lock().map_err(|_| "poisoned")?.clone();
    assert_eq!(
        documented.difference(&used).collect::<Vec<_>>(),
        Vec::<&String>::new(),
        "documented routes that no test used"
    );
    assert!(documented.len() >= 28, "{}", documented.len());

    Ok(())
}

/// Every route of the sign-in family, used once the way the browser app will use it. The routes
/// that were used are compared with the API description at the end, so a new route that no
/// test uses fails here.
#[tokio::test]
async fn auth_01_every_route_of_signing_in_works_over_http() -> Outcome {
    each_database("auth_01_http_every_route", async |db| {
        let (rig, app) = rig_and_app(db, 10).await?;
        let used = called();
        let ada = Browser::new(&app, &used);
        let mut tour = Tour {
            rig,
            app,
            used,
            ada,
        };
        first_start(&mut tour).await?;
        invitations(&mut tour).await?;
        sign_up(&mut tour).await?;
        code_app(&mut tour).await?;
        passkeys(&mut tour).await?;
        sessions_and_password(&mut tour).await?;
        every_route_was_used(&tour)
    })
    .await
}

#[tokio::test]
async fn perf_02_the_invitation_list_takes_fields_cursor_and_limit_over_http() -> Outcome {
    each_database("perf_02_http_invitation_list", async |db| {
        let (rig, app) = rig_and_app(db, 10).await?;
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        for number in 0..3 {
            let reply = ada
                .send(
                    "POST",
                    "/api/v1/auth/invitations",
                    Some(json!({"email": format!("person{number}@example.test")})),
                )
                .await?;
            assert!(ok(&reply));
        }
        // Two fields, two to a page: only those keys, and a cursor for the rest.
        let first = ada
            .send(
                "GET",
                "/api/v1/auth/invitations?fields=id,email&limit=2",
                None,
            )
            .await?;
        assert_eq!(first.status, StatusCode::OK);
        let items = first.body["items"].as_array().ok_or("no items")?;
        assert_eq!(items.len(), 2);
        for item in items {
            let mut keys: Vec<&String> = item.as_object().ok_or("no object")?.keys().collect();
            keys.sort();
            assert_eq!(keys, ["email", "id"]);
        }
        let cursor = text(&first, "/next_cursor")?;
        let last = ada
            .send(
                "GET",
                &format!("/api/v1/auth/invitations?limit=2&cursor={cursor}"),
                None,
            )
            .await?;
        assert_eq!(last.body["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(last.body["next_cursor"], Value::Null);
        assert!(
            last.body["items"][0]["expires_at"].is_string(),
            "the route's default holds every field"
        );
        // A field that does not exist, a cursor that is not one or was changed, and a page size
        // that is no number are refused, each with its own code.
        let mut changed = cursor.clone();
        changed.pop();
        changed.push(if cursor.ends_with('A') { 'B' } else { 'A' });
        for (bad, code) in [
            ("fields=password".to_owned(), "request.unknown_field"),
            ("cursor=nonsense".to_owned(), "request.bad_cursor"),
            (format!("cursor={changed}"), "request.bad_cursor"),
            ("limit=many".to_owned(), "request.invalid"),
        ] {
            let reply = ada
                .send("GET", &format!("/api/v1/auth/invitations?{bad}"), None)
                .await?;
            assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{bad}");
            assert_eq!(reply.code(), code, "{bad}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_a_password_alone_shows_nothing_about_the_person_until_the_second_step_is_done()
-> Outcome {
    each_database("sec_12_http_hidden_person", async |db| {
        let (rig, app) = rig_and_app(db, 10).await?;
        let used = called();
        let mut ada = Browser::new(&app, &used);
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        ada.send(
            "POST",
            "/api/v1/auth/two-step/code-app",
            Some(json!({"password": PASSWORD})),
        )
        .await?;
        let confirmed = ada
            .send(
                "POST",
                "/api/v1/auth/two-step/code-app/confirm",
                Some(json!({"code": totp(&rig, "ada@example.test").await?, "password": PASSWORD})),
            )
            .await?;
        assert!(ok(&confirmed));
        rig.clock.advance(30);
        let mut laptop = Browser::new(&app, &used);
        let password_only = laptop
            .send(
                "POST",
                "/api/v1/auth/sign-in",
                Some(sign_in_body("ada@example.test", PASSWORD)),
            )
            .await?;
        assert_eq!(password_only.status, StatusCode::OK);
        assert_eq!(password_only.body["person"], Value::Null);
        assert!(password_only.body["second_step"].is_object());
        let shown = password_only.body.to_string();
        for private in ["Ada Admin", "ada@example.test", "administrator"] {
            assert!(!shown.contains(private), "{private}");
        }
        let finished = laptop
            .send(
                "POST",
                "/api/v1/auth/second-step",
                Some(json!({"code": totp(&rig, "ada@example.test").await?})),
            )
            .await?;
        assert_eq!(finished.body["person"]["email"], "ada@example.test");
        assert_eq!(finished.body["second_step"], Value::Null);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_02_the_lists_with_a_hard_cap_say_whether_they_were_cut() -> Outcome {
    each_database("perf_02_http_capped_lists", async |db| {
        let (rig, app) = rig_and_app(db, 10).await?;
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let sessions = ada.send("GET", "/api/v1/auth/sessions", None).await?;
        assert_eq!(sessions.body["truncated"], false);
        let two_step = ada.send("GET", "/api/v1/auth/two-step", None).await?;
        assert_eq!(two_step.body["truncated"], false);
        assert!(two_step.body["passkeys"].is_array());
        Ok(())
    })
    .await
}
