//! OPS-08 and PRIV-01: a request leaves one JSON log line with a request ID, and nothing in it
//! can identify a person.

// Dependencies of the library that this test does not use itself (`unused_crate_dependencies`).
use bb_core as _;
use hyper_util as _;
use mime_guess as _;
use rcgen as _;
use rust_embed as _;
use rustls as _;
use tempfile as _;
use tokio_rustls as _;
use tower_http as _;

#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod logs {
    use std::io;
    use std::sync::{Arc, Mutex};

    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::Request;
    use bb_api::{ClientAddr, Options, WebSource, router};
    use tower::ServiceExt as _;

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);

    impl io::Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn ops_08_a_request_is_logged_as_json_with_its_id_and_without_personal_data() {
        let capture = Capture::default();
        let writer = capture.clone();
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_current_span(true)
            .with_writer(move || writer.clone())
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let app = router(Options {
            web: WebSource::Folder("no/such/folder".into()),
            hsts_max_age: None,
            header_timeout: std::time::Duration::from_secs(30),
            data_folder: std::env::temp_dir(),
            trusted_proxies: vec![],
        });
        let mut request = Request::get("/users/jane.doe@example.org/profile?name=Jane+Doe")
            .header("authorization", "Bearer secret-token-123")
            .header("cookie", "session=abc123")
            .body(Body::empty())
            .unwrap();
        request.extensions_mut().insert(ConnectInfo(ClientAddr(
            "[::ffff:203.0.113.77]:51000".parse().unwrap(),
        )));
        let response = app.oneshot(request).await.unwrap();
        let id = response
            .headers()
            .get("x-request-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();

        let text = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
        let line = text.lines().next().expect("one log line");
        assert!(line.starts_with('{') && line.ends_with('}'), "{line}");
        assert!(line.contains(&format!("\"request_id\":\"{id}\"")), "{line}");
        assert!(line.contains("\"status\":404"), "{line}");
        assert!(line.contains("\"net\":\"203.0.113.0\""), "{line}");
        for personal in [
            "jane",
            "example.org",
            "Doe",
            "profile",
            "secret-token",
            "abc123",
            "203.0.113.77",
        ] {
            assert!(!text.contains(personal), "{personal} is in the log: {line}");
        }
    }
}
