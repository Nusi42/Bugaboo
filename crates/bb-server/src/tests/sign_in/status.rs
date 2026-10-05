//! The status page of the administrators and `/health/ready` with a database (OPS-08, M1-F19,
//! M1-F20).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use bb_api::{LimitsConfig, ProbeHandle, router};
use bb_app::status::DatabaseKind;
use bb_testing::each_database;
use serde_json::json;
use tower::ServiceExt as _;

use super::http::{Browser, called, options, setup_body};
use super::rig::{Outcome, PASSWORD, Rig};
use crate::probe::Probe;

const STATUS: &str = "/api/v1/admin/status";

/// The router of the rig with a real probe.
pub(super) fn app_with_probe(rig: &Rig, certificate: Option<std::path::PathBuf>) -> axum::Router {
    let mut options = options(rig, LimitsConfig::default(), true);
    options.probe = ProbeHandle::new(Arc::new(Probe::new(
        rig.database.clone(),
        rig.seal.clone(),
        DatabaseKind::BuiltIn,
        certificate,
    )));
    router(options)
}

#[tokio::test]
async fn ops_08_only_administrators_see_the_status_of_the_server() -> Outcome {
    each_database("ops_08_status_access", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = app_with_probe(&rig, None);
        let used = called();

        // Nobody signed in: not signed in. Nothing about the server leaks.
        let mut stranger = Browser::new(&app, &used);
        let reply = stranger.send("GET", STATUS, None).await?;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        assert_eq!(reply.code(), "auth.not_signed_in");

        // A person who is not an administrator: forbidden.
        let mut ada = Browser::new(&app, &used);
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let invitation = ada
            .send(
                "POST",
                "/api/v1/auth/invitations",
                Some(json!({"email": "bob@example.test"})),
            )
            .await?;
        let secret = invitation.body["link_path"]
            .as_str()
            .and_then(|path| path.split_once('#'))
            .map(|(_, secret)| secret.to_owned())
            .ok_or("no link")?;
        let mut bob = Browser::new(&app, &used);
        bob.send(
            "POST",
            "/api/v1/auth/invitation/accept",
            Some(json!({"secret": secret, "display_name": "Bob", "password": PASSWORD})),
        )
        .await?;
        let reply = bob.send("GET", STATUS, None).await?;
        assert_eq!(reply.status, StatusCode::FORBIDDEN);
        assert_eq!(reply.code(), "access.forbidden");

        // An administrator sees the state; the answer may be kept only privately and is asked
        // about again every time.
        let reply = ada.send("GET", STATUS, None).await?;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.header("cache-control"), "private, no-cache");
        assert_eq!(reply.body["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(reply.body["database"]["kind"], "built_in");
        assert_eq!(reply.body["database"]["ok"], true);
        assert!(reply.body["started_at"].is_string());
        assert!(reply.body["uptime_seconds"].is_u64());
        assert_eq!(reply.body["certificate"], serde_json::Value::Null);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_08_the_status_shows_the_result_of_checking_the_security_log() -> Outcome {
    each_database("ops_08_status_security_log", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;

        // Setting up wrote entries, and the anchor file vouches for the end of the log.
        let reply = ada.send("GET", STATUS, None).await?;
        let log = &reply.body["security_log"];
        assert_eq!(log["result"], "valid");
        assert!(log["entries"].as_u64().unwrap_or_default() >= 1);
        assert_eq!(log["anchored"], true);
        assert_eq!(log["head_seq"], log["entries"]);
        assert_eq!(log["head_hash"].as_str().map(str::len), Some(16));

        // Without the anchor file nothing vouches for the end of the log: a warning (M1-F20).
        std::fs::remove_file(rig.folder.path().join("logs").join("security-head.log"))?;
        let reply = ada.send("GET", STATUS, None).await?;
        assert_eq!(reply.body["security_log"]["result"], "valid");
        assert_eq!(reply.body["security_log"]["anchored"], false);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_08_the_status_shows_when_the_certificate_ends() -> Outcome {
    each_database("ops_08_status_certificate", async |db| {
        let rig = Rig::start(db, 10).await?;
        let folder = tempfile::tempdir()?;
        let source = bb_api::tls::TlsSource::SelfSigned {
            folder: folder.path().to_path_buf(),
            names: vec![],
        };
        bb_api::tls::server_config(&source)?;
        let app = app_with_probe(&rig, Some(bb_api::tls::certificate_path(folder.path())));
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let reply = ada.send("GET", STATUS, None).await?;
        let ends = reply.body["certificate"]["expires_at"]
            .as_str()
            .ok_or("no end")?
            .parse::<jiff::Timestamp>()?;
        // Certificates made by Bugaboo are valid for a year.
        let days = (ends.as_second() - jiff::Timestamp::now().as_second()) / 86_400;
        assert!((360..=366).contains(&days), "{days}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_08_ready_means_the_database_answers() -> Outcome {
    each_database("ops_08_ready_database", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = app_with_probe(&rig, None);
        let ready = || async {
            let request = Request::get("/health/ready").body(Body::empty())?;
            let response = app.clone().oneshot(request).await?;
            let status = response.status();
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>((
                status,
                String::from_utf8(bytes.to_vec())?,
            ))
        };
        let (status, body) = ready().await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body,
            r#"{"status":"ok","checks":{"data_folder":"ok","database":"ok"}}"#
        );

        rig.database.close().await;
        let (status, body) = ready().await?;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            body,
            r#"{"status":"unavailable","checks":{"data_folder":"ok","database":"failed"}}"#
        );
        Ok(())
    })
    .await
}
