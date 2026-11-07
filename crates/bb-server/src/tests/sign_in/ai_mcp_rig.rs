//! What the tests of AI apps over HTTP share: a router with the MCP server mounted, requests of an app
//! (no cookie, no `Origin`, only a bearer token), and the steps of signing in with OAuth as an app
//! does them (AI-20 to AI-23, ADR-0042).

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use bb_api::{LimitsConfig, router};
use bb_auth::pkce_challenge;
use bb_core::oauth::form_pairs;
use bb_testing::{Failure, TestDatabase};
use serde_json::{Value, json};
use tower::ServiceExt as _;

use super::http::{Browser, options};
use super::rig::Rig;

/// The proof of the PKCE challenge every test app sends.
const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
/// The address the test apps come back to; the port is one the app picked.
const RETURN_AT: &str = "http://127.0.0.1:51734/callback";
/// The address the test apps register.
const REGISTERED: &str = "http://127.0.0.1/callback";

/// An answer of the server to an app.
pub(super) struct Answer {
    pub(super) status: StatusCode,
    headers: HeaderMap,
    pub(super) json: Value,
}

impl Answer {
    /// A header of the answer, empty when there is none.
    pub(super) fn header(&self, name: &str) -> &str {
        self.headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
    }
}

/// An app that signed in.
pub(super) struct Signed {
    pub(super) client: String,
    pub(super) access: String,
    pub(super) refresh: String,
}

/// The requests of an app to a router.
pub(super) struct Mcp<'a> {
    app: &'a Router,
}

/// The organization has AI on (a model on this computer needs no agreement), so apps may sign in.
pub(super) async fn ai_on(rig: &Rig) -> Result<(), Failure> {
    rig.set_organization_switch("ai.enabled", true).await
}

impl<'a> Mcp<'a> {
    /// A rig and a router with the MCP server mounted.
    pub(super) async fn start(db: &TestDatabase) -> Result<(Rig, Router), Failure> {
        let rig = Rig::start(db).await?;
        let mcp = bb_mcp::router(bb_mcp::McpOptions {
            api: rig.services.clone(),
        });
        let app = router(bb_api::Options {
            mcp: Some(mcp),
            ..options(&rig, LimitsConfig::default())
        });
        Ok((rig, app))
    }

    pub(super) const fn new(app: &'a Router) -> Self {
        Self { app }
    }

    async fn send(&self, request: Request<Body>) -> Result<Answer, Failure> {
        let response = self.app.clone().oneshot(request).await?;
        let (parts, body) = response.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX).await?;
        Ok(Answer {
            status: parts.status,
            headers: parts.headers,
            json: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        })
    }

    /// A `GET` with no credentials.
    pub(super) async fn get(&self, path: &str) -> Result<Answer, Failure> {
        self.get_with(path, &[]).await
    }

    /// A `GET` with these headers.
    pub(super) async fn get_with(
        &self,
        path: &str,
        headers: &[(&str, &str)],
    ) -> Result<Answer, Failure> {
        let mut request = Request::builder()
            .method(Method::GET)
            .uri(path)
            .header(header::HOST, "bugaboo.test");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        self.send(request.body(Body::empty())?).await
    }

    /// A `POST` of JSON with no credentials.
    pub(super) async fn post_json(&self, path: &str, body: Value) -> Result<Answer, Failure> {
        let request = Request::builder()
            .method(Method::POST)
            .uri(path)
            .header(header::HOST, "bugaboo.test")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&body)?))?;
        self.send(request).await
    }

    /// A `POST` of a form, as the token endpoint takes it.
    pub(super) async fn post_form(&self, path: &str, body: &str) -> Result<Answer, Failure> {
        let request = Request::builder()
            .method(Method::POST)
            .uri(path)
            .header(header::HOST, "bugaboo.test")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(body.to_owned()))?;
        self.send(request).await
    }

    /// A message to `/mcp`, with the token of an app when there is one.
    pub(super) async fn post(&self, token: Option<&str>, body: Value) -> Result<Answer, Failure> {
        self.post_with(token, &[], body).await
    }

    /// A message to `/mcp` with more headers, such as the `Origin` of a browser page.
    pub(super) async fn post_with(
        &self,
        token: Option<&str>,
        headers: &[(&str, &str)],
        body: Value,
    ) -> Result<Answer, Failure> {
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header(header::HOST, "bugaboo.test")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json, text/event-stream");
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        self.send(request.body(Body::from(serde_json::to_vec(&body)?))?)
            .await
    }

    /// A call of a tool.
    pub(super) async fn call_tool(
        &self,
        signed: &Signed,
        tool: &str,
        arguments: Value,
    ) -> Result<Answer, Failure> {
        self.post(
            Some(&signed.access),
            json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "tools/call",
                "params": { "name": tool, "arguments": arguments },
            }),
        )
        .await
    }
}

/// An app registers itself, the person (who is signed in on `person`) allows it with `given`
/// although it asked for `asked`, and the app swaps its code for tokens.
pub(super) async fn sign_in_app(
    mcp: &Mcp<'_>,
    person: &mut Browser,
    name: &str,
    asked: &str,
    given: &str,
) -> Result<Signed, Failure> {
    sign_in_app_in(mcp, person, name, asked, given, &[]).await
}

/// [`sign_in_app`] for an app the person limits to these projects (by ID).
pub(super) async fn sign_in_app_in(
    mcp: &Mcp<'_>,
    person: &mut Browser,
    name: &str,
    asked: &str,
    given: &str,
    projects: &[String],
) -> Result<Signed, Failure> {
    let registered = mcp
        .post_json(
            "/oauth/register",
            json!({"client_name": name, "redirect_uris": [REGISTERED]}),
        )
        .await?;
    if registered.status != StatusCode::CREATED {
        return Err(format!("registration: {}", registered.json).into());
    }
    let client = registered.json["client_id"]
        .as_str()
        .ok_or("no client ID")?
        .to_owned();
    let request = json!({
        "client": client,
        "response_type": "code",
        "redirect_uri": RETURN_AT,
        "code_challenge": pkce_challenge(VERIFIER),
        "code_challenge_method": "S256",
        "scope": asked,
        "state": "state-1",
    });
    let shown = person
        .send(
            "GET",
            &format!(
                "/api/v1/oauth/authorization?client={client}&response_type=code&redirect_uri={}&code_challenge={}&code_challenge_method=S256&scope={asked}&state=state-1",
                bb_core::oauth::percent_encode(RETURN_AT),
                pkce_challenge(VERIFIER)
            ),
            None,
        )
        .await?;
    if shown.body["app_name"] != json!(name) {
        return Err(format!("the consent step: {}", shown.body).into());
    }
    let decided = person
        .send(
            "POST",
            "/api/v1/oauth/authorization",
            Some(json!({"request": request, "allow": true, "scope": given, "projects": projects})),
        )
        .await?;
    let redirect = decided.body["redirect_to"]
        .as_str()
        .ok_or_else(|| format!("no redirect: {}", decided.body))?
        .to_owned();
    let query = redirect.split_once('?').map_or("", |(_, rest)| rest);
    let pairs = form_pairs(query).ok_or("a broken answer")?;
    let code = pairs
        .iter()
        .find(|(name, _)| name == "code")
        .map(|(_, code)| code.clone())
        .ok_or("no code")?;
    let tokens = mcp
        .post_form(
            "/oauth/token",
            &format!(
                "grant_type=authorization_code&client_id={client}&code={code}&redirect_uri={}&code_verifier={VERIFIER}",
                bb_core::oauth::percent_encode(RETURN_AT)
            ),
        )
        .await?;
    if tokens.status != StatusCode::OK {
        return Err(format!("the token: {}", tokens.json).into());
    }
    Ok(Signed {
        client,
        access: tokens.json["access_token"]
            .as_str()
            .ok_or("no token")?
            .to_owned(),
        refresh: tokens.json["refresh_token"]
            .as_str()
            .ok_or("no token")?
            .to_owned(),
    })
}
