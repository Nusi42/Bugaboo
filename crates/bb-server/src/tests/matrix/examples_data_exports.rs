//! The example requests of the routes of *Download my data* (PRIV-03).
//!
//! The administrator's export is asked for a person of the matrix with a reason; while one waits,
//! a second is refused as `data-export.waiting`, which is no refusal of the permission. No job runs
//! in the matrix, so no file is ready: the link of an export and a download with a link that is not
//! ours open nothing for anybody, and what the matrix proves of them is that a session is needed.

use bb_testing::Failure;
use serde_json::json;

use super::examples::Request;
use super::routes::Fixture;

/// An ID of an export that does not exist, and a link this server never signed.
const NO_EXPORT: &str = "0191d3a4-0000-7000-8000-000000000000";
const NO_LINK: &str = "not-a-signed-link";

pub(super) fn data_export_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    Ok(Some(match operation {
        "askForMyData" => ("POST", "/api/v1/me/data-export".to_owned(), None),
        "getMyDataExport" => ("GET", "/api/v1/me/data-export".to_owned(), None),
        "askForSomeonesData" => (
            "POST",
            format!("/api/v1/people/{}/data-export", fixture.a_person()),
            Some(json!({"reason": "The person asked by letter"})),
        ),
        "listDataExports" => ("GET", "/api/v1/admin/data-exports".to_owned(), None),
        "getDataExportAddress" => (
            "GET",
            format!("/api/v1/data-exports/{NO_EXPORT}/address"),
            None,
        ),
        "downloadDataExport" => (
            "GET",
            format!("/api/v1/data-exports/download/{NO_LINK}"),
            None,
        ),
        _ => return Ok(None),
    }))
}
