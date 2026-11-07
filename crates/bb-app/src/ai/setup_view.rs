//! Reading the setup of AI (AI-01, AI-02, PRIV-08, COMP-08): the settings as they are, what goes
//! where, and the switch of one project. Reads only; the changes are in `setup`.

use bb_core::ai::{AgreementStatus, Destination, ProviderKind, SWITCHED_OFF_MAX};
use bb_core::id::ProjectId;
use bb_core::people_kinds::PeopleKind;
use bb_core::settings::{AI_ENABLED, SettingValue};
use bb_policy::{Authorized, Evaluation, Masked, PeopleContext, Permission, PersonRef, Place};
use jiff::Timestamp;

use super::call::AiState;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// Ask for the setup of AI of the organization.
pub struct AiSetupQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// Who confirmed the agreement, when, and for which destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgreementView {
    /// What the administrator confirmed it for.
    pub destination: Destination,
    /// The administrator, as the person who asks may see them.
    pub confirmed_by: PersonRef,
    /// When.
    pub confirmed_at: Timestamp,
}

/// The setup of AI of the organization. The key itself is never in it (SEC-16).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiSetup {
    /// Whether AI is on for the organization.
    pub enabled: bool,
    /// The kind of provider.
    pub provider: ProviderKind,
    /// The model.
    pub model: String,
    /// The address of an OpenAI-compatible model, as entered.
    pub address: String,
    /// Whether names of people go to an outside provider.
    pub send_names: bool,
    /// A key is stored.
    pub key_set: bool,
    /// Where the data goes, when the address is a good one.
    pub destination: Option<Destination>,
    /// The agreement that was confirmed, if there is one.
    pub agreement: Option<AgreementView>,
    /// Where the agreement for the provider stands.
    pub agreement_status: AgreementStatus,
}

impl Masked for AiSetup {}

/// Ask what goes where.
pub struct WhatGoesWhereQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// A project that has switched AI off for itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchedOff {
    /// The project.
    pub id: ProjectId,
    /// What people call it.
    pub name: String,
}

/// What an administrator should know before switching AI on, and what is true while it is on
/// (COMP-08): the provider, its country, the model, and which data may go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhatGoesWhere {
    /// Where the data goes.
    pub destination: Destination,
    /// The model.
    pub model: String,
    /// AI is on for the organization.
    pub enabled: bool,
    /// The projects that switched AI off for themselves; AI works in every other project (and in
    /// the sub-projects of those, unless they switch it on again).
    pub switched_off: Vec<SwitchedOff>,
    /// More projects switched it off than the list names.
    pub truncated: bool,
    /// Whether people go to the provider by name; if not, as their role.
    pub names_sent: bool,
    /// The agreement that was confirmed, if there is one.
    pub agreement: Option<AgreementView>,
    /// Where the agreement for the provider stands.
    pub agreement_status: AgreementStatus,
}

impl Masked for WhatGoesWhere {}

/// Ask for the switch of a project.
pub struct ProjectAiQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project: its ID or its short name.
    pub project: String,
}

/// The switch of one project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectAi {
    /// The project's own switch; none while it follows the project above or the organization.
    pub own: Option<bool>,
    /// The organization has AI on.
    pub organization_on: bool,
    /// AI counts in this project: the organization and the project both have it on.
    pub works_here: bool,
}

impl Masked for ProjectAi {}

impl<S: AuthStore> Ctx<'_, S> {
    async fn agreement_view(
        &self,
        viewer: &Evaluation,
        agreement: Option<bb_core::ai::Agreement>,
    ) -> Result<Option<AgreementView>, AppError> {
        let Some(agreement) = agreement else {
            return Ok(None);
        };
        let confirmed_by = self
            .person_ref(
                viewer,
                PeopleKind::LastChangedBy,
                PeopleContext::Management,
                agreement.confirmed_by,
            )
            .await?;
        Ok(Some(AgreementView {
            destination: agreement.destination,
            confirmed_by,
            confirmed_at: agreement.confirmed_at,
        }))
    }

    /// The setup as it is now, for the answer of every change.
    pub(crate) async fn ai_setup_now(&self, viewer: &Evaluation) -> Result<AiSetup, AppError> {
        let state = self.ai_state_at(None).await?;
        let key_set = self
            .store()
            .ai_key()
            .await
            .map_err(|error| internal::<S>(&error))?
            .is_some();
        let destination = state.destination().ok();
        let agreement = self.ai_agreement().await?;
        let agreement_status = AgreementStatus::of(
            destination.unwrap_or(Destination::OtherProvider),
            agreement.as_ref(),
        );
        Ok(AiSetup {
            enabled: state.organization_on,
            provider: state.kind,
            model: state.model.clone(),
            address: state.address.clone(),
            send_names: state.send_names,
            key_set,
            destination,
            agreement: self.agreement_view(viewer, agreement).await?,
            agreement_status,
        })
    }
}

/// The setup of AI of the organization.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*.
pub async fn ai_setup<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AiSetupQuery,
) -> Result<Authorized<AiSetup>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    Ok(access.authorize(ctx.ai_setup_now(&access).await?))
}

/// What goes where.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*.
pub async fn what_goes_where<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: WhatGoesWhereQuery,
) -> Result<Authorized<WhatGoesWhere>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let state: AiState = ctx.ai_state_at(None).await?;
    let destination = state.destination().unwrap_or(Destination::OtherProvider);
    let agreement = ctx.ai_agreement().await?;
    let mut switched_off = Vec::new();
    let mut truncated = false;
    for (id, value) in ctx
        .store()
        .project_values(AI_ENABLED.definition())
        .await
        .map_err(|error| internal::<S>(&error))?
    {
        if switched_off.len() >= SWITCHED_OFF_MAX {
            truncated = true;
            break;
        }
        if value == SettingValue::Bool(false)
            && let Some(project) = ctx
                .store()
                .project(id)
                .await
                .map_err(|error| internal::<S>(&error))?
        {
            switched_off.push(SwitchedOff {
                id,
                name: project.name,
            });
        }
    }
    Ok(access.authorize(WhatGoesWhere {
        destination,
        model: state.model,
        enabled: state.organization_on,
        switched_off,
        truncated,
        // A model on the organization's own network gets the names each person may see
        // (M7-Q5); an outside provider gets roles unless an administrator chose otherwise.
        names_sent: !destination.is_outside() || state.send_names,
        agreement_status: AgreementStatus::of(destination, agreement.as_ref()),
        agreement: ctx.agreement_view(&access, agreement).await?,
    }))
}

/// The switch of one project.
///
/// # Errors
/// `access.forbidden` without *Change project settings* in the project, `resource.not-found` for a
/// project the person may not know about.
pub async fn project_ai<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ProjectAiQuery,
) -> Result<Authorized<ProjectAi>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let project = ctx
        .find_project(&input.project)
        .await?
        .ok_or(bb_core::ErrorCode::NotFound)?
        .project
        .id;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeProjectSettings,
            Place::Project(project),
        )
        .await?;
    let layers = ctx
        .store()
        .setting_layers(Some(project), None)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let own = layers
        .projects
        .iter()
        .find(|(id, _)| *id == project)
        .and_then(|(_, values)| values.get(AI_ENABLED.definition().key()))
        .and_then(|value| match value {
            SettingValue::Bool(on) => Some(*on),
            SettingValue::Integer(_) | SettingValue::Text(_) => None,
        });
    let state = AiState::of(&layers);
    Ok(access.authorize(ProjectAi {
        own,
        organization_on: state.organization_on,
        works_here: state.counts(),
    }))
}
