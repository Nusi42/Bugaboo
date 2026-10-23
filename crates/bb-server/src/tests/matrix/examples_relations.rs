//! The example requests of the routes of relations, comments, mentions, history and following
//! (M3 WP-07, SEC-MATRIX).
//!
//! Every example that works on one issue makes a new one first, reported by the administrator of
//! the matrix in the target project (`examples_issues::an_issue`). A route that works on a comment
//! lets the actor write the comment themselves when they may, so a person who holds *Comment* is
//! its author and is let in, while one who may not gets the comment of the administrator and is
//! refused for the reason the table says.

use bb_app::comments::AddComment;
use bb_app::relations::{AddRelation, CreateRelationKind, RelationKindsQuery};
use bb_core::relation::Direction;
use bb_testing::Failure;
use serde_json::json;

use super::examples::Request;
use super::examples_issues::{an_issue, version_of};
use super::routes::Fixture;
use crate::sign_in_tests::http::Browser;
use crate::sign_in_tests::rig::Rig;

/// An editor id for the lock requests of the matrix.
const AN_EDITOR: &str = "9d8c7b6a-5f4e-4d3c-8b2a-1f0e9d8c7b6a";

/// An ID that holds no lock and no kind, for the routes that ask for one.
const NOBODYS_RANDOM_ID: &str = "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f";

/// The ID of the first kind of relation, from the start.
async fn a_kind(fixture: &Fixture<'_>) -> Result<String, Failure> {
    let page = fixture
        .rig()
        .api()
        .relation_kinds(
            Rig::info(),
            RelationKindsQuery {
                session_token: fixture.admin_session().to_owned(),
                cursor: None,
                limit: 1,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    page.items
        .first()
        .map(|found| found.id.to_string())
        .ok_or_else(|| "the matrix has no kind of relation".into())
}

/// A comment under a new issue of the target project, written by the administrator: its ID.
async fn admins_comment(fixture: &Fixture<'_>, issue: &str) -> Result<String, Failure> {
    Ok(fixture
        .rig()
        .api()
        .add_comment(
            Rig::info(),
            AddComment {
                session_token: fixture.admin_session().to_owned(),
                issue: issue.to_owned(),
                text: format!("Comment {}", fixture.next_number()),
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value()
        .id
        .to_string())
}

/// A comment under `issue`, written by the actor if they may and by the administrator if not.
async fn comment_for(
    fixture: &Fixture<'_>,
    browser: &mut Browser,
    issue: &str,
) -> Result<String, Failure> {
    let written = browser
        .send(
            "POST",
            &format!("/api/v1/issues/{issue}/comments"),
            Some(json!({"text": format!("Comment {}", fixture.next_number())})),
        )
        .await?;
    match written.body["id"].as_str() {
        Some(id) if written.status.is_success() => Ok(id.to_owned()),
        _ => admins_comment(fixture, issue).await,
    }
}

/// The lock token the actor gets on the text of `comment`, or one that holds no lock.
async fn lock_for(browser: &mut Browser, comment: &str) -> Result<String, Failure> {
    let taken = browser
        .send(
            "POST",
            "/api/v1/edit-locks",
            Some(json!({
                "target": {"record": {"kind": "comment", "id": comment}, "part": "text"},
                "editor_id": AN_EDITOR,
            })),
        )
        .await?;
    Ok(taken.body["lock_token"]
        .as_str()
        .unwrap_or(NOBODYS_RANDOM_ID)
        .to_owned())
}

/// The request of a route of relations, comments, history or following, if the operation is one.
pub(super) async fn relation_request(
    fixture: &Fixture<'_>,
    operation: &str,
    browser: &mut Browser,
) -> Result<Option<Request>, Failure> {
    Ok(Some(match operation {
        "listRelationKinds" => ("GET", "/api/v1/relation-kinds".into(), None),
        "createRelationKind" => (
            "POST",
            "/api/v1/relation-kinds".into(),
            Some(json!({
                "forward": format!("precedes {}", fixture.next_number()),
                "backward": format!("follows {}", fixture.next_number()),
            })),
        ),
        "changeRelationKind" => {
            let made = fixture
                .rig()
                .api()
                .create_relation_kind(
                    Rig::info(),
                    CreateRelationKind {
                        session_token: fixture.admin_session().to_owned(),
                        forward: format!("leads to {}", fixture.next_number()),
                        backward: format!("comes from {}", fixture.next_number()),
                    },
                )
                .await
                .map_err(|error| error.to_string())?
                .into_value();
            (
                "PATCH",
                format!("/api/v1/relation-kinds/{}", made.id),
                Some(json!({"version": 1, "retired": false, "forward": made.forward})),
            )
        }
        "getIssueRelations" => (
            "GET",
            format!("/api/v1/issues/{}/relations", an_issue(fixture).await?),
            None,
        ),
        "listSubIssues" => (
            "GET",
            format!("/api/v1/issues/{}/sub-issues", an_issue(fixture).await?),
            None,
        ),
        "listRelatedIssues" => (
            "GET",
            format!("/api/v1/issues/{}/related-issues", an_issue(fixture).await?),
            None,
        ),
        "setIssueParent" => {
            let child = an_issue(fixture).await?;
            let parent = an_issue(fixture).await?;
            let version = version_of(fixture, &child).await?;
            (
                "PUT",
                format!("/api/v1/issues/{child}/parent"),
                Some(json!({"version": version, "parent": parent})),
            )
        }
        "addRelation" => {
            let first = an_issue(fixture).await?;
            let other = an_issue(fixture).await?;
            (
                "POST",
                format!("/api/v1/issues/{first}/relations"),
                Some(json!({
                    "other": other,
                    "kind": a_kind(fixture).await?,
                    "direction": "forward",
                })),
            )
        }
        "removeRelation" => {
            let first = an_issue(fixture).await?;
            let other = an_issue(fixture).await?;
            let added = fixture
                .rig()
                .api()
                .add_relation(
                    Rig::info(),
                    AddRelation {
                        session_token: fixture.admin_session().to_owned(),
                        issue: first,
                        other,
                        kind: a_kind(fixture).await?.parse().map_err(|_| "no kind ID")?,
                        direction: Direction::Forward,
                    },
                )
                .await
                .map_err(|error| error.to_string())?
                .into_value();
            ("DELETE", format!("/api/v1/relations/{}", added.id), None)
        }
        _ => return text_request(fixture, operation, browser).await,
    }))
}

/// The request of a route of comments, mentions, history or following, if the operation is one.
async fn text_request(
    fixture: &Fixture<'_>,
    operation: &str,
    browser: &mut Browser,
) -> Result<Option<Request>, Failure> {
    Ok(Some(match operation {
        "addComment" => (
            "POST",
            format!("/api/v1/issues/{}/comments", an_issue(fixture).await?),
            Some(json!({"text": "Written by the matrix."})),
        ),
        "saveComment" => {
            let issue = an_issue(fixture).await?;
            let comment = comment_for(fixture, browser, &issue).await?;
            let token = lock_for(browser, &comment).await?;
            (
                "PUT",
                format!("/api/v1/comments/{comment}"),
                Some(json!({"version": 1, "lock_token": token, "text": "Changed by the matrix."})),
            )
        }
        "deleteComment" => {
            let issue = an_issue(fixture).await?;
            let comment = comment_for(fixture, browser, &issue).await?;
            ("DELETE", format!("/api/v1/comments/{comment}"), None)
        }
        "restoreComment" => {
            let issue = an_issue(fixture).await?;
            let comment = comment_for(fixture, browser, &issue).await?;
            fixture
                .rig()
                .api()
                .delete_comment(
                    Rig::info(),
                    bb_app::comments::DeleteComment {
                        session_token: fixture.admin_session().to_owned(),
                        comment: comment.parse().map_err(|_| "no comment ID")?,
                    },
                )
                .await
                .map_err(|error| error.to_string())?;
            ("POST", format!("/api/v1/comments/{comment}/restore"), None)
        }
        "listCommentVersions" => {
            let issue = an_issue(fixture).await?;
            let comment = admins_comment(fixture, &issue).await?;
            ("GET", format!("/api/v1/comments/{comment}/versions"), None)
        }
        "listMentionChoices" => (
            "GET",
            format!(
                "/api/v1/issues/{}/mention-choices",
                an_issue(fixture).await?
            ),
            None,
        ),
        "listIssueHistory" => (
            "GET",
            format!("/api/v1/issues/{}/history", an_issue(fixture).await?),
            None,
        ),
        "listFollowers" => (
            "GET",
            format!("/api/v1/issues/{}/followers", an_issue(fixture).await?),
            None,
        ),
        "getFollowing" => (
            "GET",
            format!("/api/v1/issues/{}/follow", an_issue(fixture).await?),
            None,
        ),
        "followIssue" => (
            "PUT",
            format!("/api/v1/issues/{}/follow", an_issue(fixture).await?),
            None,
        ),
        "unfollowIssue" => (
            "DELETE",
            format!("/api/v1/issues/{}/follow", an_issue(fixture).await?),
            None,
        ),
        _ => return Ok(None),
    }))
}
