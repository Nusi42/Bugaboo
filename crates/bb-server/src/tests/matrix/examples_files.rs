//! The example requests of the routes of files (M4 WP-05, SEC-MATRIX).
//!
//! A route that works on an attachment makes a new issue first, reported by the administrator of
//! the matrix in the target project, and a file on it; so each actor who is let in works on a file
//! of an issue they did not report, and the rules about other people's issues say what they may do
//! (*See issues*, *Change issues*). A route that names an upload names the actor's own, which the
//! actor starts first where they may (so the route is tried on a real upload), and one that nobody
//! made where they may not: an upload belongs to the person who started it and nobody else can
//! reach it.

use bb_app::files::{
    DeleteAttachment, FinishUpload, IssueFilesQuery, SendPartQuery, StartUploadQuery,
};
use bb_testing::Failure;
use serde_json::json;

use super::examples::Request;
use super::examples_issues::an_issue;
use super::routes::Fixture;
use crate::sign_in_tests::http::Browser;
use crate::sign_in_tests::rig::Rig;

/// An ID that no upload has.
const NOBODYS_UPLOAD: &str = "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f";

/// The words of the file the administrator of the matrix adds.
const THE_FILE: &[u8] = b"Added by the matrix.";

/// The words of the part that the example of `sendUploadPart` sends (as the matrix writes a body).
const A_PART: &str = "of a part";

/// The body of that example.
fn a_part() -> serde_json::Value {
    json!({"bytes": A_PART})
}

/// An upload of the actor on a new issue, with room for `size` bytes, if the actor may start one;
/// otherwise the ID of one that nobody started. The actor is the one the browser is signed in as.
async fn own_upload(
    fixture: &Fixture<'_>,
    browser: &mut Browser,
    size: usize,
) -> Result<String, Failure> {
    let issue = an_issue(fixture).await?;
    let started = browser
        .send(
            "POST",
            &format!("/api/v1/issues/{issue}/uploads"),
            Some(json!({"name": "matrix.txt", "size": size})),
        )
        .await?;
    Ok(started.body["id"]
        .as_str()
        .map_or_else(|| NOBODYS_UPLOAD.to_owned(), str::to_owned))
}

/// A file on the issue, added by the administrator of the matrix: its ID.
async fn admins_file(fixture: &Fixture<'_>, issue: &str) -> Result<String, Failure> {
    let session = fixture.admin_session().to_owned();
    let api = fixture.rig().api();
    let started = api
        .start_upload(
            Rig::info(),
            StartUploadQuery {
                session_token: session.clone(),
                issue: issue.to_owned(),
                comment: None,
                name: "matrix.txt".to_owned(),
                size: THE_FILE.len() as u64,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    api.send_part(
        Rig::info(),
        SendPartQuery {
            session_token: session.clone(),
            upload: started.id,
            offset: 0,
            bytes: THE_FILE.to_vec(),
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    Ok(api
        .finish_upload(
            Rig::info(),
            FinishUpload {
                session_token: session,
                upload: started.id,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value()
        .id
        .to_string())
}

/// The request of a route of an upload, on the actor's own upload where they may start one.
async fn upload_request(
    fixture: &Fixture<'_>,
    operation: &str,
    browser: &mut Browser,
) -> Result<Request, Failure> {
    let part_size = serde_json::to_vec(&a_part()).map_or(1, |bytes| bytes.len());
    Ok(match operation {
        "getUpload" => (
            "GET",
            format!(
                "/api/v1/uploads/{}",
                own_upload(fixture, browser, 10).await?
            ),
            None,
        ),
        "sendUploadPart" => (
            "PUT",
            format!(
                "/api/v1/uploads/{}?offset=0",
                own_upload(fixture, browser, part_size).await?
            ),
            Some(a_part()),
        ),
        "cancelUpload" => (
            "DELETE",
            format!(
                "/api/v1/uploads/{}",
                own_upload(fixture, browser, 10).await?
            ),
            None,
        ),
        "finishUpload" => {
            // An upload with all its bytes is the one that can be finished.
            let upload = own_upload(fixture, browser, THE_FILE.len()).await?;
            browser
                .send_bytes(
                    "PUT",
                    &format!("/api/v1/uploads/{upload}?offset=0"),
                    THE_FILE.to_vec(),
                )
                .await?;
            ("POST", format!("/api/v1/uploads/{upload}/finish"), None)
        }
        _ => ("GET", String::new(), None),
    })
}

/// The request of a route of files, if the operation is one.
pub(super) async fn file_request(
    fixture: &Fixture<'_>,
    operation: &str,
    browser: &mut Browser,
) -> Result<Option<Request>, Failure> {
    Ok(Some(match operation {
        "startUpload" => (
            "POST",
            format!("/api/v1/issues/{}/uploads", an_issue(fixture).await?),
            Some(json!({"name": "matrix.txt", "size": 10})),
        ),
        "getUpload" | "sendUploadPart" | "cancelUpload" | "finishUpload" => {
            upload_request(fixture, operation, browser).await?
        }
        "listIssueFiles" => (
            "GET",
            format!("/api/v1/issues/{}/files", an_issue(fixture).await?),
            None,
        ),
        "getFileRules" => (
            "GET",
            format!("/api/v1/issues/{}/file-rules", an_issue(fixture).await?),
            None,
        ),
        "deleteAttachment" => {
            let issue = an_issue(fixture).await?;
            let file = admins_file(fixture, &issue).await?;
            ("DELETE", format!("/api/v1/attachments/{file}"), None)
        }
        "restoreAttachment" => {
            let issue = an_issue(fixture).await?;
            let file = admins_file(fixture, &issue).await?;
            fixture
                .rig()
                .api()
                .delete_attachment(
                    Rig::info(),
                    DeleteAttachment {
                        session_token: fixture.admin_session().to_owned(),
                        attachment: file.parse().map_err(|_| "no attachment ID")?,
                    },
                )
                .await
                .map_err(|error| error.to_string())?;
            ("POST", format!("/api/v1/attachments/{file}/restore"), None)
        }
        "downloadFile" => {
            let issue = an_issue(fixture).await?;
            admins_file(fixture, &issue).await?;
            // The link is made for the person who asks: the actor's own list of the files, if they
            // may read the issue, else the administrator's (which opens nothing for anybody else).
            let own = browser
                .send("GET", &format!("/api/v1/issues/{issue}/files"), None)
                .await?;
            let link = if let Some(link) = own.body["items"][0]["download_url"].as_str() {
                link.to_owned()
            } else {
                let listed = fixture
                    .rig()
                    .api()
                    .issue_files(
                        Rig::info(),
                        IssueFilesQuery {
                            session_token: fixture.admin_session().to_owned(),
                            issue: issue.clone(),
                        },
                    )
                    .await
                    .map_err(|error| error.to_string())?
                    .into_value();
                let first = listed
                    .items
                    .first()
                    .ok_or("the matrix file is not listed")?;
                format!("{}/files/{}", bb_api::VERSION_PATH, first.download_link)
            };
            ("GET", link, None)
        }
        _ => return Ok(None),
    }))
}
