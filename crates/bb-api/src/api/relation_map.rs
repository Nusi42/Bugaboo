//! The routes of the relation map (ISS-16, UX design §4.15): the map of an issue below
//! `/api/v1/issues/{issue}/map` and the map of a search at `/api/v1/map`.
//!
//! The map holds only issues the person may see, each with the fields they may see in its project,
//! and the relations between them; it is walked only through issues they see, so no line, count or
//! gap tells of a hidden one. A map is read like a search and counts against the limit of searches.

use std::sync::Arc;

use axum::Extension;
use axum::extract::{FromRef, State};
use axum::response::Response;
use bb_app::relations::{
    IssueMapQuery, KindsShown, MapIssue, MapLine, RelationMap, SearchMapQuery,
};
use bb_core::id::RelationKindId;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session, answer_authorized,
};
use super::relations::IssueRef;
use super::search::counted;
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::limits::Limits;

/// What separates the IDs of the kinds of relation in the parameter `kinds`.
const KINDS_SEPARATOR: char = ',';

/// What the map of an issue walks.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct IssueMapParams {
    /// How many steps away the map walks, from 1 to the most the search language walks; left out
    /// for *Everything*, which is that most.
    steps: Option<u32>,
    /// The IDs of the kinds of relation shown, separated by commas; left out for every kind, and
    /// empty for none.
    kinds: Option<String>,
    /// Whether sub-issues and their parents are shown (*part of*); shown when left out.
    part_of: Option<bool>,
}

/// The search a map is drawn of.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SearchMapBody {
    /// The search text; left out or empty for every issue the person sees.
    #[serde(default)]
    search_text: String,
    /// The IDs of the kinds of relation shown; left out for every kind.
    kinds: Option<Vec<String>>,
    /// Whether sub-issues and their parents are shown (*part of*); shown when left out.
    part_of: Option<bool>,
}

/// An issue on the map.
#[derive(Serialize, ToSchema)]
pub(crate) struct MapIssueBody {
    /// How many steps away from the start it is; left out on the map of a search.
    #[serde(skip_serializing_if = "Option::is_none")]
    steps: Option<u32>,
    /// The issue, with the fields the person may see in its project.
    issue: IssueRef,
}

impl From<&MapIssue> for MapIssueBody {
    fn from(each: &MapIssue) -> Self {
        Self {
            steps: each.steps,
            issue: IssueRef::from(&each.issue),
        }
    }
}

/// A line between two issues on the map.
#[derive(Serialize, ToSchema)]
pub(crate) struct MapLineBody {
    /// The internal ID of the issue that reads `forward`: the one that blocks, or the sub-issue.
    from: String,
    /// The internal ID of the issue that reads `backward`: the one that waits, or the parent.
    to: String,
    /// The ID of the kind of relation; left out for *part of*.
    #[serde(skip_serializing_if = "Option::is_none")]
    relation_kind: Option<String>,
    /// What `from` reads.
    forward: String,
    /// What `to` reads.
    backward: String,
}

impl From<&MapLine> for MapLineBody {
    fn from(line: &MapLine) -> Self {
        Self {
            from: line.from.to_string(),
            to: line.to.to_string(),
            relation_kind: line.kind.map(|kind| kind.to_string()),
            forward: line.forward.clone(),
            backward: line.backward.clone(),
        }
    }
}

/// The relation map as the person may see it.
#[derive(Serialize, ToSchema)]
pub(crate) struct RelationMapBody {
    /// The internal ID of the issue the map starts from; left out for the map of a search.
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<String>,
    /// The issues: by steps away, then by the kind of the relation that reached each, then by
    /// Issue ID; on the map of a search, in the order of the search.
    issues: Vec<MapIssueBody>,
    /// The lines between them, of the kinds shown.
    lines: Vec<MapLineBody>,
    /// The loops of dependencies among them, each as the internal IDs of the issues on it in order.
    loops: Vec<Vec<String>>,
    /// Whether more issues are connected, or found, than the map holds.
    more: bool,
}

// Each issue was masked by its own project in the use case; the rest are IDs and words of kinds.
impl Masked for RelationMapBody {}

impl From<RelationMap> for RelationMapBody {
    fn from(map: RelationMap) -> Self {
        Self {
            start: map.start.map(|start| start.to_string()),
            issues: map.issues.iter().map(MapIssueBody::from).collect(),
            lines: map.lines.iter().map(MapLineBody::from).collect(),
            loops: map
                .loops
                .iter()
                .map(|on_it| on_it.iter().map(ToString::to_string).collect())
                .collect(),
            more: map.more,
        }
    }
}

/// The kinds of relation named by their IDs; an ID that is none is no request.
fn kinds_of<'a>(
    ids: Option<impl Iterator<Item = &'a str>>,
) -> Result<Option<Vec<RelationKindId>>, ApiError> {
    ids.map(|ids| {
        ids.filter(|id| !id.trim().is_empty())
            .map(|id| {
                id.trim()
                    .parse()
                    .map_err(|_| ApiError::new(bb_core::ErrorCode::InvalidRequest))
            })
            .collect()
    })
    .transpose()
}

/// The map of an issue: every issue connected to it through the kinds shown, up to the steps
/// chosen and at most 200 issues, walked only through issues the person sees. Loops of
/// dependencies among them are named.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/map",
    operation_id = "getIssueMap",
    tag = "relations",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue."), IssueMapParams),
    responses(
        (status = 200, description = "The relation map of the issue.", body = RelationMapBody),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn get_issue_map(
    State(auth): State<AuthHandle>,
    Extension(limits): Extension<Arc<Limits>>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiQuery(params): ApiQuery<IssueMapParams>,
) -> Result<Response, ApiError> {
    counted(&auth, &limits, &info, &session_token).await?;
    let kinds = kinds_of(
        params
            .kinds
            .as_deref()
            .map(|text| text.split(KINDS_SEPARATOR)),
    )?;
    let map = auth
        .api()?
        .issue_map(
            info,
            IssueMapQuery {
                session_token,
                issue,
                steps: params.steps,
                shown: KindsShown {
                    kinds,
                    part_of: params.part_of.unwrap_or(true),
                },
            },
        )
        .await?;
    answer_authorized(&map.map(RelationMapBody::from))
}

/// The map of a search: its results (at most 200, in its order) and the relations between them,
/// with no walk beyond them. It reads and changes nothing.
#[utoipa::path(
    post,
    path = "/api/v1/map",
    operation_id = "getSearchMap",
    tag = "relations",
    request_body = SearchMapBody,
    responses(
        (status = 200, description = "The relation map of the search.", body = RelationMapBody),
        InvalidAnswer,
        NotSignedInAnswer
    )
)]
async fn get_search_map(
    State(auth): State<AuthHandle>,
    Extension(limits): Extension<Arc<Limits>>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<SearchMapBody>,
) -> Result<Response, ApiError> {
    counted(&auth, &limits, &info, &session_token).await?;
    let kinds = kinds_of(
        body.kinds
            .as_ref()
            .map(|ids| ids.iter().map(String::as_str)),
    )?;
    let map = auth
        .api()?
        .search_map(
            info,
            SearchMapQuery {
                session_token,
                text: body.search_text,
                shown: KindsShown {
                    kinds,
                    part_of: body.part_of.unwrap_or(true),
                },
            },
        )
        .await?;
    answer_authorized(&map.map(RelationMapBody::from))
}

pub(crate) fn routes<S>(limits: &Arc<Limits>) -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_issue_map))
        .routes(routes!(get_search_map))
        .layer(Extension(Arc::clone(limits)))
}
