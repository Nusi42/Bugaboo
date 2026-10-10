//! Following an issue (ISS-07): `/api/v1/issues/{issue}/follow` for the person, and
//! `/api/v1/issues/{issue}/followers` for who follows.
//!
//! Anybody who sees an issue may follow it. The followers are shown as the kind of people
//! information *Followers* says for the reader. Delivery (e-mail, the bell) comes in M4.

use axum::extract::State;
use axum::response::Response;
use bb_app::follow::{FollowIssue, FollowersQuery, UnfollowIssue};
use bb_core::follow::FollowReason;
use bb_policy::{Masked, PersonRef};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::cursor_error;
use super::auth::{
    AuthHandle, Info, NotFoundAnswer, NotSignedInAnswer, Session, answer, answer_authorized, done,
};
use crate::errors::{ApiError, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

const FOLLOWERS_LIST: &str = "followers";

/// What a person in a list of followers is called.
const PERSON_FIELDS: [FieldSpec; 3] = [
    FieldSpec::by_default("shown"),
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
];

/// Whether the person follows an issue.
#[derive(Serialize, ToSchema)]
pub(crate) struct FollowingBody {
    /// Whether the person follows the issue.
    following: bool,
    /// Why they follow it, when they do.
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<FollowReason>,
}

impl Masked for FollowingBody {}

/// Who follows an issue, a page at a time, each as the kind *Followers* says for the reader.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/followers",
    operation_id = "listFollowers",
    tag = "follow",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue."), ListParams),
    responses((status = 200, description = "The followers.", body = Page<PersonRef>), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_followers(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let position = params.position(FOLLOWERS_LIST);
    let cursor = match &position {
        Ok(position) => position.clone(),
        Err(_) => Some(String::new()),
    };
    let page = auth
        .api()?
        .followers(
            info,
            FollowersQuery {
                session_token,
                issue,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    position?;
    let list = page.map(|page| Listed {
        items: page.items,
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&PERSON_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(FOLLOWERS_LIST, &list, &fields)?))
}

/// Whether the person follows the issue (*Follow* or *Following*).
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/follow",
    operation_id = "getFollowing",
    tag = "follow",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    responses((status = 200, description = "Whether the person follows it.", body = FollowingBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_following(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
) -> Result<Response, ApiError> {
    let state = auth
        .api()?
        .followers(
            info,
            FollowersQuery {
                session_token,
                issue,
                cursor: None,
                limit: 1,
            },
        )
        .await?;
    answer_authorized(&state.map(|followers| FollowingBody {
        following: followers.following,
        reason: followers.reason,
    }))
}

/// Makes the person a follower of the issue. Following it already changes nothing.
#[utoipa::path(
    put,
    path = "/api/v1/issues/{issue}/follow",
    operation_id = "followIssue",
    tag = "follow",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    responses((status = 204, description = "The person follows the issue."), NotSignedInAnswer, NotFoundAnswer)
)]
async fn follow_issue(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
) -> Result<Response, ApiError> {
    auth.api()?
        .follow_issue(
            info,
            FollowIssue {
                session_token,
                issue,
            },
        )
        .await?;
    Ok(done())
}

/// Takes the person out of the followers of the issue. Not following changes nothing.
#[utoipa::path(
    delete,
    path = "/api/v1/issues/{issue}/follow",
    operation_id = "unfollowIssue",
    tag = "follow",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    responses((status = 204, description = "The person does not follow the issue."), NotSignedInAnswer, NotFoundAnswer)
)]
async fn unfollow_issue(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
) -> Result<Response, ApiError> {
    auth.api()?
        .unfollow_issue(
            info,
            UnfollowIssue {
                session_token,
                issue,
            },
        )
        .await?;
    Ok(done())
}

/// The routes of following.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_followers))
        .routes(routes!(get_following, follow_issue, unfollow_issue))
}
