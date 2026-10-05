//! The status page of the administrators and `/health/ready` with a database (OPS-08, M1-F19,
//! M1-F20).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use bb_api::tls::ServedCertificate;
use bb_api::{LimitsConfig, ProbeHandle, router};
use bb_app::status::{DatabaseKind, LogBreak, LogState, ServerProbe as _};
use bb_store::Database;
use bb_testing::each_database;
use serde_json::json;
use tower::ServiceExt as _;

use super::http::{Browser, called, options, setup_body};
use super::rig::{Outcome, PASSWORD, Rig};
use crate::probe::{LogChecking, Probe};

const STATUS: &str = "/api/v1/admin/status";

/// The router of the rig with a real probe.
fn app_with_probe(rig: &Rig, certificate: Option<Arc<ServedCertificate>>) -> axum::Router {
    let mut options = options(rig, LimitsConfig::default(), true);
    // These tests change the log or its anchor file from outside and look again at once, so the
    // page keeps no answer.
    options.probe = ProbeHandle::new(Arc::new(
        Probe::new(
            rig.database.clone(),
            rig.seal.clone(),
            DatabaseKind::BuiltIn,
            certificate,
            rig.services.log_writes(),
            rig.folder.path().to_path_buf(),
        )
        .with_log_checking(LogChecking {
            keep: std::time::Duration::ZERO,
            ..LogChecking::default()
        }),
    ));
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

        // An administrator sees the state; the answer is never cached.
        let reply = ada.send("GET", STATUS, None).await?;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.header("cache-control"), "no-store");
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
async fn ops_08_the_status_shows_the_end_of_the_certificate_it_serves_now() -> Outcome {
    each_database("ops_08_status_certificate", async |db| {
        let rig = Rig::start(db, 10).await?;
        let folder = tempfile::tempdir()?;
        let served = Arc::new(bb_api::tls::serve_certificate(
            &bb_api::tls::TlsSource::SelfSigned {
                folder: folder.path().to_path_buf(),
                names: vec![],
            },
        )?);
        let app = app_with_probe(&rig, Some(Arc::clone(&served)));
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let ends = async |ada: &mut Browser| -> Result<jiff::Timestamp, Box<dyn std::error::Error + Send + Sync>> {
            let reply = ada.send("GET", STATUS, None).await?;
            Ok(reply.body["certificate"]["expires_at"]
                .as_str()
                .ok_or("no end")?
                .parse::<jiff::Timestamp>()?)
        };
        let first = ends(&mut ada).await?;
        // Certificates made by Bugaboo are valid for a year.
        let days = (first.as_second() - jiff::Timestamp::now().as_second()) / 86_400;
        assert!((360..=366).contains(&days), "{days}");

        // After a renewal the page shows the end of the new certificate, not of the old one.
        let late = jiff::Timestamp::now().as_second() + 340 * 86_400;
        served.renew(late)?;
        let second = ends(&mut ada).await?;
        assert!(second.as_second() > first.as_second() + 300 * 86_400);
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

/// The probe of the rig, with the times of reusing and re-reading set by the test.
fn probe_of(rig: &Rig, checking: LogChecking) -> Probe {
    Probe::new(
        rig.database.clone(),
        rig.seal.clone(),
        DatabaseKind::BuiltIn,
        None,
        rig.services.log_writes(),
        rig.folder.path().to_path_buf(),
    )
    .with_log_checking(checking)
}

/// How many entries the status shows, and the place of the last one.
fn log_of(state: &LogState) -> Result<(u64, i64), Box<dyn std::error::Error + Send + Sync>> {
    match state {
        LogState::Valid {
            entries,
            head: Some((seq, _)),
            ..
        } => Ok((*entries, *seq)),
        other => Err(format!("the log should be intact: {other:?}").into()),
    }
}

#[tokio::test]
async fn ops_08_the_status_page_follows_the_log_and_reuses_its_answer_until_something_is_written()
-> Outcome {
    each_database("ops_08_status_incremental", async |db| {
        let rig = Rig::start(db, 10).await?;
        let kept = probe_of(
            &rig,
            LogChecking {
                keep: std::time::Duration::from_hours(1),
                full_every: std::time::Duration::from_hours(24),
            },
        );
        let always = probe_of(
            &rig,
            LogChecking {
                keep: std::time::Duration::ZERO,
                full_every: std::time::Duration::from_hours(24),
            },
        );
        rig.admin("ada@example.test").await?;

        // The first visit reads the whole log; entries and place agree.
        let (first, head) = log_of(&kept.snapshot().await.security_log)?;
        assert!(first >= 1);
        assert_eq!(i64::try_from(first)?, head);

        // Every write by the server makes the answer old, and the next visit shows the new
        // entries: the count and the place of the last entry follow the log.
        rig.sign_in("ada@example.test", PASSWORD).await?;
        let (second, head) = log_of(&kept.snapshot().await.security_log)?;
        assert!(second > first);
        assert_eq!(i64::try_from(second)?, head);

        // An entry that was added behind the back of the server (here straight into the store)
        // is not seen while the answer is kept: nothing told the page that the log changed.
        rig.database
            .append_security_log(
                &rig.seal,
                &bb_store::NewSecurityLogEntry {
                    time: jiff::Timestamp::now(),
                    person: None,
                    action: bb_core::technical::TechnicalName::new("sign_in.failed")?,
                    subject: None,
                    network: None,
                },
            )
            .await?;
        assert_eq!(log_of(&kept.snapshot().await.security_log)?.0, second);
        // A page that keeps nothing sees it at once, and the page that kept the answer sees it
        // with the next write of the server.
        let (third, _) = log_of(&always.snapshot().await.security_log)?;
        assert_eq!(third, second + 1);
        rig.sign_in("ada@example.test", PASSWORD).await?;
        assert!(log_of(&kept.snapshot().await.security_log)?.0 > third);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_15_the_status_page_warns_when_a_log_entry_could_not_be_written() -> Outcome {
    each_database("sec_15_status_failed_write", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let reply = ada.send("GET", STATUS, None).await?;
        assert_eq!(reply.body["security_log"]["write_failures"], 0);

        // The anchor file cannot be written any more: a folder has taken its place.
        let anchor = rig.folder.path().join("logs").join("security-head.log");
        std::fs::remove_file(&anchor)?;
        std::fs::create_dir(&anchor)?;

        // Signing in goes on (the decision of M1: a log that cannot be written does not lock
        // everybody out) ...
        let mut again = Browser::new(&app, &called());
        let signed = again
            .send(
                "POST",
                "/api/v1/auth/sign-in",
                Some(json!({"email": "ada@example.test", "password": PASSWORD})),
            )
            .await?;
        assert_eq!(signed.status, StatusCode::OK);

        // ... and the status page counts what could not be written.
        let reply = ada.send("GET", STATUS, None).await?;
        let failures = reply.body["security_log"]["write_failures"].as_u64();
        assert!(failures.is_some_and(|count| count >= 1), "{failures:?}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_15_the_status_page_warns_when_the_anchor_is_ahead() -> Outcome {
    each_database("sec_15_status_anchor_ahead", async |db| {
        let rig = Rig::start(db, 10).await?;
        rig.admin("ada@example.test").await?;
        rig.sign_in("ada@example.test", PASSWORD).await?;

        // A database that was restored from an older state: it has fewer entries than the anchor
        // file (a new one with none, here), and the same keys and anchor file.
        let older = tempfile::tempdir()?;
        let restored = Database::connect_file(&older.path().join("older.db")).await?;
        restored.migrate(jiff::Timestamp::now()).await?;
        let probe = Probe::new(
            restored,
            rig.seal.clone(),
            DatabaseKind::BuiltIn,
            None,
            rig.services.log_writes(),
            rig.folder.path().to_path_buf(),
        );
        let state = probe.snapshot().await.security_log;
        assert!(
            matches!(
                state,
                LogState::Broken {
                    reason: LogBreak::Behind,
                    ..
                }
            ),
            "{state:?}"
        );

        // The page says so, with the reason the person can act on.
        let mut options = options(&rig, LimitsConfig::default(), true);
        options.probe = ProbeHandle::new(Arc::new(probe));
        let app = router(options);
        let mut ada = Browser::new(&app, &called());
        ada.send(
            "POST",
            "/api/v1/auth/sign-in",
            Some(json!({"email": "ada@example.test", "password": PASSWORD})),
        )
        .await?;
        let reply = ada.send("GET", STATUS, None).await?;
        assert_eq!(reply.body["security_log"]["result"], "broken");
        assert_eq!(reply.body["security_log"]["reason"], "behind");
        Ok(())
    })
    .await
}

/// Gives every account on this computer a right to read `folder`.
fn open_up(folder: &std::path::Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(windows)]
    {
        let output = std::process::Command::new("icacls")
            .arg(folder)
            .args(["/grant", "*S-1-5-32-545:(OI)(CI)R"])
            .output()?;
        assert!(output.status.success());
    }
    Ok(())
}

#[tokio::test]
async fn sec_16_the_status_page_shows_what_of_the_data_folder_is_too_open() -> Outcome {
    each_database("sec_16_status_data_folder", async |db| {
        let rig = Rig::start(db, 10).await?;
        let data = crate::data_folder::DataFolder::prepare(&rig.folder.path().join("data"))?;
        let mut options = options(&rig, LimitsConfig::default(), true);
        options.probe = ProbeHandle::new(Arc::new(Probe::new(
            rig.database.clone(),
            rig.seal.clone(),
            DatabaseKind::BuiltIn,
            None,
            rig.services.log_writes(),
            data.root().to_path_buf(),
        )));
        let app = router(options);
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;

        // A folder that Bugaboo made: nothing is too open.
        let reply = ada.send("GET", STATUS, None).await?;
        let open = &reply.body["data_folder_open"];
        for part in ["folder", "database", "files", "backups", "setup_code_file"] {
            assert_eq!(open[part], "private", "{part}");
        }

        // The person opens the folder for backups to everybody; the page says so and the folder
        // keeps the rights the person gave it.
        open_up(&data.root().join("backups"))?;
        let reply = ada.send("GET", STATUS, None).await?;
        let open = &reply.body["data_folder_open"];
        assert_eq!(open["backups"], "open");
        assert_eq!(open["files"], "private");
        assert_eq!(open["folder"], "private");
        assert!(
            data.rights().backups.is_open(),
            "the rights were left as they are"
        );
        Ok(())
    })
    .await
}
