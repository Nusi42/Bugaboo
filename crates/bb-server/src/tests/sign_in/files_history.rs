//! Files in the history of an issue (ISS-06, ATT-01, M4-F50): adding a file, putting it into the
//! trash and bringing it back each have a row, with the name of the file. Both databases.

use axum::http::StatusCode;
use bb_testing::{TestResult, each_database};
use serde_json::Value;

use super::files_http::{ATTACHMENTS, upload};
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
