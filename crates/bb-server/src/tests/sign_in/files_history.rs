//! Files in the history of an issue (ISS-06, ATT-01, M4-F50): adding a file, putting it into the
//! trash and bringing it back each have a row, with the name of the file. Both databases.

use axum::http::StatusCode;
use bb_core::files::TRASH_KEPT;
use bb_testing::{TestResult, each_database};
use serde_json::Value;

use super::files_http::{ATTACHMENTS, DAY, run_jobs, upload};
use super::http::rig_and_app;
use super::issues_site::{ISSUES, Site, id_in, key_of, ok};

/// What the file rows of the history say, newest first: the event and the name.
fn file_rows(history: &Value) -> Vec<(String, String)> {
    history["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| item["kind"] == "file")
        .map(|item| {
            (
                item["file_event"].as_str().unwrap_or_default().to_owned(),
                item["file_name"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

#[tokio::test]
async fn iss_06_history_has_a_row_for_each_file_added_deleted_and_brought_back() -> TestResult {
    each_database("iss_06_history_files", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let path = format!("{ISSUES}/{issue}/history?tab=changes");
        let file = upload(&mut ada, &issue, "jam.txt", b"the jam").await?;
        let id = id_in(&file)?;
        let gone = ada
            .send("DELETE", &format!("{ATTACHMENTS}/{id}"), None)
            .await?;
        assert_eq!(gone.status, StatusCode::NO_CONTENT);
        ok(&ada
            .send("POST", &format!("{ATTACHMENTS}/{id}/restore"), None)
            .await?)?;
        let history = ada.send("GET", &path, None).await?;
        let rows = file_rows(ok(&history)?);
        let expected: Vec<(String, String)> = ["restored", "deleted", "added"]
            .iter()
            .map(|event| ((*event).to_owned(), "jam.txt".to_owned()))
            .collect();
        assert_eq!(rows, expected, "{}", history.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_06_a_file_that_is_gone_for_good_keeps_its_row_without_its_name() -> TestResult {
    each_database("iss_06_history_purged_file", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let file = upload(&mut ada, &issue, "private-notes.txt", b"the jam").await?;
        let gone = ada
            .send("DELETE", &format!("{ATTACHMENTS}/{}", id_in(&file)?), None)
            .await?;
        assert_eq!(gone.status, StatusCode::NO_CONTENT);
        // The days of the trash pass and the clean-up removes the file for good.
        rig.clock.advance(TRASH_KEPT.as_secs() + DAY);
        run_jobs(&rig).await?;
        let mut ada = super::http::Browser::signed_in_as(&app, super::http::ADA_EMAIL).await?;
        let history = ada
            .send(
                "GET",
                &format!("{ISSUES}/{issue}/history?tab=changes"),
                None,
            )
            .await?;
        // The rows stay, and nothing names the file any more: its name was the person's content.
        let expected: Vec<(String, String)> = ["deleted", "added"]
            .iter()
            .map(|event| ((*event).to_owned(), String::new()))
            .collect();
        assert_eq!(file_rows(ok(&history)?), expected, "{}", history.body);
        assert!(
            !history.body.to_string().contains("private-notes"),
            "{}",
            history.body
        );
        Ok(())
    })
    .await
}
