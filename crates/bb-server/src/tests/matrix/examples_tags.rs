//! The example requests of the routes of tags, labels, moving an issue and changing many issues
//! at once (M3 WP-08, SEC-MATRIX).
//!
//! Every example that works on one issue makes a new one first, reported by the administrator of
//! the matrix in the target project (`examples_issues::an_issue`). The routes of the people who
//! look after tags and labels make what they work on first, as the administrator.

use bb_app::tags::{AddTag, CreateLabel, CreateLabelGroup, LabelGroupsQuery};
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
    let labels_and_tags = fixture
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
    labels_and_tags
        .tags
        .first()
        .map(|tag| tag.id.to_string())
        .ok_or_else(|| "the tag is not on the issue".into())
}

/// A group of labels, created by the administrator for the whole organization or in the project:
/// its ID.
async fn a_group(fixture: &Fixture<'_>, project: Option<String>) -> Result<String, Failure> {
    Ok(fixture
        .rig()
        .api()
        .create_label_group(
            Rig::info(),
            CreateLabelGroup {
                session_token: fixture.admin_session().to_owned(),
                project,
                name: format!("group {}", fixture.next_number()),
                pick_one: false,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value()
        .id
        .to_string())
}

/// A group of labels created in the target project, the one that is there already if there is one
/// (a route that only reads takes the same group for every actor, so the matrix does not use up the
/// room of the project).
async fn a_group_of_the_target(fixture: &Fixture<'_>) -> Result<String, Failure> {
    let target = target_of(fixture);
    let listed = fixture
        .rig()
        .api()
        .label_groups(
            Rig::info(),
            LabelGroupsQuery {
                session_token: fixture.admin_session().to_owned(),
                project: Some(target.clone()),
                cursor: None,
                limit: 100,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    match listed.items.first() {
        Some(found) => Ok(found.id.to_string()),
        None => a_group(fixture, Some(target)).await,
    }
}

/// A label made by the administrator, in no group: its ID.
async fn a_label(fixture: &Fixture<'_>) -> Result<String, Failure> {
    label_in(fixture, None).await
}

/// A label in a group created in the target project: its ID.
async fn a_target_label(fixture: &Fixture<'_>) -> Result<String, Failure> {
    let group = a_group(fixture, Some(target_of(fixture))).await?;
    label_in(fixture, Some(group)).await
}

/// The target project, as the requests name it.
fn target_of(fixture: &Fixture<'_>) -> String {
    fixture.project(super::actors::Project::Target).to_string()
}

async fn label_in(fixture: &Fixture<'_>, group: Option<String>) -> Result<String, Failure> {
    Ok(fixture
        .rig()
        .api()
        .create_label(
            Rig::info(),
            CreateLabel {
                session_token: fixture.admin_session().to_owned(),
                name: format!("label {}", fixture.next_number()),
                color: None,
                group: group
                    .map(|each| each.parse().map_err(|_| "no group ID"))
                    .transpose()?,
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
        "getIssueLabelsAndTags" => (
            "GET",
            format!(
                "/api/v1/issues/{}/labels-and-tags",
                an_issue(fixture).await?
            ),
            None,
        ),
        "addTag" => (
            "POST",
            format!("/api/v1/issues/{}/tags", an_issue(fixture).await?),
            Some(json!({"name": format!("tag {}", fixture.next_number())})),
        ),
        "removeTag" => {
            let issue = an_issue(fixture).await?;
            let labels_and_tags = fixture
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
            let tag = labels_and_tags
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
        "listLabelGroups" => ("GET", "/api/v1/label-groups".into(), None),
        "createLabelGroup" => (
            "POST",
            "/api/v1/label-groups".into(),
            Some(json!({
                "name": format!("group {}", fixture.next_number()),
                "pick_one": true,
                "project": target_of(fixture),
            })),
        ),
        "changeLabelGroup" => (
            "PATCH",
            format!(
                "/api/v1/label-groups/{}",
                a_group(fixture, Some(target_of(fixture))).await?
            ),
            Some(json!({"version": 1, "name": format!("renamed {}", fixture.next_number())})),
        ),
        "listLabelGroupOffers" => (
            "GET",
            format!(
                "/api/v1/label-groups/{}/offers",
                a_group_of_the_target(fixture).await?
            ),
            None,
        ),
        "changeLabelGroupOffers" => (
            "PUT",
            format!(
                "/api/v1/label-groups/{}/offers",
                a_group(fixture, None).await?
            ),
            Some(json!({
                "version": 1,
                "changes": [{
                    "project": fixture.project(super::actors::Project::Target).to_string(),
                    "offered": false,
                }],
            })),
        ),
        "createLabel" => (
            "POST",
            "/api/v1/labels".into(),
            Some(json!({
                "name": format!("label {}", fixture.next_number()),
                "label_group": a_group(fixture, Some(target_of(fixture))).await?,
            })),
        ),
        "previewLabelChange" => (
            "POST",
            format!("/api/v1/labels/{}/preview", a_target_label(fixture).await?),
            Some(json!({"name": format!("renamed {}", fixture.next_number())})),
        ),
        "changeLabel" => (
            "PATCH",
            format!("/api/v1/labels/{}", a_target_label(fixture).await?),
            Some(json!({"version": 1, "color": "cat6"})),
        ),
        "combineLabels" => (
            "POST",
            format!("/api/v1/labels/{}/combine", a_target_label(fixture).await?),
            Some(json!({"into": a_target_label(fixture).await?, "version": 1, "into_version": 1})),
        ),
        _ => return many_changes_request(fixture, operation).await,
    }))
}

/// The request of a route of moving an issue or changing many issues, if the operation is one.
async fn many_changes_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
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
        "previewManyChanges" => (
            "POST",
            "/api/v1/many-changes/preview".into(),
            Some(json!({
                "issues": [an_issue(fixture).await?],
                "action": {"kind": "change", "change": {"status": "done"}},
            })),
        ),
        "applyManyChanges" => {
            let issue = an_issue(fixture).await?;
            let version = version_of(fixture, &issue).await?;
            (
                "POST",
                "/api/v1/many-changes".into(),
                Some(json!({
                    "issues": [{"issue": issue, "version": version}],
                    "action": {"kind": "change", "change": {"status": "done"}},
                })),
            )
        }
        "getManyChangeRun" => (
            "GET",
            format!("/api/v1/many-changes/runs/{NOBODYS_ID}"),
            None,
        ),
        "stopManyChangeRun" => (
            "POST",
            format!("/api/v1/many-changes/runs/{NOBODYS_ID}/stop"),
            None,
        ),
        _ => return Ok(None),
    }))
}
