//! The example requests of the routes of the releases of a project (ISS-08): each works on a release
//! of the target project that the administrator of the matrix made for it, so every actor meets the
//! same kind of release and a refusal is the refusal of the permission.

use bb_app::releases::{CreateRelease, DeleteRelease};
use bb_testing::Failure;
use serde_json::json;

use super::actors::Project;
use super::examples::Request;
use super::routes::Fixture;
use crate::sign_in_tests::rig::Rig;

/// A new release of the target project, made by the administrator of the matrix.
async fn a_release(fixture: &Fixture<'_>) -> Result<String, Failure> {
    Ok(fixture
        .rig()
        .api()
        .create_release(
            Rig::info(),
            CreateRelease {
                session_token: fixture.admin_session().to_owned(),
                project: fixture.project(Project::Target).to_string(),
                name: format!("Release {}", fixture.next_number()),
                release_date: None,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value()
        .id
        .to_string())
}

/// A release of the target project in the trash.
async fn a_deleted_release(fixture: &Fixture<'_>) -> Result<String, Failure> {
    let release = a_release(fixture).await?;
    fixture
        .rig()
        .api()
        .delete_release(
            Rig::info(),
            DeleteRelease {
                session_token: fixture.admin_session().to_owned(),
                project: fixture.project(Project::Target).to_string(),
                release: release.parse().map_err(|_| "a release ID")?,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(release)
}

pub(super) async fn release_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    let target = fixture.project(Project::Target).to_string();
    let releases = format!("/api/v1/projects/{target}/releases");
    Ok(Some(match operation {
        "listReleases" => ("GET", releases, None),
        "createRelease" => (
            "POST",
            releases,
            Some(json!({"name": format!("Release {}", fixture.next_number())})),
        ),
        "changeRelease" => (
            "PATCH",
            format!("{releases}/{}", a_release(fixture).await?),
            Some(json!({"version": 1, "state": "released"})),
        ),
        "deleteRelease" => (
            "DELETE",
            format!("{releases}/{}", a_release(fixture).await?),
            None,
        ),
        "restoreRelease" => (
            "POST",
            format!("{releases}/{}/restore", a_deleted_release(fixture).await?),
            None,
        ),
        _ => return Ok(None),
    }))
}
