//! SEC-10 and SEC-11 over a real connection: HTTPS with a generated certificate, the full header
//! set including HSTS, compression, and the plain-HTTP rules for trusted proxies.

// Dependencies of the library that this test does not use itself (`unused_crate_dependencies`).
use axum as _;
use mime_guess as _;
use rcgen as _;
use rust_embed as _;
use tower as _;
use tower_http as _;
use tracing as _;
use tracing_subscriber as _;

mod common;

#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod https {
    use std::net::{IpAddr, SocketAddr};
    use std::sync::Arc;

    use bb_api::tls::{TlsSource, server_config};
    use bb_api::{Incoming, ListenerOptions, Options, WebSource, serve};
    use rustls::pki_types::pem::PemObject as _;
    use rustls::pki_types::{CertificateDer, ServerName};
    use rustls::{ClientConfig, RootCertStore};
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::oneshot;
    use tokio_rustls::TlsConnector;

    use super::common::{EXPECTED, HSTS};

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
        let tcp = TcpListener::bind(LOOPBACK).await.unwrap();
        let address = tcp.local_addr().unwrap();
        let incoming = Incoming::start(
            tcp,
            ListenerOptions {
                tls: config,
                only_from,
            },
        )
        .unwrap();
        let options = Options {
            web: WebSource::Folder(app.path().to_path_buf()),
            https: tls,
            data_folder: tls_folder.path().to_path_buf(),
            trusted_proxies: vec![],
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
            for (name, value) in EXPECTED.iter().chain([&HSTS]) {
                assert!(
                    head.contains(&format!("{name}: {}", value.to_lowercase())),
                    "{name} on {path}: {head}"
                );
            }
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
}
