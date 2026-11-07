//! Setting up AI (AI-01, AI-02, PRIV-08, COMP-08, ADR-0013): the settings, the key, the agreement
//! step, the switch of a project and the try of the provider.
//!
//! AI is off until an administrator switches it on. Switching on an outside provider needs the
//! confirmation that a data processing agreement is in place, kept with who and when; a model on
//! the organization's own network needs none. Changing the provider or the address, and entering
//! the key, ask for the password again, and a new provider does not get the key of the old one
//! (the lesson of the mail server, M4-F123). Every change is an event and a line of the security
//! log, and none of them tells the key.

use bb_core::ErrorCode;
use bb_core::ai::{
    Agreement, AgreementStatus, Destination, KEY_MAX, MODEL_MAX, ProviderKind, TRY_TEXT,
    destination_of,
};
use bb_core::event::{FieldChange, RecordKind};
use bb_core::settings::{
    AI_ADDRESS, AI_ENABLED, AI_MODEL, AI_PROVIDER, AI_SEND_NAMES, Scope, SettingDefinition,
    SettingValue,
};
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Permission, Place};

use super::call::AiState;
use super::setup_view::{AiSetup, ProjectAi};
use super::{AiPart, AiRequest, AiRole, AiTurn};
use crate::auth::{
    AppError, AuthStore, Ctx, Done, Stops, by, change_failed, changed, fixed_entity, internal,
};
use crate::settings::{field_of, json_of, record_of};

/// The record of the settings of AI in the events: one fixed record.
const AI_RECORD: &str = "ai";

/// Change the settings of AI of the organization. A part left out stays as it is.
pub struct ChangeAiSetup {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Switch AI on or off for the organization.
    pub enabled: Option<bool>,
    /// The kind of provider.
    pub provider: Option<ProviderKind>,
    /// The model.
    pub model: Option<String>,
    /// The address of an OpenAI-compatible model; empty takes it away.
    pub address: Option<String>,
    /// Whether names of people go to an outside provider.
    pub send_names: Option<bool>,
    /// The password of the person who asks: needed when the provider or the address changes.
    pub password: Option<String>,
}

/// Enter, change or remove the key of the provider.
pub struct SetAiKey {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The key; none (or an empty one) removes it, for a model that asks for none.
    pub key: Option<String>,
    /// The password of the person who asks.
    pub password: String,
}

/// Confirm that a data processing agreement with the provider is in place.
pub struct ConfirmAiAgreement {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The destination the person saw when they confirmed; a request for another one is refused,
    /// so a page that is out of date confirms nothing.
    pub destination: Destination,
}

/// Switch AI on or off for one project, or let it follow the one above again.
pub struct ChangeProjectAi {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project: its ID or its short name.
    pub project: String,
    /// The project's own switch; none goes back to following the project above.
    pub enabled: Option<bool>,
}

/// Try the provider with one short call.
pub struct TryAiProvider {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// A change of a setting: what it is now and what it becomes.
struct Step {
    definition: &'static SettingDefinition,
    before: SettingValue,
    after: SettingValue,
}

impl Step {
    fn new(
        definition: &'static SettingDefinition,
        before: SettingValue,
        after: SettingValue,
    ) -> Option<Self> {
        (after != before).then_some(Self {
            definition,
            before,
            after,
        })
    }

    /// The settings that differ between the two states, in the order of the registry.
    fn between(now: &AiState, next: &AiState) -> Vec<Self> {
        let text = |text: &str| SettingValue::Text(text.to_owned());
        [
            Self::new(
                AI_ENABLED.definition(),
                SettingValue::Bool(now.organization_on),
                SettingValue::Bool(next.organization_on),
            ),
            Self::new(
                AI_PROVIDER.definition(),
                text(now.kind.code()),
                text(next.kind.code()),
            ),
            Self::new(AI_MODEL.definition(), text(&now.model), text(&next.model)),
            Self::new(
                AI_ADDRESS.definition(),
                text(&now.address),
                text(&next.address),
            ),
            Self::new(
                AI_SEND_NAMES.definition(),
                SettingValue::Bool(now.send_names),
                SettingValue::Bool(next.send_names),
            ),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn field_change(&self) -> FieldChange {
        FieldChange {
            field: field_of(self.definition.key()),
            before: Some(json_of(&self.before)),
            after: Some(json_of(&self.after)),
        }
    }
}

/// Changes the settings of AI of the organization.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*, `request.invalid` for a model that
/// is too long or missing while AI is switched on, `ai.address-invalid` and
/// `ai.address-not-encrypted` for an address that cannot be used, `auth.wrong-credentials` when the
/// provider or the address changes without the right password, `ai.agreement-needed` when AI would
/// be on for an outside provider that has no confirmed agreement.
pub async fn change_ai_setup<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeAiSetup,
) -> Result<Authorized<AiSetup>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let now_state = ctx.ai_state_at(None).await?;
    let model = input.model.map(|model| model.trim().to_owned());
    let address = input.address.map(|address| address.trim().to_owned());
    if model
        .as_ref()
        .is_some_and(|model| model.chars().count() > MODEL_MAX)
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let next = AiState {
        organization_on: input.enabled.unwrap_or(now_state.organization_on),
        place_on: input.enabled.unwrap_or(now_state.organization_on),
        kind: input.provider.unwrap_or(now_state.kind),
        model: model.clone().unwrap_or_else(|| now_state.model.clone()),
        address: address.clone().unwrap_or_else(|| now_state.address.clone()),
        send_names: input.send_names.unwrap_or(now_state.send_names),
    };
    // An address that is entered must be a good one, also while AI is off.
    let next_address = next.model_address().or_else(|code| {
        if next.address.is_empty() && !next.organization_on {
            Ok(None)
        } else {
            Err(code)
        }
    })?;
    let destination = destination_of(next.kind, next_address.as_ref());
    if next.organization_on && (next.model.is_empty() || next_address_missing(&next)) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    // The provider moves when its kind does, or its address while the kind uses one: the password
    // is asked again, the agreement and the key of the old one go. A stale address under
    // Anthropic is not a provider.
    let moves = next.kind != now_state.kind
        || (next.kind == ProviderKind::OpenAiCompatible && next.address != now_state.address);
    if moves {
        let password = input.password.ok_or(ErrorCode::WrongCredentials)?;
        ctx.confirm_password(&actor.person, password).await?;
    }
    let stored = ctx.ai_agreement().await?;
    let agreement_kept = !moves && AgreementStatus::of(destination, stored.as_ref()).holds();
    if next.organization_on && destination.row().needs_agreement && !agreement_kept {
        return Err(ErrorCode::AiAgreementNeeded.into());
    }
    let steps = Step::between(&now_state, &next);
    if steps.is_empty() {
        return Ok(access.authorize(ctx.ai_setup_now(&access).await?));
    }
    let mut changes: Vec<FieldChange> = steps.iter().map(Step::field_change).collect();
    if moves {
        changes.push(changed("ai.agreement"));
        changes.push(changed("ai.key"));
    }
    let subject = fixed_entity(RecordKind::Setting, AI_RECORD)?;
    let event = ctx.event(by(actor.person.id), subject.clone(), changes)?;
    let now = ctx.now();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                for step in &steps {
                    S::put_setting_value(
                        transaction,
                        Scope::Organization,
                        step.definition,
                        &step.after,
                        now,
                        None,
                    )
                    .await?;
                }
                if moves {
                    S::delete_ai_agreement(transaction).await?;
                    S::forget_ai_key(transaction).await?;
                }
                Ok(())
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(
        SecurityAction::AiSettingsChanged,
        Some(actor.person.id),
        Some(subject),
    )
    .await;
    Ok(access.authorize(ctx.ai_setup_now(&access).await?))
}

/// Whether the settings need an address and have none: an OpenAI-compatible model is reached at
/// the address that is entered.
fn next_address_missing(state: &AiState) -> bool {
    state.kind == ProviderKind::OpenAiCompatible && state.address.is_empty()
}

/// Enters, changes or removes the key of the provider. It is stored encrypted like other secrets
/// (SEC-16); the event says only that it changed, and the security log keeps the fact.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*, `auth.wrong-credentials` for a wrong
/// password, `request.invalid` for a key that is too long.
pub async fn set_ai_key<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SetAiKey,
) -> Result<Authorized<Done>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let key = input.key.filter(|key| !key.is_empty());
    if key.as_ref().is_some_and(|key| key.len() > KEY_MAX) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    ctx.confirm_password(&actor.person, input.password).await?;
    let failed = |error| internal::<S>(&error);
    match &key {
        Some(key) => ctx
            .store()
            .put_ai_key(key.as_bytes(), ctx.now())
            .await
            .map_err(failed)?,
        None => ctx.store().delete_ai_key().await.map_err(failed)?,
    }
    let subject = fixed_entity(RecordKind::Setting, AI_RECORD)?;
    let event = ctx.event(
        by(actor.person.id),
        subject.clone(),
        vec![changed("ai.key")],
    )?;
    // The event is the change itself: the secret was written above, and its value is in no event.
    ctx.store()
        .record_change(&event, |_| Box::pin(async { Ok(()) }))
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(
        SecurityAction::AiKeyChanged,
        Some(actor.person.id),
        Some(subject),
    )
    .await;
    Ok(access.authorize(Done))
}

/// Confirms that a data processing agreement with the provider is in place, with who and when.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*, `request.invalid` when the
/// destination is not the one the settings lead to now or needs no agreement.
pub async fn confirm_ai_agreement<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ConfirmAiAgreement,
) -> Result<Authorized<AiSetup>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let destination = ctx.ai_state_at(None).await?.destination()?;
    if destination != input.destination || !destination.row().needs_agreement {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let agreement = Agreement {
        destination,
        confirmed_by: actor.person.id,
        confirmed_at: ctx.now(),
    };
    let subject = fixed_entity(RecordKind::Setting, AI_RECORD)?;
    let event = ctx.event(
        by(actor.person.id),
        subject.clone(),
        vec![changed("ai.agreement")],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::put_ai_agreement(transaction, &agreement).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(
        SecurityAction::AiAgreementConfirmed,
        Some(actor.person.id),
        Some(subject),
    )
    .await;
    Ok(access.authorize(ctx.ai_setup_now(&access).await?))
}

/// Switches AI on or off for one project, or lets it follow the project above again.
///
/// # Errors
/// `access.forbidden` without *Change project settings* in the project, `resource.not-found` for a
/// project the person may not know about.
pub async fn change_project_ai<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeProjectAi,
) -> Result<Authorized<ProjectAi>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let project = ctx
        .find_project(&input.project)
        .await?
        .ok_or(ErrorCode::NotFound)?
        .project
        .id;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeProjectSettings,
            Place::Project(project),
        )
        .await?;
    let scope = Scope::Project(project);
    let definition = AI_ENABLED.definition();
    let layers = ctx
        .store()
        .setting_layers(Some(project), None)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let before = layers
        .projects
        .iter()
        .find(|(id, _)| *id == project)
        .and_then(|(_, values)| values.get(definition.key()))
        .map(json_of);
    let after = input.enabled.map(SettingValue::Bool);
    let record = fixed_entity(RecordKind::Setting, &record_of(scope, definition.key()))?;
    let event = ctx.event(
        by(actor.person.id),
        record.clone(),
        vec![FieldChange {
            field: field_of(definition.key()),
            before,
            after: after.as_ref().map(json_of),
        }],
    )?;
    let now = ctx.now();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                match &after {
                    Some(value) => {
                        S::put_setting_value(transaction, scope, definition, value, now, None)
                            .await
                            .map(|_| ())
                    }
                    None => S::clear_setting_value(transaction, scope, definition)
                        .await
                        .map(|_| ()),
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(
        SecurityAction::AiProjectSwitchChanged,
        Some(actor.person.id),
        Some(record),
    )
    .await;
    let layers = ctx
        .store()
        .setting_layers(Some(project), None)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let state = AiState::of(&layers);
    let own = input.enabled;
    Ok(access.authorize(ProjectAi {
        own,
        organization_on: state.organization_on,
        works_here: state.counts(),
    }))
}

/// Tries the provider with one short call that holds no data of the organization. It works while AI
/// is still off, so an administrator can see that it answers before switching it on, but never
/// before the agreement of an outside provider is confirmed.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*, `rate-limit.exceeded` for a second
/// try within a few seconds, `ai.agreement-needed`, `ai.unavailable` when the provider did not
/// answer.
pub async fn try_ai_provider<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: TryAiProvider,
) -> Result<Authorized<Done>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let state = ctx.ai_state_at(None).await?;
    ctx.reserve_ai_try(actor.person.id)?;
    let request = AiRequest {
        system: String::new(),
        turns: vec![AiTurn {
            role: AiRole::User,
            parts: vec![AiPart::Text(TRY_TEXT.to_owned())],
        }],
        tools: Vec::new(),
        max_tokens: 16,
    };
    let answered = ctx.call_provider(&state, &request).await;
    // A try is an event whether or not it worked; the answer follows it.
    let event = ctx.event(
        by(actor.person.id),
        fixed_entity(RecordKind::Setting, AI_RECORD)?,
        vec![changed(if answered.is_ok() {
            "ai.try-answered"
        } else {
            "ai.try-failed"
        })],
    )?;
    ctx.store()
        .record_change(&event, |_| Box::pin(async { Ok(()) }))
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    answered?;
    Ok(access.authorize(Done))
}
