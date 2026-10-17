//! The example requests of the routes of workflows (M4 WP-04).

use bb_testing::Failure;

use super::actors::Project;
use super::examples::Request;
use super::routes::Fixture;

pub(super) fn workflow_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    let target = fixture.project(Project::Target).to_string();
    Ok(Some(match operation {
        "getProjectStatuses" => ("GET", format!("/api/v1/projects/{target}/statuses"), None),
        _ => return Ok(None),
    }))
}
