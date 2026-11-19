//! *Switch off*, *Switch on* and *Remove a person* (PRIV-04, ADR-0044, decisions 5 and 6): the
//! routes below `/api/v1/people/{person}` that end a person's every way in at once, give back the
//! sign-in, show the plan of a removal and remove a person. *Switch off* and *Switch on* need
//! *Manage people*, the plan and the removal *Remove people*, all for the whole organization.

use axum::extract::State;
use axum::response::Response;
use bb_app::people::{SwitchOff, SwitchOn};
use bb_app::privacy::removal::{KeptByLaw, PlanKind, RemovalPlan, RemovalPlanQuery, RemovePerson};
use bb_core::ErrorCode;
use bb_core::id::UserId;
use bb_core::keeping::KeepKind;
use bb_core::settings::privacy::RemovedTexts;
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized, done,
};
use super::path_id;
use crate::errors::{ApiBytes, ApiError, ApiPath};

/// Switches a person off at once: their sessions end, their access keys and the grants of AI apps
/// are taken back, their agents stop and the invitations they made are taken back. Their
/// assignments stay. Needs *Manage people*.
#[utoipa::path(
    post,
    path = "/api/v1/people/{person}/switch-off",
    operation_id = "switchPersonOff",
    tag = "people",
    params(("person" = String, Path, description = "The ID of the person.")),
    responses(
        (status = 204, description = "The person is switched off."),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn switch_person_off(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(person): ApiPath<String>,
) -> Result<Response, ApiError> {
    let person = path_id::<UserId>(&person)?;
    auth.api()?
        .switch_off(
            info,
            SwitchOff {
                session_token,
                person,
            },
        )
        .await?;
    Ok(done())
}

/// Switches a switched-off person on again: they may sign in. What was taken back stays taken
/// back. Needs *Manage people*.
#[utoipa::path(
    post,
    path = "/api/v1/people/{person}/switch-on",
    operation_id = "switchPersonOn",
    tag = "people",
    params(("person" = String, Path, description = "The ID of the person.")),
    responses(
        (status = 204, description = "The person may sign in again."),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn switch_person_on(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(person): ApiPath<String>,
) -> Result<Response, ApiError> {
    let person = path_id::<UserId>(&person)?;
    auth.api()?
        .switch_on(
            info,
            SwitchOn {
                session_token,
                person,
            },
        )
        .await?;
    Ok(done())
}

/// One kind of record in a list of the plan.
#[derive(Serialize, ToSchema)]
pub(crate) struct PlanKindBody {
    /// The kind, in plain words.
    title: String,
    /// How many of its rows name the person.
    rows: u64,
}

impl From<&PlanKind> for PlanKindBody {
    fn from(kind: &PlanKind) -> Self {
        Self {
            title: kind.title.clone(),
            rows: kind.rows,
        }
    }
}

/// One kind the keep rules keep for its period.
#[derive(Serialize, ToSchema)]
pub(crate) struct KeptByLawBody {
    /// The row of the keep rules.
    kind: KeepKind,
    /// The kind, in plain words.
    title: String,
    /// How many of its rows name the person; left out for the backups, which hold everything.
    #[serde(skip_serializing_if = "Option::is_none")]
    rows: Option<u64>,
    /// The last day it is kept (`YYYY-MM-DD`); left out when no day can be named.
    #[serde(skip_serializing_if = "Option::is_none")]
    until: Option<String>,
    /// Why no day can be named, in words.
    #[serde(skip_serializing_if = "Option::is_none")]
    no_fixed_end: Option<String>,
}

impl From<&KeptByLaw> for KeptByLawBody {
    fn from(kept: &KeptByLaw) -> Self {
        Self {
            kind: kept.kind,
            title: kept.title.clone(),
            rows: kept.rows,
            until: kept.until.map(|day| day.to_string()),
            no_fixed_end: kept.no_fixed_end.clone(),
        }
    }
}

/// The plan of the removal of a person.
#[derive(Serialize, ToSchema)]
pub(crate) struct RemovalPlanBody {
    /// The person, as the viewer sees them.
    person: PersonRef,
    /// Deleted in the change that removes the person.
    deleted_now: Vec<PlanKindBody>,
    /// Kept as records of the organization, shown as *Former user*.
    kept_as_former_user: Vec<PlanKindBody>,
    /// What the window offers first for the person's comments.
    comments: RemovedTexts,
    /// Kept for their keep period, and until when.
    kept_by_law: Vec<KeptByLawBody>,
}

impl Masked for RemovalPlanBody {}

impl From<&RemovalPlan> for RemovalPlanBody {
    fn from(plan: &RemovalPlan) -> Self {
        Self {
            person: plan.person.clone(),
            deleted_now: plan.deleted_now.iter().map(PlanKindBody::from).collect(),
            kept_as_former_user: plan
                .kept_as_former_user
                .iter()
                .map(PlanKindBody::from)
                .collect(),
            comments: plan.comments,
            kept_by_law: plan.kept_by_law.iter().map(KeptByLawBody::from).collect(),
        }
    }
}

/// What *Remove a person* would delete now, what stays shown as *Former user*, and what the law
/// keeps until when. Changes nothing. Needs *Remove people*.
#[utoipa::path(
    get,
    path = "/api/v1/people/{person}/removal-plan",
    operation_id = "getRemovalPlan",
    tag = "people",
    params(("person" = String, Path, description = "The ID of the person.")),
    responses(
        (status = 200, description = "The plan.", body = RemovalPlanBody),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn get_removal_plan(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(person): ApiPath<String>,
) -> Result<Response, ApiError> {
    let person = path_id::<UserId>(&person)?;
    let plan = auth
        .api()?
        .removal_plan(
            info,
            RemovalPlanQuery {
                session_token,
                person,
            },
        )
        .await?;
    answer_authorized(&plan.map(|plan| RemovalPlanBody::from(&plan)))
}

/// What *Remove a person* does with the person's comments.
#[derive(Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RemoveBody {
    /// Keep the comments, shown as *Former user*, or delete them; left out, the choice the
    /// setting `privacy.removed-texts` offers.
    #[serde(default)]
    comments: Option<RemovedTexts>,
}

/// Removes a switched-off person for good: deletes the account and what only serves it, keeps
/// the records of the organization shown as *Former user*, and the work days by law. Cannot be
/// undone. Needs *Remove people*.
#[utoipa::path(
    post,
    path = "/api/v1/people/{person}/remove",
    operation_id = "removePerson",
    tag = "people",
    params(("person" = String, Path, description = "The ID of the person.")),
    request_body = RemoveBody,
    responses(
        (status = 204, description = "The person is removed."),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn remove_person(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(person): ApiPath<String>,
    body: Result<ApiBytes, ApiError>,
) -> Result<Response, ApiError> {
    // No body takes the setting's choice, as switching off needs none.
    let read = body.and_then(|ApiBytes(bytes)| {
        let body = if bytes.is_empty() {
            RemoveBody::default()
        } else {
            serde_json::from_slice::<RemoveBody>(&bytes)
                .map_err(|_| ApiError::new(ErrorCode::InvalidRequest))?
        };
        Ok((path_id::<UserId>(&person)?, body.comments))
    });
    let (person, comments) = auth.after_who_asks(&info, &session_token, read).await?;
    auth.api()?
        .remove_person(
            info,
            RemovePerson {
                session_token,
                person,
                comments,
            },
        )
        .await?;
    Ok(done())
}

pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(switch_person_off))
        .routes(routes!(switch_person_on))
        .routes(routes!(get_removal_plan))
        .routes(routes!(remove_person))
}
