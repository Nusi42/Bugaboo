//! Setting up AI (AI-01, AI-02, PRIV-08, COMP-08, SEC-16, ADR-0013): the settings, the key, the
//! agreement, *What goes where* and the try of the provider below `/api/v1/ai`, and the switch of a
//! project below `/api/v1/projects/{project}/ai`.
//!
//! AI is off until an administrator switches it on. The key is written and never sent back: the
//! answers say only that one is stored. The organization's settings are changed by whoever holds
//! *Change organization settings*, a project's switch by whoever holds *Change project settings*
//! there. Like the other settings of the organization and of projects, they have no version: the
//! last change wins (ADR-0018).

use axum::extract::State;
use axum::response::Response;
use bb_app::ai::{
    AgreementView, AiSetup, AiSetupQuery, ChangeAiSetup, ChangeProjectAi, ConfirmAiAgreement,
    ProjectAi, ProjectAiQuery, SetAiKey, TryAiProvider, WhatGoesWhere, WhatGoesWhereQuery,
};
use bb_core::ai::{AgreementStatus, Destination, ProviderKind};
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    TooSoonAnswer, answer_authorized, done,
};
use crate::errors::{ApiError, ApiJson, ApiPath};

/// Who confirmed the agreement, when, and for which provider.
#[derive(Serialize, ToSchema)]
pub(crate) struct AgreementBody {
    /// Where the data goes, which the agreement was confirmed for.
    destination: Destination,
    /// The administrator, as the person who asks may see them.
    confirmed_by: PersonRef,
    /// When it was confirmed.
    confirmed_at: String,
}

impl From<&AgreementView> for AgreementBody {
    fn from(agreement: &AgreementView) -> Self {
        Self {
            destination: agreement.destination,
            confirmed_by: agreement.confirmed_by.clone(),
            confirmed_at: agreement.confirmed_at.to_string(),
        }
    }
}

/// The setup of AI of the organization. The key is never in it.
#[derive(Serialize, ToSchema)]
pub(crate) struct AiSetupBody {
    /// AI is on for the organization.
    enabled: bool,
    /// The kind of provider.
    provider: ProviderKind,
    /// The model, as the provider names it.
    model: String,
    /// The address of an OpenAI-compatible model.
    address: String,
    /// Whether names of people go to an outside provider (otherwise the role of each person).
    send_names: bool,
    /// A key is stored. The key itself is never sent.
    key_set: bool,
    /// Where the data goes; missing while the address is not a good one.
    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<Destination>,
    /// The agreement that was confirmed, if there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    agreement: Option<AgreementBody>,
    /// Where the agreement for the provider stands.
    agreement_status: AgreementStatus,
}

impl Masked for AiSetupBody {}

impl From<&AiSetup> for AiSetupBody {
    fn from(setup: &AiSetup) -> Self {
        Self {
            enabled: setup.enabled,
            provider: setup.provider,
            model: setup.model.clone(),
            address: setup.address.clone(),
            send_names: setup.send_names,
            key_set: setup.key_set,
            destination: setup.destination,
            agreement: setup.agreement.as_ref().map(AgreementBody::from),
            agreement_status: setup.agreement_status,
        }
    }
}

/// A change of the settings of AI. A part that is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeAiBody {
    /// Switch AI on or off for the organization. Switching on an outside provider needs the
    /// agreement first.
    #[serde(default)]
    enabled: Option<bool>,
    /// The kind of provider.
    #[serde(default)]
    provider: Option<ProviderKind>,
    /// The model.
    #[serde(default)]
    model: Option<String>,
    /// The address of an OpenAI-compatible model; empty takes it away.
    #[serde(default)]
    address: Option<String>,
    /// Whether names of people go to an outside provider.
    #[serde(default)]
    send_names: Option<bool>,
    /// The password of the person who asks. It is needed when the provider or the address changes.
    #[serde(default)]
    password: Option<String>,
}

/// The key of the provider.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AiKeyBody {
    /// The key. Leave it out, or leave it empty, to remove it (a model that asks for none). It is
    /// stored encrypted and never sent back.
    #[serde(default)]
    key: Option<String>,
    /// The password of the person who asks.
    password: String,
}

/// The confirmation of the agreement.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AgreementConfirmBody {
    /// Where the data goes, as the page showed it. A request for another destination than the
    /// settings lead to now is refused, so a page that is out of date confirms nothing.
    destination: Destination,
}

/// A project that has switched AI off for itself.
#[derive(Serialize, ToSchema)]
pub(crate) struct SwitchedOffBody {
    /// The ID of the project.
    id: String,
    /// What people call it.
    name: String,
}

/// What goes where.
#[derive(Serialize, ToSchema)]
pub(crate) struct WhatGoesWhereBody {
    /// Where the data goes. The name of the provider, its country and the page where it says what
    /// it does with data are in the message catalog by this word.
    destination: Destination,
    /// The model.
    model: String,
    /// AI is on for the organization.
    enabled: bool,
    /// The projects that switched AI off for themselves. AI works in every other project.
    switched_off: Vec<SwitchedOffBody>,
    /// Whether people go to the provider by name; otherwise as their role.
    names_sent: bool,
    /// The agreement that was confirmed, if there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    agreement: Option<AgreementBody>,
    /// Where the agreement for the provider stands.
    agreement_status: AgreementStatus,
}

impl Masked for WhatGoesWhereBody {}

impl From<&WhatGoesWhere> for WhatGoesWhereBody {
    fn from(view: &WhatGoesWhere) -> Self {
        Self {
            destination: view.destination,
            model: view.model.clone(),
            enabled: view.enabled,
            switched_off: view
                .switched_off
                .iter()
                .map(|project| SwitchedOffBody {
                    id: project.id.to_string(),
                    name: project.name.clone(),
                })
                .collect(),
            names_sent: view.names_sent,
            agreement: view.agreement.as_ref().map(AgreementBody::from),
            agreement_status: view.agreement_status,
        }
    }
}

/// The switch of one project.
#[derive(Serialize, ToSchema)]
pub(crate) struct ProjectAiBody {
    /// The project's own switch; missing while it follows the project above or the organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    own: Option<bool>,
    /// The organization has AI on.
    organization_on: bool,
    /// AI works in this project: the organization and the project both have it on.
    works_here: bool,
}

impl Masked for ProjectAiBody {}

impl From<&ProjectAi> for ProjectAiBody {
    fn from(view: &ProjectAi) -> Self {
        Self {
            own: view.own,
            organization_on: view.organization_on,
            works_here: view.works_here,
        }
    }
}

/// The switch of a project.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ProjectAiChangeBody {
    /// The project's own switch. Leave it out (or send null) to let the project follow the one
    /// above again.
    #[serde(default)]
    enabled: Option<bool>,
}

/// The path of the switch of a project.
#[derive(Deserialize)]
pub(crate) struct ProjectPath {
    project: String,
}

/// The setup of AI of the organization: the settings and whether a key is stored, never the key.
#[utoipa::path(
    get,
    path = "/api/v1/ai",
    operation_id = "getAiSetup",
    tag = "ai",
    responses((status = 200, description = "The setup of AI.", body = AiSetupBody), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn get_ai_setup(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let setup = auth
        .api()?
        .ai_setup(info, AiSetupQuery { session_token })
        .await?;
    answer_authorized(&setup.map(|setup| AiSetupBody::from(&setup)))
}

/// Changes the settings of AI of the organization. AI is off until it is switched on here. An
/// outside provider needs the agreement first, and changing the provider or the address needs the
/// password again.
#[utoipa::path(
    put,
    path = "/api/v1/ai",
    operation_id = "changeAiSetup",
    tag = "ai",
    request_body = ChangeAiBody,
    responses(
        (status = 200, description = "The setup of AI after the change.", body = AiSetupBody),
        (status = 409, description = "An outside provider has no confirmed agreement yet (ai.agreement-needed).", body = bb_core::Problem, content_type = "application/problem+json"),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn change_ai_setup(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<ChangeAiBody>,
) -> Result<Response, ApiError> {
    let setup = auth
        .api()?
        .change_ai_setup(
            info,
            ChangeAiSetup {
                session_token,
                enabled: body.enabled,
                provider: body.provider,
                model: body.model,
                address: body.address,
                send_names: body.send_names,
                password: body.password,
            },
        )
        .await?;
    answer_authorized(&setup.map(|setup| AiSetupBody::from(&setup)))
}

/// Enters, changes or removes the key of the provider. It is stored encrypted like other secrets and
/// never sent back. Needs the password again.
#[utoipa::path(
    put,
    path = "/api/v1/ai/key",
    operation_id = "setAiKey",
    tag = "ai",
    request_body = AiKeyBody,
    responses((status = 204, description = "The key is stored."), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer)
)]
async fn set_ai_key(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<AiKeyBody>,
) -> Result<Response, ApiError> {
    auth.api()?
        .set_ai_key(
            info,
            SetAiKey {
                session_token,
                key: body.key,
                password: body.password,
            },
        )
        .await?;
    Ok(done())
}

/// Confirms that a data processing agreement with the provider is in place. It is kept with who
/// confirmed it and when; a change of the provider asks again.
#[utoipa::path(
    post,
    path = "/api/v1/ai/agreement",
    operation_id = "confirmAiAgreement",
    tag = "ai",
    request_body = AgreementConfirmBody,
    responses((status = 200, description = "The setup of AI after the confirmation.", body = AiSetupBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer)
)]
async fn confirm_ai_agreement(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<AgreementConfirmBody>,
) -> Result<Response, ApiError> {
    let setup = auth
        .api()?
        .confirm_ai_agreement(
            info,
            ConfirmAiAgreement {
                session_token,
                destination: body.destination,
            },
        )
        .await?;
    answer_authorized(&setup.map(|setup| AiSetupBody::from(&setup)))
}

/// What goes where: the provider, its country, the model and which data may go.
#[utoipa::path(
    get,
    path = "/api/v1/ai/what-goes-where",
    operation_id = "getWhatGoesWhere",
    tag = "ai",
    responses((status = 200, description = "What goes where.", body = WhatGoesWhereBody), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn get_what_goes_where(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let view = auth
        .api()?
        .what_goes_where(info, WhatGoesWhereQuery { session_token })
        .await?;
    answer_authorized(&view.map(|view| WhatGoesWhereBody::from(&view)))
}

/// Tries the provider with one short call that holds no data of the organization. It works while AI
/// is still off, but never before the agreement of an outside provider is confirmed.
#[utoipa::path(
    post,
    path = "/api/v1/ai/try",
    operation_id = "tryAiProvider",
    tag = "ai",
    responses(
        (status = 204, description = "The provider answered."),
        (status = 409, description = "An outside provider has no confirmed agreement yet (ai.agreement-needed).", body = bb_core::Problem, content_type = "application/problem+json"),
        (status = 502, description = "The AI provider did not answer (ai.unavailable).", body = bb_core::Problem, content_type = "application/problem+json"),
        NotSignedInAnswer,
        ForbiddenAnswer,
        TooSoonAnswer
    )
)]
async fn try_ai_provider(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    auth.api()?
        .try_ai_provider(info, TryAiProvider { session_token })
        .await?;
    Ok(done())
}

/// The switch of AI of a project. Needs the permission *Change project settings* in the project.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{project}/ai",
    operation_id = "getProjectAi",
    tag = "ai",
    params(("project" = String, Path, description = "The ID or the short name of the project.")),
    responses((status = 200, description = "The switch of the project.", body = ProjectAiBody), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn get_project_ai(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<ProjectPath>,
) -> Result<Response, ApiError> {
    let view = auth
        .api()?
        .project_ai(
            info,
            ProjectAiQuery {
                session_token,
                project: path.project,
            },
        )
        .await?;
    answer_authorized(&view.map(|view| ProjectAiBody::from(&view)))
}

/// Switches AI on or off for a project, or lets it follow the project above again. AI works in a
/// project only when the organization and the project both have it on. Needs the permission
/// *Change project settings* in the project.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{project}/ai",
    operation_id = "changeProjectAi",
    tag = "ai",
    params(("project" = String, Path, description = "The ID or the short name of the project.")),
    request_body = ProjectAiChangeBody,
    responses((status = 200, description = "The switch of the project after the change.", body = ProjectAiBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn change_project_ai(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<ProjectPath>,
    ApiJson(body): ApiJson<ProjectAiChangeBody>,
) -> Result<Response, ApiError> {
    let view = auth
        .api()?
        .change_project_ai(
            info,
            ChangeProjectAi {
                session_token,
                project: path.project,
                enabled: body.enabled,
            },
        )
        .await?;
    answer_authorized(&view.map(|view| ProjectAiBody::from(&view)))
}

/// The routes of the setup of AI.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_ai_setup, change_ai_setup))
        .routes(routes!(set_ai_key))
        .routes(routes!(confirm_ai_agreement))
        .routes(routes!(get_what_goes_where))
        .routes(routes!(try_ai_provider))
        .routes(routes!(get_project_ai, change_project_ai))
}
