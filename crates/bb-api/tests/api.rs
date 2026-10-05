//! The API skeleton end to end: the OpenAPI contract, problem+json answers, rate limits (SEC-14)
//! and the rules every API answer follows.

// Dependencies of the library that this test does not use itself (`unused_crate_dependencies`).
use base64ct as _;
use bb_app as _;
use bb_auth as _;
use bb_core as _;
use bb_policy as _;
use brotli as _;
use getrandom as _;
use h2 as _;
use hmac as _;
use hyper_util as _;
use mime_guess as _;
use proptest as _;
use rcgen as _;
use rust_embed as _;
use rustls as _;
use serde as _;
use sha2 as _;
use tempfile as _;
use tokio_rustls as _;
use tower_http as _;
use tracing as _;
use tracing_subscriber as _;
use utoipa as _;
use utoipa_axum as _;

#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod api {
    use std::net::SocketAddr;
    use std::time::Duration;

    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode, header};
    use axum::response::Response;
    use bb_api::cookies::Cookie;
    use bb_api::{ClientAddr, LimitsConfig, Options, WebSource, openapi, router};
    use tower::ServiceExt as _;

    fn options(limits: LimitsConfig, proxies: &[&str]) -> Options {
        Options {
            web: WebSource::Folder("no/such/folder".into()),
            hsts_max_age: None,
            header_timeout: Duration::from_secs(30),
            data_folder: std::env::temp_dir(),
            trusted_proxies: proxies.iter().map(|p| p.parse().unwrap()).collect(),
            limits,
            auth: bb_api::AuthHandle::none(),
            probe: bb_api::ProbeHandle::none(),
        }
    }

    fn request(method: &str, path: &str, peer: &str, forwarded: Option<&str>) -> Request<Body> {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header("sec-fetch-site", "same-origin");
        if let Some(forwarded) = forwarded {
            request = request.header("x-forwarded-for", forwarded);
        }
        let mut request = request.body(Body::empty()).unwrap();
        let peer: SocketAddr = peer.parse().unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(ClientAddr(peer)));
        request
    }

    async fn json(response: Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn header_of<'a>(response: &'a Response, name: &str) -> &'a str {
        response.headers().get(name).unwrap().to_str().unwrap()
    }

    #[tokio::test]
    async fn adr_0008_the_served_spec_is_the_one_in_the_code_and_the_one_on_disk() {
        let app = router(options(LimitsConfig::default(), &[]));
        let response = app
            .oneshot(request(
                "GET",
                "/api/v1/openapi.json",
                "198.51.100.1:1",
                None,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(header_of(&response, "content-type"), "application/json");
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let served: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let in_code: serde_json::Value = serde_json::from_str(openapi::json()).unwrap();
        assert_eq!(served, in_code);

        // The file is formatted by the project's formatter, so it is compared as data.
        let on_disk = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../web/packages/api-client/openapi.json");
        let on_disk: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&on_disk).unwrap()).unwrap();
        assert_eq!(
            on_disk, served,
            "web/packages/api-client/openapi.json is out of date; run `cargo xtask gen api`"
        );
    }

    #[tokio::test]
    async fn adr_0008_the_spec_describes_the_error_body_and_every_route_the_standard_errors() {
        let spec: serde_json::Value = serde_json::from_str(openapi::json()).unwrap();
        assert_eq!(spec["openapi"], "3.1.0");
        let problem = &spec["components"]["schemas"]["Problem"];
        for field in ["type", "title", "status", "code", "message_key", "params"] {
            assert!(problem["properties"][field].is_object(), "{field}");
        }
        let codes = spec["components"]["schemas"]["ErrorCode"]["enum"]
            .as_array()
            .unwrap();
        assert!(codes.iter().any(|code| code == "rate_limit.exceeded"));
        for (path, item) in spec["paths"].as_object().unwrap() {
            for (method, operation) in item.as_object().unwrap() {
                for status in ["429", "500", "503"] {
                    let content = &operation["responses"][status]["content"];
                    assert!(
                        content["application/problem+json"].is_object(),
                        "{method} {path} {status}"
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn adr_0002_the_example_route_answers_json_and_json_is_never_compressed() {
        let app = router(options(LimitsConfig::default(), &[]));
        let mut get = request("GET", "/api/v1/status", "198.51.100.1:1", None);
        get.headers_mut()
            .insert(header::ACCEPT_ENCODING, "gzip, br".parse().unwrap());
        let response = app.oneshot(get).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(header_of(&response, "content-type"), "application/json");
        assert!(response.headers().get(header::CONTENT_ENCODING).is_none());
        assert_eq!(json(response).await, serde_json::json!({"status": "ok"}));
    }

    #[tokio::test]
    async fn ux_07_errors_are_problem_json_with_the_request_id_of_the_response() {
        let app = router(options(LimitsConfig::default(), &[]));
        let missing = app
            .clone()
            .oneshot(request("GET", "/api/v1/nothing", "198.51.100.1:1", None))
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            header_of(&missing, "content-type"),
            "application/problem+json"
        );
        let id = header_of(&missing, "x-request-id").to_owned();
        let body = json(missing).await;
        assert_eq!(body["request_id"], id.as_str());
        assert_eq!(body["code"], "resource.not_found");
        assert_eq!(body["message_key"], "error.resource.not-found");

        let wrong_method = app
            .oneshot(request("POST", "/api/v1/status", "198.51.100.1:1", None))
            .await
            .unwrap();
        assert_eq!(wrong_method.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert!(wrong_method.headers().contains_key(header::ALLOW));
        let body = json(wrong_method).await;
        assert_eq!(body["code"], "request.method_not_allowed");
        assert!(body["request_id"].is_string());
    }

    #[tokio::test]
    async fn sec_06_a_change_from_another_site_is_refused_as_problem_json() {
        let app = router(options(LimitsConfig::default(), &[]));
        let mut cross = request("POST", "/api/v1/status", "198.51.100.1:1", None);
        cross.headers_mut().remove("sec-fetch-site");
        cross
            .headers_mut()
            .insert(header::ORIGIN, "https://evil.test".parse().unwrap());
        cross
            .headers_mut()
            .insert(header::HOST, "bugaboo.test".parse().unwrap());
        let response = app.oneshot(cross).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            header_of(&response, "content-type"),
            "application/problem+json"
        );
        let body = json(response).await;
        assert_eq!(body["code"], "request.wrong_origin");
        assert!(body["request_id"].is_string());
    }

    #[tokio::test]
    async fn sec_14_a_client_over_the_limit_gets_429_with_retry_after_and_a_message_key() {
        let limits = LimitsConfig {
            requests_per_minute: 3,
            sign_in_attempts_per_minute: 3,
            ..LimitsConfig::default()
        };
        let app = router(options(limits, &[]));
        for _ in 0..3 {
            let response = app
                .clone()
                .oneshot(request("GET", "/api/v1/status", "198.51.100.1:1", None))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
        let refused = app
            .clone()
            .oneshot(request("GET", "/api/v1/status", "198.51.100.1:2", None))
            .await
            .unwrap();
        assert_eq!(refused.status(), StatusCode::TOO_MANY_REQUESTS);
        let wait: u64 = header_of(&refused, "retry-after").parse().unwrap();
        assert!((1..=60).contains(&wait), "{wait}");
        assert_eq!(
            header_of(&refused, "content-type"),
            "application/problem+json"
        );
        let body = json(refused).await;
        assert_eq!(body["code"], "rate_limit.exceeded");
        assert_eq!(body["message_key"], "error.rate-limit.exceeded");

        // Another client is not affected; unknown API addresses count too; pages do not.
        let other = app
            .clone()
            .oneshot(request("GET", "/api/v1/status", "198.51.100.9:1", None))
            .await
            .unwrap();
        assert_eq!(other.status(), StatusCode::OK);
        let unknown = app
            .clone()
            .oneshot(request("GET", "/api/anything", "198.51.100.1:3", None))
            .await
            .unwrap();
        assert_eq!(unknown.status(), StatusCode::TOO_MANY_REQUESTS);
        let page = app
            .oneshot(request("GET", "/health/live", "198.51.100.1:4", None))
            .await
            .unwrap();
        assert_eq!(page.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn sec_14_the_limit_follows_the_canonical_client_not_a_header_anyone_can_send() {
        let limits = LimitsConfig {
            requests_per_minute: 1,
            sign_in_attempts_per_minute: 1,
            ..LimitsConfig::default()
        };
        let app = router(options(limits, &["10.0.0.5"]));
        // An ordinary client that invents a different forwarded address each time is still one
        // client.
        let first = app
            .clone()
            .oneshot(request(
                "GET",
                "/api/v1/status",
                "198.51.100.1:1",
                Some("1.1.1.1"),
            ))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        let second = app
            .clone()
            .oneshot(request(
                "GET",
                "/api/v1/status",
                "198.51.100.1:1",
                Some("2.2.2.2"),
            ))
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        // Behind the trusted proxy, the address the proxy saw (the last entry) is the client,
        // whatever the client claimed before it.
        let a = app
            .clone()
            .oneshot(request(
                "GET",
                "/api/v1/status",
                "10.0.0.5:1",
                Some("9.9.9.9, 203.0.113.1"),
            ))
            .await
            .unwrap();
        assert_eq!(a.status(), StatusCode::OK);
        let b = app
            .clone()
            .oneshot(request(
                "GET",
                "/api/v1/status",
                "10.0.0.5:2",
                Some("8.8.8.8, 203.0.113.2"),
            ))
            .await
            .unwrap();
        assert_eq!(b.status(), StatusCode::OK);
        let again = app
            .oneshot(request(
                "GET",
                "/api/v1/status",
                "10.0.0.5:3",
                Some("7.7.7.7, 203.0.113.1"),
            ))
            .await
            .unwrap();
        assert_eq!(again.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn comp_13_the_cookies_have_the_attributes_the_docs_promise() {
        let session = Cookie::Session
            .set("token", Some(Duration::from_mins(1)))
            .unwrap();
        let session = session.to_str().unwrap();
        for attribute in [
            "__Host-bb_session=token",
            "Path=/",
            "Secure",
            "HttpOnly",
            "SameSite=Lax",
        ] {
            assert!(session.contains(attribute), "{session}");
        }
        assert!(!session.contains("Domain"), "{session}");
        let csrf = Cookie::Csrf.set("token", None).unwrap();
        let csrf = csrf.to_str().unwrap();
        for attribute in [
            "__Host-bb_csrf=token",
            "Path=/",
            "Secure",
            "SameSite=Strict",
        ] {
            assert!(csrf.contains(attribute), "{csrf}");
        }
        assert!(!csrf.contains("HttpOnly"), "{csrf}");
    }
}
