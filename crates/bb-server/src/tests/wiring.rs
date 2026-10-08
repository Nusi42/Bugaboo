//! The start of the server: the database is opened and migrated, the keys are opened, and the
//! setup code of the first start is made (AUTH-09, SEC-16, M1-F19, M1-F24).

use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use bb_api::{Options, router};
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};
use tower::ServiceExt as _;

use super::{Serving, Wired, wire};
use crate::config::{Settings, Sources, resolve};
use crate::data_folder::DataFolder;

fn settings(data: &Path, database: Option<&str>) -> Result<Settings, Failure> {
    let mut cli = vec![
        ("port", "47200".to_owned()),
        ("data_dir", data.to_string_lossy().into_owned()),
    ];
    if let Some(url) = database {
        cli.push(("database_url", url.to_owned()));
    }
    Ok(resolve(&Sources {
        file: None,
        env: &|_| None,
        cli: &cli,
        base: data,
    })?)
}

fn app(wired: Wired, data: &Path) -> Router {
    router(Options {
        data_folder: data.to_path_buf(),
        auth: wired.auth,
        probe: wired.probe,
        ..Options::for_tests()
    })
}

async fn call(
    app: &Router,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> Result<(StatusCode, Value), Failure> {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::HOST, "localhost:47200")
        .header(header::ORIGIN, "http://localhost:47200")
        .header(header::CONTENT_TYPE, "application/json")
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))?;
    let response = app.clone().oneshot(request).await?;
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    Ok((
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    ))
}

fn setup_body(code: &str) -> Value {
    json!({
        "code": code,
        "display_name": "Ada Admin",
        "email": "ada@example.test",
        "password": "Tr4mpoline-Quiet-Harbor-81",
    })
}

#[tokio::test]
async fn auth_09_every_start_makes_a_new_setup_code_until_an_administrator_exists() -> TestResult {
    each_database("auth_09_start_up", async |db| {
        let folder = tempfile::tempdir()?;
        let data = DataFolder::prepare(&folder.path().join("data"))?;
        let settings = settings(data.root(), Some(db.url()))?;

        // The first start: migrated, keys made, a code to print.
        let first = wire(&settings, &data, Serving::default()).await?;
        let first_code = first
            .setup_code
            .clone()
            .ok_or("no setup code at the first start")?;
        assert_eq!(first_code.len(), 19, "four groups of four");
        for file in ["master.key", "log.key", "csrf.key"] {
            assert!(data.keys().join(file).is_file(), "{file}");
        }
        let (status, body) =
            call(&app(first, data.root()), "GET", "/api/v1/auth/status", None).await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["setup_needed"], true);

        // The next start makes a new code, and the old one is dead.
        let second = wire(&settings, &data, Serving::default()).await?;
        let second_code = second
            .setup_code
            .clone()
            .ok_or("no setup code at the second start")?;
        assert_ne!(first_code, second_code);
        let second_app = app(second, data.root());
        let (status, body) = call(
            &second_app,
            "POST",
            "/api/v1/auth/setup",
            Some(setup_body(&first_code)),
        )
        .await?;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "auth.wrong_setup_code");
        let (status, _) = call(
            &second_app,
            "POST",
            "/api/v1/auth/setup",
            Some(setup_body(&second_code)),
        )
        .await?;
        assert_eq!(status, StatusCode::OK);
        assert!(
            data.security_log_anchor().is_file(),
            "the end of the log is written down"
        );

        // Once an administrator exists there is nothing to print, whatever the start.
        let third = wire(&settings, &data, Serving::default()).await?;
        assert!(third.setup_code.is_none());
        let (_, body) = call(&app(third, data.root()), "GET", "/api/v1/auth/status", None).await?;
        assert_eq!(body["setup_needed"], false);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_09_the_setup_code_file_is_gone_once_the_administrator_exists() -> TestResult {
    let folder = tempfile::tempdir()?;
    let data = DataFolder::prepare(&folder.path().join("data"))?;
    let wired = wire(&settings(data.root(), None)?, &data, Serving::default()).await?;
    let code = wired.setup_code.clone().ok_or("no setup code")?;
    // A server without a console has saved the code in a file, and the folder holds other files.
    std::fs::write(data.setup_code_file(), format!("{code}\n"))?;
    let notes = data.root().join("notes.txt");
    std::fs::write(&notes, "mine")?;
    let app = app(wired, data.root());

    // A wrong code makes no administrator, and the code stays where it is.
    let (status, _) = call(
        &app,
        "POST",
        "/api/v1/auth/setup",
        Some(setup_body("aaaa-bbbb-cccc-dddd")),
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(data.setup_code_file().is_file(), "the code is still valid");

    // The change that makes the administrator removes the file in the same run, not at the next
    // start, and nothing else in the folder.
    let (status, _) = call(&app, "POST", "/api/v1/auth/setup", Some(setup_body(&code))).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(!data.setup_code_file().exists(), "the dead code was left");
    assert_eq!(std::fs::read_to_string(&notes)?, "mine");
    Ok(())
}

#[tokio::test]
async fn auth_09_a_setup_code_file_that_cannot_be_removed_does_not_undo_the_setup() -> TestResult {
    let folder = tempfile::tempdir()?;
    let data = DataFolder::prepare(&folder.path().join("data"))?;
    let wired = wire(&settings(data.root(), None)?, &data, Serving::default()).await?;
    let code = wired.setup_code.clone().ok_or("no setup code")?;
    // Something that cannot be removed as a file is in its place (a folder).
    std::fs::create_dir(data.setup_code_file())?;
    let (status, _) = call(
        &app(wired, data.root()),
        "POST",
        "/api/v1/auth/setup",
        Some(setup_body(&code)),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "the administrator exists");
    Ok(())
}

#[tokio::test]
async fn m1_f24_the_built_in_database_is_a_file_in_the_data_folder() -> TestResult {
    let folder = tempfile::tempdir()?;
    let data = DataFolder::prepare(&folder.path().join("data"))?;
    let wired = wire(&settings(data.root(), None)?, &data, Serving::default()).await?;
    assert!(wired.setup_code.is_some());
    assert!(data.database_file().is_file());
    Ok(())
}

#[tokio::test]
async fn sec_16_a_new_installation_has_nothing_that_is_too_open() -> TestResult {
    // The default install: Bugaboo makes the folder, the database and its companion files. None of
    // it may be reported as too open (M1-F65).
    let folder = tempfile::tempdir()?;
    let data = DataFolder::prepare(&folder.path().join("data"))?;
    let wired = wire(&settings(data.root(), None)?, &data, Serving::default()).await?;
    let code = wired.setup_code.clone().ok_or("no setup code")?;
    let (status, _) = call(
        &app(wired, data.root()),
        "POST",
        "/api/v1/auth/setup",
        Some(setup_body(&code)),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert!(data.database_file().is_file());
    assert_eq!(data.rights(), bb_app::status::DataFolderRights::default());
    Ok(())
}

#[tokio::test]
async fn sec_16_missing_keys_stop_the_start_instead_of_making_new_ones() -> TestResult {
    let folder = tempfile::tempdir()?;
    let data = DataFolder::prepare(&folder.path().join("data"))?;
    let settings = settings(data.root(), None)?;
    let wired = wire(&settings, &data, Serving::default()).await?;
    let code = wired.setup_code.clone().ok_or("no code")?;
    let (status, _) = call(
        &app(wired, data.root()),
        "POST",
        "/api/v1/auth/setup",
        Some(setup_body(&code)),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    // The database holds a security log now; its key file is lost.
    std::fs::remove_file(data.keys().join("log.key"))?;
    let error = wire(&settings, &data, Serving::default())
        .await
        .err()
        .ok_or("the start went on without the log key")?;
    let text = format!("{error:#}");
    assert!(
        text.contains("key files") && text.contains("backup"),
        "{text}"
    );
    assert!(!data.keys().join("log.key").exists(), "no new key was made");
    Ok(())
}

#[tokio::test]
async fn ops_08_a_database_that_cannot_be_opened_stops_the_start_with_a_plain_explanation()
-> TestResult {
    let folder = tempfile::tempdir()?;
    let data = DataFolder::prepare(&folder.path().join("data"))?;
    let missing = folder.path().join("no-such-folder").join("x.db");
    let url = format!("sqlite:{}", missing.to_string_lossy().replace('\\', "/"));
    let settings = settings(data.root(), Some(&url))?;
    let error = wire(&settings, &data, Serving::default())
        .await
        .err()
        .ok_or("the start went on")?;
    let text = format!("{error:#}");
    assert!(
        text.starts_with("Bugaboo could not open its database"),
        "{text}"
    );
    Ok(())
}
