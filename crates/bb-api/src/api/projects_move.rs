//! Moving a project below `/api/v1/projects/{project}` (PRJ-03, *Show, then do*): the preview of
//! what a move would do, and the move itself, which asks for the version of the preview.
//!
//! The preview is a `POST`, not a `GET`: it is a question that is answered afresh every time, its
//! answer is never kept, and where the project would go travels in the body. Both routes need the
//! permission *Move sub-projects into or out of the project* in the old and in the new parent
//! (the top level counts as the whole organization).

use axum::extract::State;
use axum::response::Response;
use bb_app::projects::{
    FieldLevelChange, LiftView, MovePreview, MovePreviewQuery, MoveProject, PeopleLevelChange,
    PersonChange, ProjectChange, Standing,
};
use bb_policy::{Masked, Permission, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized,
};
use super::projects::{ProjectBody, ProjectRefBody, project_id};
use crate::errors::{ApiError, ApiJson, ApiPath};

/// A word of the vocabulary with the message that names it: a field, a kind of people information
/// or a level.
#[derive(Serialize, ToSchema)]
pub(crate) struct WordBody {
    /// The word, as it stands in the API (`cost-center`, `can-see`).
    code: String,
    /// The key of the message that names it.
    message_key: String,
}

/// What a person has in one project.
#[derive(Serialize, ToSchema)]
pub(crate) struct StandingBody {
    /// The person sees the project.
    sees: bool,
    /// A *No access* holds the person back there.
    held_back: bool,
    /// The names of their roles there, in the order of roles.
    roles: Vec<String>,
}

impl From<&Standing> for StandingBody {
    fn from(standing: &Standing) -> Self {
        Self {
            sees: standing.sees,
            held_back: standing.held_back,
            roles: standing.roles.clone(),
        }
    }
}

/// A level of a field that the move changes for the person.
#[derive(Serialize, ToSchema)]
pub(crate) struct FieldLevelChangeBody {
    /// The field.
    field: WordBody,
    /// The level now.
    before: WordBody,
    /// The level after the move.
    after: WordBody,
}

impl From<&FieldLevelChange> for FieldLevelChangeBody {
    fn from(change: &FieldLevelChange) -> Self {
        Self {
            field: WordBody {
                code: change.field.code().to_owned(),
                message_key: change.field.name().as_str().to_owned(),
            },
            before: WordBody {
                code: change.before.code().to_owned(),
                message_key: change.before.name().as_str().to_owned(),
            },
            after: WordBody {
                code: change.after.code().to_owned(),
                message_key: change.after.name().as_str().to_owned(),
            },
        }
    }
}

/// A level of a kind of people information that the move changes for the person.
#[derive(Serialize, ToSchema)]
pub(crate) struct PeopleLevelChangeBody {
    /// The kind of people information.
    kind: WordBody,
    /// The level now.
    before: WordBody,
    /// The level after the move.
    after: WordBody,
}

impl From<&PeopleLevelChange> for PeopleLevelChangeBody {
    fn from(change: &PeopleLevelChange) -> Self {
        Self {
            kind: WordBody {
                code: change.kind.code().to_owned(),
                message_key: change.kind.name().as_str().to_owned(),
            },
            before: WordBody {
                code: change.before.code().to_owned(),
                message_key: change.before.name().as_str().to_owned(),
            },
            after: WordBody {
                code: change.after.code().to_owned(),
                message_key: change.after.name().as_str().to_owned(),
            },
        }
    }
}

/// What a move changes for one person in one project that moves.
#[derive(Serialize, ToSchema)]
pub(crate) struct ProjectChangeBody {
    /// The project.
    project: ProjectRefBody,
    /// What the person has there now.
    now: StandingBody,
    /// What they would have after the move.
    after: StandingBody,
    /// What they could not do and could after the move.
    gains: Vec<Permission>,
    /// What they could do and could not after the move.
    loses: Vec<Permission>,
    /// The fields whose level changes.
    fields: Vec<FieldLevelChangeBody>,
    /// The kinds of people information whose level changes.
    people: Vec<PeopleLevelChangeBody>,
}

impl From<&ProjectChange> for ProjectChangeBody {
    fn from(change: &ProjectChange) -> Self {
        Self {
            project: ProjectRefBody::from(&change.project),
            now: StandingBody::from(&change.now),
            after: StandingBody::from(&change.after),
            gains: change.gains.clone(),
            loses: change.loses.clone(),
            fields: change.fields.iter().map(Into::into).collect(),
            people: change.people.iter().map(Into::into).collect(),
        }
    }
}

/// What a move changes for one person.
#[derive(Serialize, ToSchema)]
pub(crate) struct PersonChangeBody {
    /// The person, as the person who asks may see them.
    person: PersonRef,
    /// The projects where something changes for them.
    projects: Vec<ProjectChangeBody>,
}

impl From<&PersonChange> for PersonChangeBody {
    fn from(change: &PersonChange) -> Self {
        Self {
            person: change.person.clone(),
            projects: change.projects.iter().map(Into::into).collect(),
        }
    }
}

/// Somebody who could take away a *No access* that applies to them after the move.
#[derive(Serialize, ToSchema)]
pub(crate) struct LiftBody {
    /// The person, as the person who asks may see them.
    person: PersonRef,
    /// The project the *No access* is on.
    project: ProjectRefBody,
}

impl From<&LiftView> for LiftBody {
    fn from(lift: &LiftView) -> Self {
        Self {
            person: lift.person.clone(),
            project: ProjectRefBody::from(&lift.project),
        }
    }
}

/// What a move would do, before anything happens.
#[derive(Serialize, ToSchema)]
pub(crate) struct MovePreviewBody {
    /// The project that would move.
    project: ProjectRefBody,
    /// Where it would go, when the person may see that project; `null` for the top level.
    to: Option<ProjectRefBody>,
    /// The version of this preview. The move asks for it, and works only while nothing it was made
    /// from has changed.
    preview: String,
    /// The move would be refused: somebody could take away a *No access* that applies to them.
    blocked: bool,
    /// Who gains or loses access or changes role, in the projects the person sees.
    people: Vec<PersonChangeBody>,
    /// Who could take away a *No access* after the move and could not before.
    lifts: Vec<LiftBody>,
    /// The list was cut: it names the first people it found.
    truncated: bool,
}

impl Masked for MovePreviewBody {}

impl From<MovePreview> for MovePreviewBody {
    fn from(preview: MovePreview) -> Self {
        Self {
            project: ProjectRefBody::from(&preview.project),
            to: preview.to.as_ref().map(ProjectRefBody::from),
            preview: preview.preview,
            blocked: preview.blocked,
            people: preview.people.iter().map(Into::into).collect(),
            lifts: preview.lifts.iter().map(Into::into).collect(),
            truncated: preview.truncated,
        }
    }
}

/// Where a project would go.
#[derive(Deserialize, ToSchema)]
pub(crate) struct WhereToBody {
    /// The ID of the project it would go into; `null` for the top level.
    to: Option<String>,
}

/// Where a project goes, and the version of the preview that was looked at.
#[derive(Deserialize, ToSchema)]
pub(crate) struct MoveBody {
    /// The ID of the project it goes into; `null` for the top level.
    to: Option<String>,
    /// The `preview` of the answer to the preview that was looked at.
    preview: String,
}

/// What a move of a project would do: who gains or loses access or changes role, which fields and
/// kinds of people information change for whom, and whether the move would be refused. Needs the
/// permission *Move sub-projects into or out of the project*.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{project}/move-preview",
    operation_id = "previewProjectMove",
    tag = "projects",
    params(("project" = String, Path, description = "The ID of the project that would move.")),
    request_body = WhereToBody,
    responses(
        (status = 200, description = "What the move would do.", body = MovePreviewBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn preview_project_move(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
    ApiJson(body): ApiJson<WhereToBody>,
) -> Result<Response, ApiError> {
    let project = project_id(&project)?;
    let to = body.to.as_deref().map(project_id).transpose()?;
    let preview = auth
        .api()?
        .preview_move(
            info,
            MovePreviewQuery {
                session_token,
                project,
                to,
            },
        )
        .await?;
    answer_authorized(&preview.map(MovePreviewBody::from))
}

/// Moves a project, with everything in it, into another project or to the top level. It works
/// only with the `preview` of a preview that is still true: when the projects, who has which
/// role or the levels of the roles changed since, it answers `project.preview_changed`. A move
/// after which somebody could take away a *No access* that applies to them is refused. Needs the
/// permission *Move sub-projects into or out of the project* in the old and the new parent.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{project}/move",
    operation_id = "moveProject",
    tag = "projects",
    params(("project" = String, Path, description = "The ID of the project that moves.")),
    request_body = MoveBody,
    responses(
        (status = 200, description = "The project after the move.", body = ProjectBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn move_project(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
    ApiJson(body): ApiJson<MoveBody>,
) -> Result<Response, ApiError> {
    let project = project_id(&project)?;
    let to = body.to.as_deref().map(project_id).transpose()?;
    let moved = auth
        .api()?
        .move_project(
            info,
            MoveProject {
                session_token,
                project,
                to,
                preview: body.preview,
            },
        )
        .await?;
    answer_authorized(&moved.map(ProjectBody::from))
}

/// The routes that move a project.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(preview_project_move))
        .routes(routes!(move_project))
}
