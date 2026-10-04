//! One real sign-in over TLS and HTTP/2, as a browser makes it: no `Host` header, the authority
//! in the address, the cookies and the token in the headers (SEC-06, SEC-10, SEC-12). Like the
//! origin test of the API skeleton, but through the whole stack down to the database.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Bytes;
use axum::http::{HeaderMap, Request, StatusCode};
use bb_api::tls::{TlsSource, server_config};
use bb_api::{
    AuthHandle, HSTS_MAX_AGE_SELF_SIGNED, Incoming, LimitsConfig, ListenerOptions, Options,
    WebSource, serve,
};
use bb_testing::each_database;
use h2::client::SendRequest;
use rustls::pki_types::pem::PemObject as _;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, RootCertStore};
use serde_json::{Value, json};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio_rustls::TlsConnector;

use super::rig::{Outcome, PASSWORD, Rig};

type Failure = Box<dyn std::error::Error + Send + Sync>;

/// A browser on one HTTP/2 connection.
struct Client {
    sender: SendRequest<Bytes>,
    port: u16,
    session: Option<String>,
    token: Option<String>,
}

struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
}

impl Client {
    async fn connect(
        address: std::net::SocketAddr,
        certificate: &std::path::Path,
    ) -> Result<Self, Failure> {
        let tcp = TcpStream::connect(address).await?;
        let mut roots = RootCertStore::empty();
        roots.add(CertificateDer::from_pem_file(certificate)?)?;
        let mut config =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()?
                .with_root_certificates(roots)
                .with_no_client_auth();
        config.alpn_protocols = vec![b"h2".to_vec()];
        let tls = TlsConnector::from(Arc::new(config))
            .connect(ServerName::try_from("localhost")?, tcp)
            .await?;
        assert_eq!(tls.get_ref().1.alpn_protocol(), Some(&b"h2"[..]));
        let (sender, connection) = h2::client::handshake(tls).await?;
        tokio::spawn(connection);
        Ok(Self {
            sender,
            port: address.port(),
            session: None,
            token: None,
        })
    }

    /// One request. `origin` is what the browser says it came from; the token is the one the
    /// app read from its cookie, unless `token` replaces it.
    async fn call(
        &mut self,
        method: &str,
        path: &str,
        body: Option<Value>,
        origin: Option<&str>,
        token: Option<&str>,
    ) -> Result<Answer, Failure> {
        let mut request = Request::builder()
            .method(method)
            .uri(format!("https://localhost:{}{path}", self.port));
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        if let Some(session) = &self.session {
            request = request.header(
                "cookie",
                format!(
                    "__Host-bb_session={session}; __Host-bb_csrf={}",
                    self.token.clone().unwrap_or_default()
                ),
            );
        }
        if let Some(token) = token.map(str::to_owned).or_else(|| self.token.clone()) {
            request = request.header("x-csrf-token", token);
        }
        let payload = match &body {
            Some(value) => {
                request = request.header("content-type", "application/json");
                Some(Bytes::from(serde_json::to_vec(value)?))
            }
            None => None,
        };
        let (response, mut stream) = self
            .sender
            .clone()
            .ready()
            .await?
            .send_request(request.body(())?, payload.is_none())?;
        if let Some(payload) = payload {
            stream.send_data(payload, true)?;
        }
        let (parts, mut incoming) = response.await?.into_parts();
        let mut bytes = Vec::new();
        while let Some(chunk) = incoming.data().await {
            let chunk = chunk?;
            incoming.flow_control().release_capacity(chunk.len())?;
            bytes.extend_from_slice(&chunk);
        }
        let answer = Answer {
            status: parts.status,
            headers: parts.headers,
            body: if bytes.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(&bytes)?
            },
        };
        for cookie in answer.headers.get_all("set-cookie") {
            let text = cookie.to_str()?;
            let pair = text.split(';').next().unwrap_or_default();
            let cleared = text.contains("Max-Age=0");
            match pair.split_once('=') {
                Some(("__Host-bb_session", value)) => {
                    self.session = (!cleared).then(|| value.to_owned());
                }
                Some(("__Host-bb_csrf", value)) => {
                    self.token = (!cleared).then(|| value.to_owned());
                }
                _ => {}
            }
        }
        Ok(answer)
    }
}

/// A server on a real socket with a certificate it made itself.
struct Running {
    address: std::net::SocketAddr,
    folder: tempfile::TempDir,
    stop: oneshot::Sender<()>,
    server: tokio::task::JoinHandle<std::io::Result<()>>,
}

async fn start(rig: &Rig) -> Result<Running, Failure> {
    let folder = tempfile::tempdir()?;
    let config = server_config(&TlsSource::SelfSigned {
        folder: folder.path().to_path_buf(),
        names: vec![],
    })?;
    let tcp = TcpListener::bind("127.0.0.1:0").await?;
    let address = tcp.local_addr()?;
    let incoming = Incoming::start(
        tcp,
        ListenerOptions {
            tls: Some(config),
            only_from: None,
            max_connections: 100,
        },
    );
    let options = Options {
        web: WebSource::Folder("no/such/folder".into()),
        hsts_max_age: Some(HSTS_MAX_AGE_SELF_SIGNED),
        header_timeout: Duration::from_secs(5),
        data_folder: folder.path().to_path_buf(),
        trusted_proxies: vec![],
        limits: LimitsConfig::default(),
        auth: AuthHandle::new(rig.services.clone()),
    };
    let (stop, stopped) = oneshot::channel::<()>();
    let server = tokio::spawn(serve(incoming, options, async move {
        let _ = stopped.await;
    }));
    Ok(Running {
        address,
        folder,
        stop,
        server,
    })
}

/// The first start: the page asks what to show, then the setup code makes the administrator.
async fn first_start(client: &mut Client, rig: &Rig, own: &str) -> Outcome {
    let api = "/api/v1/auth";
    let status = client
        .call("GET", &format!("{api}/status"), None, None, None)
        .await?;
    assert_eq!(status.status, StatusCode::OK);
    assert_eq!(status.body["setup_needed"], true);
    assert!(status.headers.contains_key("strict-transport-security"));
    assert!(status.headers.contains_key("content-security-policy"));
    let setup = json!({
        "code": rig.setup_code,
        "display_name": "Ada Admin",
        "email": "ada@example.test",
        "password": PASSWORD,
    });
    let made = client
        .call(
            "POST",
            &format!("{api}/setup"),
            Some(setup),
            Some(own),
            None,
        )
        .await?;
    assert_eq!(made.status, StatusCode::OK, "{}", made.body);
    let cookies: Vec<String> = made
        .headers
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok().map(str::to_owned))
        .collect();
    assert!(cookies.iter().any(|cookie| {
        cookie.starts_with("__Host-bb_session=")
            && cookie.contains("HttpOnly")
            && cookie.contains("Secure")
    }));
    assert!(
        cookies.iter().any(|cookie| {
            cookie.starts_with("__Host-bb_csrf=") && !cookie.contains("HttpOnly")
        })
    );
    let me = client
        .call("GET", &format!("{api}/me"), None, None, None)
        .await?;
    assert_eq!(me.body["email"], "ada@example.test");
    Ok(())
}

/// A change from another site, one without the token, and one with a forged token are refused;
/// the right one signs out.
async fn changes_need_origin_and_token(client: &mut Client, own: &str) -> Outcome {
    let sign_out = "/api/v1/auth/sign-out";
    let elsewhere = client
        .call("POST", sign_out, None, Some("https://evil.test"), None)
        .await?;
    assert_eq!(elsewhere.status, StatusCode::FORBIDDEN);
    assert_eq!(elsewhere.body["code"], "request.wrong_origin");
    let no_origin = client.call("POST", sign_out, None, None, None).await?;
    assert_eq!(no_origin.body["code"], "request.wrong_origin");
    let forged = client
        .call("POST", sign_out, None, Some(own), Some("forged"))
        .await?;
    assert_eq!(forged.status, StatusCode::FORBIDDEN);
    assert_eq!(forged.body["code"], "request.wrong_token");
    let own_token = client.token.clone().ok_or("no token")?;
    let out = client
        .call("POST", sign_out, None, Some(own), Some(&own_token))
        .await?;
    assert_eq!(out.status, StatusCode::NO_CONTENT);
    assert!(client.session.is_none());
    let gone = client
        .call("GET", "/api/v1/auth/me", None, None, None)
        .await?;
    assert_eq!(gone.status, StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn sec_12_over_tls_and_http2_a_person_sets_up_signs_out_and_signs_in_again() -> Outcome {
    each_database("sec_12_tls_http2_sign_in", async |db| {
        let rig = Rig::start(db, 10).await?;
        let running = start(&rig).await?;
        let certificate = running.folder.path().join("self-signed-cert.pem");
        let mut client = Client::connect(running.address, &certificate).await?;
        let own = format!("https://localhost:{}", running.address.port());
        let api = "/api/v1/auth";

        first_start(&mut client, &rig, &own).await?;
        let first_token = client.token.clone().ok_or("no token")?;
        changes_need_origin_and_token(&mut client, &own).await?;

        // Signing in again: a wrong password is turned away, the right one gets a new session.
        let wrong = client
            .call(
                "POST",
                &format!("{api}/sign-in"),
                Some(json!({"email": "ada@example.test", "password": "Wrong-Password-1"})),
                Some(&own),
                None,
            )
            .await?;
        assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
        let again = client
            .call(
                "POST",
                &format!("{api}/sign-in"),
                Some(json!({"email": "ada@example.test", "password": PASSWORD})),
                Some(&own),
                None,
            )
            .await?;
        assert_eq!(again.status, StatusCode::OK);
        assert_ne!(client.token.clone().ok_or("no token")?, first_token);
        let me = client
            .call("GET", &format!("{api}/me"), None, None, None)
            .await?;
        assert_eq!(me.body["email"], "ada@example.test");

        let _ = running.stop.send(());
        running.server.await??;
        Ok(())
    })
    .await
}
