//! The example requests of the editing routes (SEC-MATRIX): locks, drafts and the check of
//! versions, all about the description of the target project, which is the one kind of record
//! the server knows besides the notes of the tests.

use serde_json::{Value, json};

use super::actors::Project;
use super::examples::Request;
use super::routes::Fixture;

/// The thing the editing routes are about: the description of the target project.
fn editing_what(target: &str) -> Value {
    json!({"record": {"kind": "project", "id": target}, "part": "description"})
}

/// An open editor, as the browser would make its ID.
const AN_EDITOR: &str = "7c1d2e3f-4a5b-4c6d-8e7f-a0b1c2d3e4f5";

/// An ID that holds no lock, for the routes that ask for the token of one.
pub(super) const NO_LOCK_TOKEN: &str = "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f";

/// The request of an editing route, if the operation is one.
pub(super) fn editing_request(fixture: &Fixture<'_>, operation: &str) -> Option<Request> {
    let target = fixture.project(Project::Target).to_string();
    Some(match operation {
        // Editing: the record is the target project, its part the description.
        "takeEditLock" => (
            "POST",
            "/api/v1/edit-locks".into(),
            Some(json!({"target": editing_what(&target), "editor_id": AN_EDITOR})),
        ),
        "readEditLock" => (
            "GET",
            format!("/api/v1/edit-locks?record_kind=project&record_id={target}&part=description"),
            None,
        ),
        "heartbeatEditLock" => (
            "POST",
            "/api/v1/edit-locks/heartbeat".into(),
            Some(json!({
                "target": editing_what(&target),
                "lock_token": NO_LOCK_TOKEN,
                "draft": {"text": "Typed by the matrix", "base_version": 1}
            })),
        ),
        "releaseEditLock" => (
            "POST",
            "/api/v1/edit-locks/release".into(),
            Some(json!({"target": editing_what(&target), "lock_token": NO_LOCK_TOKEN})),
        ),
        "askToFinish" => (
            "POST",
            "/api/v1/edit-locks/ask-to-finish".into(),
            Some(json!({"target": editing_what(&target)})),
        ),
        "editAnyway" => (
            "POST",
            "/api/v1/edit-locks/edit-anyway".into(),
            Some(json!({"target": editing_what(&target), "editor_id": AN_EDITOR})),
        ),
        "releaseStuckEditLock" => (
            "POST",
            "/api/v1/edit-locks/release-stuck".into(),
            Some(json!({"target": editing_what(&target)})),
        ),
        "saveDraft" => (
            "PUT",
            "/api/v1/drafts".into(),
            Some(json!({
                "target": {"record": {"kind": "project", "id": target}, "part": "description"},
                "draft": {"text": "Typed by the matrix", "base_version": 1}
            })),
        ),
        "readDraft" => (
            "GET",
            format!("/api/v1/drafts?record_kind=project&record_id={target}&part=description"),
            None,
        ),
        "throwAwayDraft" => (
            "POST",
            "/api/v1/drafts/throw-away".into(),
            Some(json!({"record": {"kind": "project", "id": target}, "part": "description"})),
        ),
        "checkVersions" => (
            "POST",
            "/api/v1/versions/check".into(),
            Some(json!({"records": [{"kind": "project", "id": target}]})),
        ),
        _ => return None,
    })
}
