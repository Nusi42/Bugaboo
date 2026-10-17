//! The example requests of the routes of layouts (M3 WP-09).
//!
//! The routes work on the layout of the new-issue form of the target project. The ones that change
//! a layout make it first, as the administrator of the matrix, so each actor works on a layout that
//! is there. A save carries a token of a lock nobody holds: an actor who is let in is told that
//! the lock is lost, which is not a refusal of the permission.

use bb_app::layouts::CreateLayout;
use bb_core::layout::LayoutKind;
use bb_testing::Failure;
use serde_json::json;

use super::actors::Project;
use super::examples::Request;
use super::routes::Fixture;
use crate::sign_in_tests::rig::Rig;

/// A token that holds no lock.
const NOBODYS_LOCK: &str = "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f";

/// The layout of the new-issue form of the target project, made by the administrator of the matrix
/// when it is not there: its ID and its version.
async fn a_layout(fixture: &Fixture<'_>) -> Result<(String, i64), Failure> {
    let editor = fixture
        .rig()
        .api()
        .create_layout(
            Rig::info(),
            CreateLayout {
                session_token: fixture.admin_session().to_owned(),
                project: Some(fixture.project(Project::Target).to_string()),
                issue_type: None,
                kind: LayoutKind::NewIssue,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    let slot = editor.layout.ok_or("the layout was not made")?;
    Ok((slot.id.to_string(), slot.version))
}

pub(super) async fn layout_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    let target = fixture.project(Project::Target).to_string();
    Ok(Some(match operation {
        "getFormLayout" => (
            "GET",
            format!("/api/v1/layouts/form?project={target}&kind=new-issue"),
            None,
        ),
        "getLayoutEditor" => (
            "GET",
            format!("/api/v1/layouts/editor?project={target}&kind=new-issue"),
            None,
        ),
        "createLayout" => (
            "POST",
            "/api/v1/layouts".into(),
            Some(json!({"project": target, "kind": "new-issue"})),
        ),
        "saveLayout" => {
            let (layout, version) = a_layout(fixture).await?;
            (
                "PUT",
                format!("/api/v1/layouts/{layout}"),
                Some(json!({"version": version, "lock_token": NOBODYS_LOCK, "sections": []})),
            )
        }
        "deleteLayout" => {
            let (layout, version) = a_layout(fixture).await?;
            (
                "DELETE",
                format!("/api/v1/layouts/{layout}?version={version}"),
                None,
            )
        }
        _ => return Ok(None),
    }))
}
