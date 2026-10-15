//! The example requests of the routes of tags, labels, moving an issue and changing many issues
//! at once (M3 WP-08, SEC-MATRIX).
//!
//! Every example that works on one issue makes a new one first, reported by the administrator of
//! the matrix in the target project (`examples_issues::an_issue`). The routes of the people who
//! look after tags and labels make what they work on first, as the administrator.

use bb_app::tags::{AddTag, MakeLabel, MakeLabelGroup};
use bb_testing::Failure;
use serde_json::json;

use super::examples::Request;
use super::examples_issues::{an_issue, version_of};
use super::routes::Fixture;
use crate::sign_in_tests::rig::Rig;

/// An ID that no tag, label, group or run has (a time-ordered ID, as they have).
const NOBODYS_ID: &str = "01890a5d-ac96-774b-bcce-b302099a8057";

/// A tag put on a new issue of the target project by the administrator: the ID of the tag.
async fn a_tag(fixture: &Fixture<'_>) -> Result<String, Failure> {
    let issue = an_issue(fixture).await?;
    let sorting = fixture
        .rig()
        .api()
        .add_tag(
            Rig::info(),
            AddTag {
                session_token: fixture.admin_session().to_owned(),
                issue,
                name: format!("tag {}", fixture.next_number()),
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    sorting
        .tags
        .first()
        .map(|tag| tag.id.to_string())
        .ok_or_else(|| "the tag is not on the issue".into())
}

/// A label made by the administrator: its ID.
async fn a_label(fixture: &Fixture<'_>) -> Result<String, Failure> {
    Ok(fixture
        .rig()
        .api()
        .make_label(
            Rig::info(),
            MakeLabel {
                session_token: fixture.admin_session().to_owned(),
                name: format!("label {}", fixture.next_number()),
                color: None,
                group: None,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value()
        .id
        .to_string())
}

/// The request of a route of tags, labels, moving or changing many issues, if the operation is one.
pub(super) async fn tag_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    Ok(Some(match operation {
        "getIssueSorting" => (
            "GET",
            format!("/api/v1/issues/{}/sorting", an_issue(fixture).await?),
            None,
        ),
        "addTag" => (
            "POST",
            format!("/api/v1/issues/{}/tags", an_issue(fixture).await?),
            Some(json!({"name": format!("tag {}", fixture.next_number())})),
        ),
        "removeTag" => {
            let issue = an_issue(fixture).await?;
            let sorting = fixture
                .rig()
                .api()
                .add_tag(
                    Rig::info(),
                    AddTag {
                        session_token: fixture.admin_session().to_owned(),
                        issue: issue.clone(),
                        name: format!("tag {}", fixture.next_number()),
                    },
                )
                .await
                .map_err(|error| error.to_string())?
                .into_value();
            let tag = sorting
                .tags
                .first()
                .ok_or("the tag is not on the issue")?
                .id;
            ("DELETE", format!("/api/v1/issues/{issue}/tags/{tag}"), None)
        }
        "addLabel" => (
            "POST",
            format!("/api/v1/issues/{}/labels", an_issue(fixture).await?),
            Some(json!({"label": a_label(fixture).await?})),
        ),
        "removeLabel" => (
            "DELETE",
            format!(
                "/api/v1/issues/{}/labels/{}",
                an_issue(fixture).await?,
                NOBODYS_ID
            ),
            None,
        ),
        _ => return admin_request(fixture, operation).await,
    }))
}

/// The request of a route of the people who look after tags and labels, if the operation is one.
async fn admin_request(fixture: &Fixture<'_>, operation: &str) -> Result<Option<Request>, Failure> {
    Ok(Some(match operation {
        "listTagSuggestions" => (
            "GET",
            format!(
                "/api/v1/projects/{}/tag-suggestions?text=ta",
                fixture.project(super::actors::Project::Target)
            ),
            None,
        ),
        "listTags" => ("GET", "/api/v1/tags".into(), None),
        "previewTagChange" => (
            "POST",
            format!("/api/v1/tags/{}/preview", a_tag(fixture).await?),
            Some(json!({"name": format!("renamed {}", fixture.next_number())})),
        ),
        "changeTag" => (
            "PATCH",
            format!("/api/v1/tags/{}", a_tag(fixture).await?),
            Some(json!({"version": 1, "color": "cat5"})),
        ),
        "combineTags" => (
            "POST",
            format!("/api/v1/tags/{}/combine", a_tag(fixture).await?),
            Some(json!({"into": a_tag(fixture).await?, "version": 1, "into_version": 1})),
        ),
        "listLabels" => ("GET", "/api/v1/labels".into(), None),
        "makeLabelGroup" => (
            "POST",
            "/api/v1/label-groups".into(),
            Some(json!({"name": format!("group {}", fixture.next_number()), "pick_one": true})),
        ),
        "changeLabelGroup" => {
            let made = fixture
                .rig()
                .api()
                .make_label_group(
                    Rig::info(),
                    MakeLabelGroup {
                        session_token: fixture.admin_session().to_owned(),
                        name: format!("group {}", fixture.next_number()),
                        pick_one: false,
                    },
                )
                .await
                .map_err(|error| error.to_string())?
                .into_value();
            (
                "PATCH",
                format!("/api/v1/label-groups/{}", made.id),
                Some(json!({"version": 1, "name": format!("renamed {}", fixture.next_number())})),
            )
        }
        "makeLabel" => (
            "POST",
            "/api/v1/labels".into(),
            Some(json!({"name": format!("label {}", fixture.next_number())})),
        ),
        "previewLabelChange" => (
            "POST",
            format!("/api/v1/labels/{}/preview", a_label(fixture).await?),
            Some(json!({"name": format!("renamed {}", fixture.next_number())})),
        ),
        "changeLabel" => (
            "PATCH",
            format!("/api/v1/labels/{}", a_label(fixture).await?),
            Some(json!({"version": 1, "color": "cat6"})),
        ),
        "combineLabels" => (
            "POST",
            format!("/api/v1/labels/{}/combine", a_label(fixture).await?),
            Some(json!({"into": a_label(fixture).await?, "version": 1, "into_version": 1})),
        ),
        _ => return bulk_request(fixture, operation).await,
    }))
}

/// The request of a route of moving an issue or changing many issues, if the operation is one.
async fn bulk_request(fixture: &Fixture<'_>, operation: &str) -> Result<Option<Request>, Failure> {
    Ok(Some(match operation {
        "moveIssue" => {
            let issue = an_issue(fixture).await?;
            let version = version_of(fixture, &issue).await?;
            (
                "POST",
                format!("/api/v1/issues/{issue}/move"),
                Some(json!({
                    "version": version,
                    "project": fixture.project(super::actors::Project::Target).to_string(),
                })),
            )
        }
        "previewBulkChange" => (
            "POST",
            "/api/v1/bulk/preview".into(),
            Some(json!({
                "issues": [an_issue(fixture).await?],
                "action": {"kind": "change", "change": {"status": "done"}},
            })),
        ),
        "applyBulkChange" => {
            let issue = an_issue(fixture).await?;
            let version = version_of(fixture, &issue).await?;
            (
                "POST",
                "/api/v1/bulk/changes".into(),
                Some(json!({
                    "issues": [{"issue": issue, "version": version}],
                    "action": {"kind": "change", "change": {"status": "done"}},
                })),
            )
        }
        "getBulkRun" => ("GET", format!("/api/v1/bulk/runs/{NOBODYS_ID}"), None),
        "cancelBulkRun" => (
            "POST",
            format!("/api/v1/bulk/runs/{NOBODYS_ID}/cancel"),
            None,
        ),
        _ => return Ok(None),
    }))
}
