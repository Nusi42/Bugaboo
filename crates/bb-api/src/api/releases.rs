//! The release routes below `/api/v1/projects/{project}/releases` (ISS-08, ADR-0049): the list of
//! the releases offered in a project for everybody who sees it, and making, changing, marking,
//! retiring, deleting and taking back a release for those who hold *Manage releases* in the
//! project it was made in. API v1 grows by these routes only (ARCH-API-FREEZE).

use axum::extract::State;
use axum::response::Response;
use bb_app::releases::{
    ChangeRelease, CreateRelease, DeleteRelease, ReleasePage, ReleaseRef, ReleaseView,
    ReleasesQuery,
};
use bb_core::given::SetTo;
use bb_core::id::ReleaseId;
use bb_core::issue::CalendarDay;
use bb_core::project_release::ReleaseState;
use bb_policy::{Authorized, Masked};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::cursor_error;
use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer, answer_authorized,
};
use super::projects::ProjectRef;
use super::{body_id, path_id};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The name of the list, so its cursors are its own.
const RELEASES_LIST: &str = "releases";

/// The fields of a release in a list: all of them by default, a release is small.
const RELEASE_FIELDS: [FieldSpec; 8] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("project"),
    FieldSpec::by_default("release_date"),
    FieldSpec::by_default("state"),
    FieldSpec::by_default("released_on"),
    FieldSpec::by_default("may_manage"),
    FieldSpec::by_default("version"),
];

/// A release: the fields that can be asked for.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct ReleaseItem {
    /// The ID of the release.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    id: Option<String>,
    /// What it is called.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    name: Option<String>,
    /// The project it was made in: offered there and in its sub-projects.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(nested)]
    project: Option<ProjectRef>,
    /// *Release date* (`2026-11-14`); left out while it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    release_date: Option<String>,
    /// *Planned*, *Released* or *Retired*.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    state: Option<ReleaseState>,
    /// *Released on* (`2026-11-14`): the day it was marked *Released*; left out while it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    released_on: Option<String>,
    /// Whether the person who asks may make, change, mark and delete releases of its project.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    may_manage: Option<bool>,
    /// Counts up with every change. A change is made on the version that was looked at.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    version: Option<i64>,
}

impl From<&ReleaseView> for ReleaseItem {
    fn from(release: &ReleaseView) -> Self {
        Self {
            id: Some(release.id.to_string()),
            name: Some(release.name.clone()),
            project: Some(ProjectRef::from(&release.project)),
            release_date: release.release_date.map(|day| day.to_string()),
            state: Some(release.state),
            released_on: release.released_on.map(|day| day.to_string()),
            may_manage: Some(release.may_manage),
            version: Some(release.version),
        }
    }
}

/// A release an issue names, as a small reference.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct ReleaseRefBody {
    /// The ID of the release.
    #[masked(open)]
    id: String,
    /// What it is called.
    #[masked(open)]
    name: String,
    /// *Planned*, *Released* or *Retired*.
    #[masked(open)]
    state: ReleaseState,
    /// *Release date* (`2026-11-14`); left out while it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    release_date: Option<String>,
    /// Whether it is offered where the issue is now: `false` for an issue that moved out of the
    /// tree of the release's project, which keeps it.
    #[masked(open)]
    offered: bool,
}

impl From<&ReleaseRef> for ReleaseRefBody {
    fn from(release: &ReleaseRef) -> Self {
        Self {
            id: release.id.to_string(),
            name: release.name.clone(),
            state: release.state,
            release_date: release.release_date.map(|day| day.to_string()),
            offered: release.offered,
        }
    }
}

#[cfg(test)]
impl ReleaseRefBody {
    /// A reference with nothing in it, for the tests of the field tables.
    pub(crate) fn blank() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            state: ReleaseState::Planned,
            release_date: None,
            offered: true,
        }
    }
}

/// The IDs of releases in a request body, each one an ID or the request is not valid.
pub(crate) fn releases_in(texts: Option<&[String]>) -> Result<Option<Vec<ReleaseId>>, ApiError> {
    texts.map(super::body_ids).transpose()
}

/// A release a request sets or takes away (`null`).
pub(crate) fn set_release(
    given: Option<SetTo<String>>,
) -> Result<Option<SetTo<ReleaseId>>, ApiError> {
    given
        .map(|given| match given {
            SetTo::Value(text) => body_id(&text).map(SetTo::Value),
            SetTo::Nothing => Ok(SetTo::Nothing),
        })
        .transpose()
}

/// A release to make.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewReleaseBody {
    /// What it is called, unique within its project.
    name: String,
    /// Its *Release date* (`2026-11-14`). None when left out.
    #[schema(value_type = Option<String>, format = Date)]
    release_date: Option<CalendarDay>,
}

/// What to change about a release: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChangeReleaseBody {
    /// The version the person looked at.
    version: i64,
    /// A new name.
    name: Option<String>,
    /// A new *Release date* (`2026-11-14`); `null` takes it away.
    #[serde(default, deserialize_with = "bb_core::given::deserialize")]
    #[schema(value_type = Option<String>, format = Date, nullable)]
    release_date: Option<SetTo<CalendarDay>>,
    /// A new state: *Released* keeps the day as *Released on*, *Retired* stops offering it for new
    /// choices.
    state: Option<ReleaseState>,
    /// Why a release goes back from *Released* to *Planned*: needed then, kept in the history.
    reason: Option<String>,
}

/// The project and the release a route names.
fn release_of(project: String, release: &str) -> Result<(String, ReleaseId), ApiError> {
    Ok((project, path_id(release)?))
}

/// The answer of a change of one release.
fn release_answer(release: Authorized<ReleaseView>) -> Result<Response, ApiError> {
    answer_authorized(&release.map(|view| ReleaseItem::from(&view)))
}

/// The releases offered in a project: those made in it and in the projects above it that the
/// person sees, retired ones too, by *Release date* (none last), then by name. Needs only seeing
/// the project.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{project}/releases",
    operation_id = "listReleases",
    tag = "releases",
    params(("project" = String, Path, description = "The ID or the short name of the project."), ListParams),
    responses((status = 200, description = "The releases.", body = Page<ReleaseItem>), InvalidAnswer, NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_releases(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(RELEASES_LIST);
    let page = auth
        .api()?
        .releases(
            info,
            ReleasesQuery {
                session_token,
                project,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    let list = page.map(|page: ReleasePage| Listed {
        items: page.items.iter().map(ReleaseItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&RELEASE_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(RELEASES_LIST, &list, &fields)?))
}

/// Makes a release in the project, in the state *Planned*. Needs *Manage releases* there.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{project}/releases",
    operation_id = "createRelease",
    tag = "releases",
    params(("project" = String, Path, description = "The ID or the short name of the project.")),
    request_body = NewReleaseBody,
    responses(
        (status = 200, description = "The release.", body = ReleaseItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn create_release(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
    ApiJson(body): ApiJson<NewReleaseBody>,
) -> Result<Response, ApiError> {
    let made = auth
        .api()?
        .create_release(
            info,
            CreateRelease {
                session_token,
                project,
                name: body.name,
                release_date: body.release_date,
            },
        )
        .await?;
    release_answer(made)
}

/// Renames a release, gives it another *Release date*, marks it *Released*, takes it back to
/// *Planned* (with a reason) or retires it. Needs *Manage releases* in its project.
#[utoipa::path(
    patch,
    path = "/api/v1/projects/{project}/releases/{release}",
    operation_id = "changeRelease",
    tag = "releases",
    params(
        ("project" = String, Path, description = "The ID or the short name of the project it was made in."),
        ("release" = String, Path, description = "The ID of the release.")
    ),
    request_body = ChangeReleaseBody,
    responses(
        (status = 200, description = "The release as it is now.", body = ReleaseItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_release(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((project, release)): ApiPath<(String, String)>,
    ApiJson(body): ApiJson<ChangeReleaseBody>,
) -> Result<Response, ApiError> {
    let (project, release) = release_of(project, &release)?;
    let changed = auth
        .api()?
        .change_release(
            info,
            ChangeRelease {
                session_token,
                project,
                release,
                version: body.version,
                name: body.name,
                release_date: body.release_date,
                state: body.state,
                reason: body.reason,
            },
        )
        .await?;
    release_answer(changed)
}

/// Puts a release that no issue names into the trash (*Undo* takes it back). A release that an
/// issue names is refused (`release.in-use`) and is retired instead; the refusal says nothing of
/// which issues or how many. Needs *Manage releases* in its project.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{project}/releases/{release}",
    operation_id = "deleteRelease",
    tag = "releases",
    params(
        ("project" = String, Path, description = "The ID or the short name of the project it was made in."),
        ("release" = String, Path, description = "The ID of the release.")
    ),
    responses(
        (status = 200, description = "The release, in the trash.", body = ReleaseItem),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn delete_release(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((project, release)): ApiPath<(String, String)>,
) -> Result<Response, ApiError> {
    let (project, release) = release_of(project, &release)?;
    let deleted = auth
        .api()?
        .delete_release(
            info,
            DeleteRelease {
                session_token,
                project,
                release,
            },
        )
        .await?;
    release_answer(deleted)
}

/// Takes a release out of the trash (*Undo*). Needs *Manage releases* in its project.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{project}/releases/{release}/restore",
    operation_id = "restoreRelease",
    tag = "releases",
    params(
        ("project" = String, Path, description = "The ID or the short name of the project it was made in."),
        ("release" = String, Path, description = "The ID of the release.")
    ),
    responses(
        (status = 200, description = "The release, out of the trash.", body = ReleaseItem),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn restore_release(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((project, release)): ApiPath<(String, String)>,
) -> Result<Response, ApiError> {
    let (project, release) = release_of(project, &release)?;
    let restored = auth
        .api()?
        .restore_release(
            info,
            DeleteRelease {
                session_token,
                project,
                release,
            },
        )
        .await?;
    release_answer(restored)
}

/// The routes of the releases.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_releases, create_release))
        .routes(routes!(change_release, delete_release))
        .routes(routes!(restore_release))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iss_08_the_field_table_names_exactly_the_fields_of_a_release() {
        let item = ReleaseItem {
            id: Some(String::new()),
            name: Some(String::new()),
            project: Some(ProjectRef {
                id: String::new(),
                short_name: String::new(),
                name: String::new(),
            }),
            release_date: Some(String::new()),
            state: Some(ReleaseState::Planned),
            released_on: Some(String::new()),
            may_manage: Some(false),
            version: Some(1),
        };
        crate::lean::assert_fields_are_the_keys_of(&RELEASE_FIELDS, &item);
    }
}
