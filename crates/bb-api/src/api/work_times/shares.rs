//! The share of a person in a project (PLAN-15, ADR-0030, decision 2 and amendment M8), below
//! `/api/v1/projects/{project}/shares`: the people of the project with their shares, and setting
//! one. Setting needs *Decide who has which role* in the project, which a Project lead has at the
//! start; no new permission. A person with no share set has 100 percent.

use axum::extract::State;
use axum::response::Response;
use bb_app::work_times::{MemberShare, ProjectSharesQuery, SetShare};
use bb_core::id::UserId;
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::super::access_bodies::cursor_error;
use super::super::access_keys::ProjectLabelBody;
use super::super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer, answer_authorized,
};
use super::super::path_id;
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The fields of a person of a project in the list that can be asked for.
const MEMBER_FIELDS: [FieldSpec; 5] = [
    FieldSpec::by_default("person"),
    FieldSpec::by_default("percent"),
    FieldSpec::by_default("over_100"),
    FieldSpec::by_default("others"),
    FieldSpec::by_default("others_hidden"),
];

/// The name of the list, so its cursors are its own.
const SHARES_LIST: &str = "project-shares";

/// A person of a project with their share there.
#[derive(Serialize, ToSchema)]
pub(crate) struct MemberShareBody {
    /// The person, as the reader sees people of the kind *Work times*.
    person: PersonRef,
    /// The share in the project, in percent.
    percent: u8,
    /// Whether the shares of the person that are in force add up to more than 100 percent; only for
    /// a reader who decides who has which role in the project. Never the sum itself: it would tell
    /// how much the projects the reader may not see hold.
    #[serde(skip_serializing_if = "Option::is_none")]
    over_100: Option<bool>,
    /// The other projects the person has a share in, as the reader may name them; only for a reader
    /// who decides who has which role.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    others: Vec<ProjectLabelBody>,
    /// Whether the person has a share in another project that the reader may not see: it is never
    /// named.
    others_hidden: bool,
}

impl Masked for MemberShareBody {}

impl From<&MemberShare> for MemberShareBody {
    fn from(member: &MemberShare) -> Self {
        Self {
            person: member.person.clone(),
            percent: member.percent,
            over_100: member.over_100,
            others: member.others.iter().map(ProjectLabelBody::from).collect(),
            others_hidden: member.others_hidden,
        }
    }
}

/// A share.
#[derive(Serialize, Deserialize, ToSchema)]
pub(crate) struct ShareBody {
    /// The share in percent, 0 to 100.
    #[schema(minimum = 0, maximum = 100)]
    percent: u8,
}

impl Masked for ShareBody {}

impl From<&bb_app::work_times::ChangedShare> for ShareBody {
    fn from(changed: &bb_app::work_times::ChangedShare) -> Self {
        Self {
            percent: changed.percent,
        }
    }
}

/// The people of a project with their shares there, by name, a page at a time. Every reader who
/// sees the project and is shown people of the kind *Work times* reads the shares; a reader who
/// decides who has which role there is also told whether a person's shares add up to more than 100
/// percent, with the other projects they may see named and a mark when there are ones they may not.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{project}/shares",
    operation_id = "listProjectShares",
    tag = "work-times",
    params(("project" = String, Path, description = "The ID of the project."), ListParams),
    responses(
        (status = 200, description = "The people with their shares.", body = Page<MemberShareBody>),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn list_project_shares(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(SHARES_LIST);
    let found = auth
        .api()?
        .project_shares(
            info,
            ProjectSharesQuery {
                session_token,
                project,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    let list = found.map(|page| Listed {
        items: page.items.iter().map(MemberShareBody::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&MEMBER_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(SHARES_LIST, &list, &fields)?))
}

/// Sets the share of a person in a project. Needs *Decide who has which role* in the project. A
/// share counts as nothing while the person has no role there, and is kept.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{project}/shares/{person}",
    operation_id = "setShare",
    tag = "work-times",
    params(
        ("project" = String, Path, description = "The ID of the project."),
        ("person" = String, Path, description = "The ID of the person.")
    ),
    request_body = ShareBody,
    responses(
        (status = 200, description = "The share as it is now.", body = ShareBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn set_share(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((project, person)): ApiPath<(String, String)>,
    body: Result<ApiJson<ShareBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = path_id::<UserId>(&person).and_then(|person| {
        let ApiJson(body) = body?;
        Ok((person, body.percent))
    });
    let (person, percent) = auth.after_who_asks(&info, &session_token, read).await?;
    let changed = auth
        .api()?
        .set_share(
            info,
            SetShare {
                session_token,
                project,
                person,
                percent,
            },
        )
        .await?;
    answer_authorized(&changed.map(|changed| ShareBody::from(&changed)))
}

/// The routes of the shares.
pub(super) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_project_shares))
        .routes(routes!(set_share))
}
