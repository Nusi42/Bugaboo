//! SEC-10 and SEC-11 over a real connection: HTTPS with a generated certificate, the full header
//! set including HSTS, compression, and the plain-HTTP rules for trusted proxies.

// Dependencies of the library that this test does not use itself (`unused_crate_dependencies`).
use base64ct as _;
use bb_api_words as _;
use bb_app as _;
use bb_auth as _;
use bb_core as _;
use bb_policy as _;
use brotli as _;
use getrandom as _;
use hmac as _;
use hyper_util as _;
use mime_guess as _;
use proptest as _;
use rcgen as _;
use rust_embed as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use tower as _;
use tower_http as _;
use tracing as _;
use tracing_subscriber as _;
use utoipa as _;
use utoipa_axum as _;

#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod https {
    use std::net::{IpAddr, SocketAddr};
    use std::sync::Arc;
    use std::time::Duration;

    use axum::http::{Request, StatusCode};
    use bb_api::tls::{TlsSource, server_config};
    use bb_api::{Incoming, ListenerOptions, Options, WebSource, serve};
    use rustls::pki_types::pem::PemObject as _;
    use rustls::pki_types::{CertificateDer, ServerName};
    use rustls::{ClientConfig, RootCertStore};
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::oneshot;
    use tokio_rustls::TlsConnector;

    use bb_api::{HSTS_NAME, SECURITY_HEADERS, hsts_value};

    const LOOPBACK: &str = "127.0.0.1:0";

    struct Running {
        address: SocketAddr,
        stop: Option<oneshot::Sender<()>>,
        tls_folder: tempfile::TempDir,
        _app: tempfile::TempDir,
    }

    impl Drop for Running {
        fn drop(&mut self) {
            if let Some(stop) = self.stop.take() {
                let _ = stop.send(());
            }
        }
    }

    async fn start(tls: bool, only_from: Option<Vec<IpAddr>>) -> Running {
        start_with(tls, only_from, 1024).await
    }

    async fn start_with(
        tls: bool,
        only_from: Option<Vec<IpAddr>>,
        max_connections: usize,
    ) -> Running {
        let app = tempfile::tempdir().unwrap();
        let big = "<p>Bugaboo keeps work in one calm place.</p>".repeat(100);
        std::fs::write(app.path().join("index.html"), big).unwrap();
        let tls_folder = tempfile::tempdir().unwrap();
        let config = tls.then(|| {
            server_config(&TlsSource::SelfSigned {
                folder: tls_folder.path().to_path_buf(),
                names: vec![],
            })
            .unwrap()
        });
        let behind_proxy = only_from.is_some();
        let tcp = TcpListener::bind(LOOPBACK).await.unwrap();
        let address = tcp.local_addr().unwrap();
        let incoming = Incoming::start(
            tcp,
            ListenerOptions {
                tls: config,
                only_from,
                max_connections,
            },
        );
        let options = Options {
            web: WebSource::Folder(app.path().to_path_buf()),
            hsts_max_age: if tls {
                Some(bb_api::HSTS_MAX_AGE_SELF_SIGNED)
            } else {
                behind_proxy.then_some(bb_api::HSTS_MAX_AGE_TRUSTED)
            },
            header_timeout: Duration::from_secs(1),
            data_folder: tls_folder.path().to_path_buf(),
            ..Options::for_tests()
        };
        let (stop, stopped) = oneshot::channel::<()>();
        tokio::spawn(serve(incoming, options, async move {
            let _ = stopped.await;
        }));
        Running {
            address,
            stop: Some(stop),
            tls_folder,
            _app: app,
        }
    }

    /// Sends one request and returns the raw response bytes.
    async fn fetch(server: &Running, tls: bool, request: &str) -> Vec<u8> {
        let tcp = TcpStream::connect(server.address).await.unwrap();
        let mut out = Vec::new();
        if tls {
            let mut roots = RootCertStore::empty();
            let cert = CertificateDer::from_pem_file(
                server.tls_folder.path().join("self-signed-cert.pem"),
            )
            .unwrap();
            roots.add(cert).unwrap();
            let client = ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(roots)
            .with_no_client_auth();
            let name = ServerName::try_from("localhost").unwrap();
            let mut stream = TlsConnector::from(Arc::new(client))
                .connect(name, tcp)
                .await
                .unwrap();
            stream.write_all(request.as_bytes()).await.unwrap();
            let _ = stream.read_to_end(&mut out).await;
        } else {
            let mut stream = tcp;
            stream.write_all(request.as_bytes()).await.unwrap();
            let _ = stream.read_to_end(&mut out).await;
        }
        out
    }

    fn get(path: &str, extra: &str) -> String {
        format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n{extra}\r\n")
    }

    fn head_of(response: &[u8]) -> String {
        let end = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap_or(response.len());
        String::from_utf8_lossy(&response[..end]).to_lowercase()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sec_10_https_with_a_generated_certificate_sends_the_full_header_set() {
        let server = start(true, None).await;
        for path in ["/", "/api/x", "/missing.js"] {
            let response = fetch(&server, true, &get(path, "")).await;
            let head = head_of(&response);
            let hsts = hsts_value(bb_api::HSTS_MAX_AGE_SELF_SIGNED);
            for (name, value) in SECURITY_HEADERS
                .iter()
                .copied()
                .chain([(HSTS_NAME, hsts.as_str())])
            {
                assert!(
                    head.contains(&format!("{name}: {}", value.to_lowercase())),
                    "{name} on {path}: {head}"
                );
            }
        }
    }

    /// One request over HTTP/2 on TLS, as a browser sends it: no Host header, the authority is in
    /// the address. Returns the status.
    async fn h2_post(server: &Running, origin: Option<&str>) -> StatusCode {
        let tcp = TcpStream::connect(server.address).await.unwrap();
        let mut roots = RootCertStore::empty();
        let cert =
            CertificateDer::from_pem_file(server.tls_folder.path().join("self-signed-cert.pem"))
                .unwrap();
        roots.add(cert).unwrap();
        let mut client =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_root_certificates(roots)
                .with_no_client_auth();
        client.alpn_protocols = vec![b"h2".to_vec()];
        let name = ServerName::try_from("localhost").unwrap();
        let tls = TlsConnector::from(Arc::new(client))
            .connect(name, tcp)
            .await
            .unwrap();
        assert_eq!(tls.get_ref().1.alpn_protocol(), Some(&b"h2"[..]));
        let (sender, connection) = h2::client::handshake(tls).await.unwrap();
        tokio::spawn(connection);
        let port = server.address.port();
        let mut request = Request::post(format!("https://localhost:{port}/api/v1/status"));
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        let (response, _) = sender
            .ready()
            .await
            .unwrap()
            .send_request(request.body(()).unwrap(), true)
            .unwrap();
        response.await.unwrap().status()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sec_06_over_tls_and_http2_the_origin_check_uses_the_authority_and_the_scheme() {
        let server = start(true, None).await;
        let port = server.address.port();
        // Same origin: passes the check (the route only answers GET, so: 405, not 403).
        let same = h2_post(&server, Some(&format!("https://localhost:{port}"))).await;
        assert_eq!(same, StatusCode::METHOD_NOT_ALLOWED);
        // Another site, another scheme, another port, or no origin at all: refused.
        for origin in [
            Some("https://evil.test".to_owned()),
            Some(format!("http://localhost:{port}")),
            Some(format!("https://localhost:{}", port.wrapping_add(1))),
            None,
        ] {
            let status = h2_post(&server, origin.as_deref()).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{origin:?}");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sec_10_plain_http_is_not_answered_on_an_https_port() {
        let server = start(true, None).await;
        let response = fetch(&server, false, &get("/", "")).await;
        assert!(
            !String::from_utf8_lossy(&response).contains("200 OK"),
            "plain request got a page"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sec_11_text_is_compressed_when_the_browser_accepts_it() {
        let server = start(true, None).await;
        let plain = fetch(&server, true, &get("/", "")).await;
        let zipped = fetch(&server, true, &get("/", "Accept-Encoding: gzip\r\n")).await;
        assert!(!head_of(&plain).contains("content-encoding"));
        assert!(head_of(&zipped).contains("content-encoding: gzip"));
        assert!(zipped.len() < plain.len());
        let brotli = fetch(&server, true, &get("/", "Accept-Encoding: br\r\n")).await;
        assert!(head_of(&brotli).contains("content-encoding: br"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sec_10_a_plain_listener_with_trusted_proxies_refuses_other_addresses() {
        let loopback: IpAddr = "127.0.0.1".parse().unwrap();
        let other: IpAddr = "10.255.255.1".parse().unwrap();

        let trusted = start(false, Some(vec![loopback])).await;
        let response = fetch(&trusted, false, &get("/health/live", "")).await;
        assert!(String::from_utf8_lossy(&response).contains("200 OK"));

        let refused = start(false, Some(vec![other])).await;
        let response = fetch(&refused, false, &get("/health/live", "")).await;
        assert!(
            response.is_empty(),
            "an address that is not trusted got an answer"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sec_10_a_mapped_form_of_a_trusted_proxy_is_trusted() {
        let mapped: IpAddr = "::ffff:127.0.0.1".parse().unwrap();
        let server = start(false, Some(vec![mapped])).await;
        let response = fetch(&server, false, &get("/health/live", "")).await;
        assert!(String::from_utf8_lossy(&response).contains("200 OK"));
        // Behind a trusted proxy the connection counts as HTTPS: the full HSTS lifetime.
        assert!(head_of(&response).contains(&format!(
            "{HSTS_NAME}: {}",
            hsts_value(bb_api::HSTS_MAX_AGE_TRUSTED)
        )));
    }

    /// The answers that hand out a secret (a big one included) are tested where there is a
    /// database to answer them: `perf_03_answers_with_a_secret_are_not_stored_or_compressed`
    /// (bb-server) and `perf_03_no_route_that_hands_out_a_secret_is_compressed` (bb-api).
    #[tokio::test(flavor = "multi_thread")]
    async fn sec_11_json_answers_are_compressed_above_1_kib_only() {
        let server = start(true, None).await;
        // The API description: a big answer that holds no secret.
        let big = "/api/v1/openapi.json";
        let plain = fetch(&server, true, &get(big, "")).await;
        assert!(!head_of(&plain).contains("content-encoding"));
        for (accepts, encoding) in [("gzip", "gzip"), ("br", "br"), ("br, gzip", "br")] {
            let zipped = fetch(
                &server,
                true,
                &get(big, &format!("Accept-Encoding: {accepts}\r\n")),
            )
            .await;
            let head = head_of(&zipped);
            assert!(head.contains("200 ok"), "{head}");
            assert!(
                head.contains(&format!("content-encoding: {encoding}")),
                "{accepts}: {head}"
            );
            assert!(head.contains("application/json"));
            assert!(zipped.len() < plain.len());
        }
        // A small one stays as it is.
        let small = fetch(
            &server,
            true,
            &get("/health/ready", "Accept-Encoding: gzip, br\r\n"),
        )
        .await;
        assert!(!head_of(&small).contains("content-encoding"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sec_10_a_client_that_sends_its_headers_too_slowly_is_disconnected() {
        let server = start(false, None).await;
        let mut stream = TcpStream::connect(server.address).await.unwrap();
        stream
            .write_all(
                b"GET / HTTP/1.1
Host: local",
            )
            .await
            .unwrap();
        let mut out = Vec::new();
        let ended =
            tokio::time::timeout(Duration::from_secs(10), stream.read_to_end(&mut out)).await;
        assert!(ended.is_ok(), "the server kept a half-sent request open");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sec_10_no_more_connections_than_allowed_are_served_at_once() {
        let server = start_with(false, None, 1).await;
        let first = TcpStream::connect(server.address).await.unwrap();
        let mut second = TcpStream::connect(server.address).await.unwrap();
        second
            .write_all(get("/health/live", "").as_bytes())
            .await
            .unwrap();
        let mut out = Vec::new();
        let waiting =
            tokio::time::timeout(Duration::from_millis(400), second.read_to_end(&mut out)).await;
        assert!(
            waiting.is_err(),
            "a second connection was served at the limit"
        );
        drop(first);
        let answer =
            tokio::time::timeout(Duration::from_secs(5), second.read_to_end(&mut out)).await;
        assert!(answer.is_ok());
        assert!(String::from_utf8_lossy(&out).contains("200 OK"));
    }
}
