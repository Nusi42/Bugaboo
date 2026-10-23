//! Signing in over HTTP: the cookies, the token against forged requests (SEC-12), the limits
//! (SEC-14), the answers that never say whether an account exists, and every route once.

use bb_app::auth::SecondStepStore as _;
use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_api::cookies::{CSRF_HEADER, Cookie};
use bb_api::openapi::operations;
use bb_api::permissions::{Answer, Declared};
use bb_api::{AuthHandle, ClientAddr, LimitsConfig, Options, router};
use bb_app::context::Clock as _;
use bb_auth::TotpSecret;
use bb_testing::{Failure, TestDatabase, TestResult, each_database};
use serde_json::{Value, json};
use tower::ServiceExt as _;
use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator as _};
use webauthn_authenticator_rs::softpasskey::SoftPasskey;

use super::rig::{OTHER_PASSWORD, PASSWORD, PUBLIC_URL, Rig, Tries};

mod cookies;
mod lists;
mod tour;

/// The third number of the next browser's address (a network is the first three numbers).
static NEXT_HOST: AtomicU8 = AtomicU8::new(1);

/// The names of the cookies of a session, from the one cookie table of `bb-api`.
pub(super) const SESSION: &str = Cookie::Session.name();
pub(super) const CSRF: &str = Cookie::Csrf.name();

/// What the server answered.
pub(crate) struct Reply {
    pub(crate) status: StatusCode,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Value,
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

    pub(super) fn cookies(&self) -> Vec<String> {
        self.headers
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .map(str::to_owned)
            .collect()
    }
}

/// Whether an answer says it is JSON (a file is sent as bytes, which are not).
fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|kind| kind.contains("json"))
}

/// What the server answered with a file: the body as it is, not JSON.
pub(crate) struct RawReply {
    pub(crate) status: StatusCode,
    pub(crate) headers: HeaderMap,
    pub(crate) bytes: Vec<u8>,
}

impl RawReply {
    pub(crate) fn header(&self, name: &str) -> &str {
        self.headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
    }
}

/// How a request carries the token of the session.
#[derive(Clone, Copy)]
enum Token<'a> {
    /// Like the generated client: the value of the `__Host-bb_csrf` cookie, if there is one.
    FromCookie,
    /// No token at all.
    Nothing,
    /// This value.
    Is(&'a str),
}

/// A browser: it keeps cookies and sends the token like the app does.
pub(crate) struct Browser {
    app: Router,
    jar: BTreeMap<String, String>,
    called: Arc<Mutex<BTreeSet<String>>>,
    peer: SocketAddr,
    /// Headers every request of this browser carries besides the usual ones.
    headers: Vec<(&'static str, String)>,
}

/// The options of a router that signs people in with the services of `rig`.
pub(super) fn options(rig: &Rig, limits: LimitsConfig) -> Options {
    Options {
        limits,
        auth: AuthHandle::new(rig.services.clone()),
        ..Options::for_tests()
    }
}

impl Browser {
    pub(crate) fn new(app: &Router, called: &Arc<Mutex<BTreeSet<String>>>) -> Self {
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

    /// A browser that made the first administrator (Ada) with the setup code of `rig` and is signed
    /// in as her.
    pub(super) async fn signed_in(app: &Router, rig: &Rig) -> Result<Self, Failure> {
        let mut browser = Self::new(app, &called());
        browser.set_up(rig).await?;
        Ok(browser)
    }

    /// A browser of the person with this address (and the password of the tests), signed in.
    pub(super) async fn signed_in_as(app: &Router, email: &str) -> Result<Self, Failure> {
        let mut browser = Self::new(app, &called());
        let reply = browser.sign_in(email, PASSWORD).await?;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        Ok(browser)
    }

    /// Creates the first administrator (Ada) with the setup code of `rig`; this browser is signed in
    /// as her, and the answer is returned.
    pub(super) async fn set_up(&mut self, rig: &Rig) -> Result<Reply, Failure> {
        self.send(
            "POST",
            "/api/v1/auth/setup",
            Some(setup_body(&rig.setup_code)),
        )
        .await
    }

    /// Signs in with e-mail address and password, as the sign-in page does; the answer is returned.
    pub(super) async fn sign_in(&mut self, email: &str, password: &str) -> Result<Reply, Failure> {
        self.send(
            "POST",
            "/api/v1/auth/sign-in",
            Some(sign_in_body(email, password)),
        )
        .await
    }

    /// Invites the person with this address and name (this browser's person must be allowed to),
    /// and returns the browser of the person who accepted: signed in, not an administrator.
    pub(super) async fn invite(
        &mut self,
        email: &str,
        display_name: &str,
    ) -> Result<Self, Failure> {
        let invitation = self
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
        let mut person = Self::new(&self.app, &self.called);
        let accepted = person
            .send(
                "POST",
                "/api/v1/auth/invitations/accept",
                Some(json!({"secret": secret, "display_name": display_name, "password": PASSWORD})),
            )
            .await?;
        assert_eq!(accepted.status, StatusCode::OK, "{}", accepted.body);
        Ok(person)
    }

    /// From now on every request carries this header (`If-None-Match`, `Accept-Encoding`).
    pub(super) fn set_header(&mut self, name: &'static str, value: &str) {
        self.headers.retain(|(known, _)| *known != name);
        self.headers.push((name, value.to_owned()));
    }

    /// The browser holds this cookie, as if it had been set by an earlier answer.
    pub(super) fn set_cookie(&mut self, name: &str, value: &str) {
        self.jar.insert(name.to_owned(), value.to_owned());
    }

    /// From now on no extra headers.
    pub(super) fn clear_headers(&mut self) {
        self.headers.clear();
    }

    pub(crate) async fn send(
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
        let request = self.request_to(method, path, token, origin);
        self.finish(request, method, path, body).await
    }

    /// A request of this browser: its headers, its cookies and its token, without a body.
    fn request_to(
        &self,
        method: &str,
        path: &str,
        token: Token<'_>,
        origin: &str,
    ) -> axum::http::request::Builder {
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
                    request = request.header(CSRF_HEADER, value);
                }
            }
            Token::Nothing => {}
            Token::Is(value) => request = request.header(CSRF_HEADER, value),
        }
        request
    }

    /// Sends bytes as they are (a part of an upload) and reads the JSON answer.
    pub(crate) async fn send_bytes(
        &mut self,
        method: &str,
        path: &str,
        bytes: Vec<u8>,
    ) -> Result<Reply, Failure> {
        let request = self
            .request_to(method, path, Token::FromCookie, "http://bugaboo.test")
            .header(header::CONTENT_TYPE, "application/octet-stream");
        let (status, headers, body) = Box::pin(self.exchange(request, Body::from(bytes))).await?;
        Self::reply_of(method, path, status, headers, &body)
    }

    /// Fetches an address and keeps the answer as it is: the bytes of a file, not JSON.
    pub(crate) async fn fetch(&mut self, path: &str) -> Result<RawReply, Failure> {
        let request = self.request_to("GET", path, Token::FromCookie, "http://bugaboo.test");
        let (status, headers, bytes) = Box::pin(self.exchange(request, Body::empty())).await?;
        // A file is never kept by a cache.
        if status == StatusCode::OK {
            assert_eq!(
                headers
                    .get(header::CACHE_CONTROL)
                    .and_then(|value| value.to_str().ok()),
                Some("no-store"),
                "GET {path}"
            );
        }
        Ok(RawReply {
            status,
            headers,
            bytes: bytes.to_vec(),
        })
    }

    /// Sends a request and keeps the cookies the answer sets.
    async fn exchange(
        &mut self,
        request: axum::http::request::Builder,
        body: Body,
    ) -> Result<(StatusCode, HeaderMap, axum::body::Bytes), Failure> {
        let mut request = request.body(body)?;
        request
            .extensions_mut()
            .insert(ConnectInfo(ClientAddr(self.peer)));
        let response = self.app.clone().oneshot(request).await?;
        let (parts, body) = response.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX).await?;
        for cookie in parts
            .headers
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
        {
            let pair = cookie.split(';').next().unwrap_or_default();
            if let Some((name, value)) = pair.split_once('=') {
                if cookie.contains("Max-Age=0") {
                    self.jar.remove(name);
                } else {
                    self.jar.insert(name.to_owned(), value.to_owned());
                }
            }
        }
        Ok((parts.status, parts.headers, bytes))
    }

    /// The JSON answer, after the check of what it says about keeping it.
    fn reply_of(
        method: &str,
        path: &str,
        status: StatusCode,
        headers: HeaderMap,
        bytes: &[u8],
    ) -> Result<Reply, Failure> {
        let json = is_json(&headers);
        let reply = Reply {
            status,
            headers,
            body: if bytes.is_empty() || !json {
                Value::Null
            } else {
                serde_json::from_slice(bytes)?
            },
        };
        assert_cache_rules(method, path, &reply);
        Ok(reply)
    }

    /// A change that carries no token of the session (a page that forgot it, or a forger).
    pub(super) async fn send_without_token(
        &mut self,
        method: &str,
        path: &str,
        body: Value,
    ) -> Result<Reply, Failure> {
        self.send_as(
            method,
            path,
            Some(body),
            Token::Nothing,
            "http://bugaboo.test",
        )
        .await
    }

    /// A script with an access key: no cookie, no token and no `Origin` (a program that is not
    /// a web page sends none of them), only `Authorization: Bearer <key>`.
    pub(super) async fn script(
        &mut self,
        method: &str,
        path: &str,
        body: Option<Value>,
        key: &str,
    ) -> Result<Reply, Failure> {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::HOST, "bugaboo.test")
            .header(header::AUTHORIZATION, format!("Bearer {key}"))
            .header(header::USER_AGENT, "curl/8.5.0");
        self.finish(request, method, path, body).await
    }

    /// Sends the request with its body, keeps the cookies the answer sets and checks what every
    /// answer says about keeping it.
    pub(super) async fn finish(
        &mut self,
        mut request: axum::http::request::Builder,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Reply, Failure> {
        let body = match body {
            Some(value) => {
                request = request.header(header::CONTENT_TYPE, "application/json");
                Body::from(serde_json::to_vec(&value)?)
            }
            None => Body::empty(),
        };
        let (status, headers, bytes) = Box::pin(self.exchange(request, body)).await?;
        Self::reply_of(method, path, status, headers, &bytes)
    }
}

/// The `GET` routes whose answers hold personal data, as the route table says them.
pub(super) fn never_stored_reads() -> &'static BTreeSet<String> {
    static ROUTES: std::sync::OnceLock<BTreeSet<String>> = std::sync::OnceLock::new();
    ROUTES.get_or_init(|| {
        operations()
            .iter()
            .filter(|operation| {
                operation.method == Method::GET && operation.kind == Answer::NeverStored
            })
            .map(|operation| operation.path.clone())
            .collect()
    })
}

/// The ID that stands in for any `{…}` part of a route, when only the layers in front of the route
/// are of interest.
const ANY_ID: &str = "0197a5e2-7c1f-4b0a-9a3e-2f6d1c8b4a10";

/// The address of a route written as `template`, with an ID in every `{…}` part (a setting key in
/// `{key}`).
pub(super) fn example_path(template: &str) -> String {
    template
        .split('/')
        .map(|part| match part {
            "{key}" => "some.key",
            part if part.starts_with('{') => ANY_ID,
            part => part,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Whether the address is the route written as `template`, whose `{…}` parts stand for any one
/// part of an address.
fn is_route(template: &str, route: &str) -> bool {
    let (template, route): (Vec<&str>, Vec<&str>) =
        (template.split('/').collect(), route.split('/').collect());
    template.len() == route.len()
        && template
            .iter()
            .zip(&route)
            .all(|(wanted, found)| wanted == found || wanted.starts_with('{'))
}

/// What every answer of the API says about keeping it (PERF-03, `bb-api` `etag`), checked on
/// every request of every test that goes through a [`Browser`]: a read of something that is not
/// personal data may be kept and must be asked about again, and has a tag; everything else is
/// never stored and has no tag.
fn assert_cache_rules(method: &str, path: &str, reply: &Reply) {
    let route = path.split('?').next().unwrap_or(path);
    let kept = method == "GET"
        && reply.status == StatusCode::OK
        && !never_stored_reads()
            .iter()
            .any(|template| is_route(template, route));
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

/// The address of the first administrator (Ada), whom the setup call makes.
pub(crate) const ADA_EMAIL: &str = "ada@example.test";

/// The body that makes the first administrator with this setup code.
pub(crate) fn setup_body(code: &str) -> Value {
    json!({
        "code": code,
        "display_name": "Ada Admin",
        "email": ADA_EMAIL,
        "password": PASSWORD,
    })
}

/// A rig with the default number of tries and its router.
pub(super) async fn rig_and_app(db: &TestDatabase) -> Result<(Rig, Router), Failure> {
    rig_and_app_with(db, Tries::Default).await
}

/// A rig that allows `tries` failed tries, and its router.
pub(super) async fn rig_and_app_with(
    db: &TestDatabase,
    tries: Tries,
) -> Result<(Rig, Router), Failure> {
    let rig = Rig::start_with(db, tries).await?;
    let app = router(options(&rig, LimitsConfig::default()));
    Ok((rig, app))
}

pub(crate) fn called() -> Arc<Mutex<BTreeSet<String>>> {
    Arc::new(Mutex::new(BTreeSet::new()))
}

pub(super) fn sign_in_body(email: &str, password: &str) -> Value {
    json!({"email": email, "password": password})
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
