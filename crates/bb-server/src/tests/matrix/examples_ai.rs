//! The example requests of the routes of *Never sent to AI* (M7 WP-03).
//!
//! The routes that set a mark take the version of the mark, so each request reads it first as the
//! administrator of the matrix: every actor then writes over a mark that is at the version they
//! ask for.

use bb_app::ai::NeverSentQuery;
use bb_testing::Failure;
use serde_json::json;

use super::actors::Project;
use super::examples::Request;
use super::routes::Fixture;
use crate::sign_in_tests::rig::Rig;

/// The word of the built-in field the matrix marks.
const FIELD: &str = "description";

/// The version of the mark of the target project, and of the field, as they are now.
async fn versions(fixture: &Fixture<'_>) -> Result<(i64, i64), Failure> {
    let list = fixture
        .rig()
        .api()
        .never_sent_list(
            Rig::info(),
            NeverSentQuery {
                session_token: fixture.admin_session().to_owned(),
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    let target = fixture.project(Project::Target);
    let project = list
        .projects
        .iter()
        .find(|each| each.project.id == target)
        .map_or(0, |each| each.version);
    let field = list
        .fields
        .iter()
        .find(|each| each.field.code() == FIELD)
        .map_or(0, |each| each.version);
    Ok((project, field))
}

pub(super) async fn ai_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    let target = fixture.project(Project::Target).to_string();
    Ok(Some(match operation {
        "listNeverSentToAi" => ("GET", "/api/v1/never-sent-to-ai".to_owned(), None),
        "setFieldNeverSentToAi" => (
            "PUT",
            format!("/api/v1/never-sent-to-ai/fields/{FIELD}"),
            Some(json!({"marked": true, "version": versions(fixture).await?.1})),
        ),
        "setProjectNeverSentToAi" => (
            "PUT",
            format!("/api/v1/never-sent-to-ai/projects/{target}"),
            Some(json!({"marked": true, "version": versions(fixture).await?.0})),
        ),
        _ => return Ok(None),
    }))
}
