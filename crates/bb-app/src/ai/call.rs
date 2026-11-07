//! The one way to call the provider (AI-01, AI-02, PRIV-08): whether AI counts at a place, which
//! provider the settings lead to, and the call itself with its time limit and the sentence a
//! failure becomes. Every way in (the assistant, MCP, agents, the try of an administrator) goes
//! through here, so the rules are written once.

use bb_core::ErrorCode;
use bb_core::ai::{
    ANSWER_TOKENS_MAX, Agreement, Destination, ModelAddress, ProviderKind, counts_here,
    destination_of,
};
use bb_core::id::ProjectId;
use bb_core::settings::{AI_ADDRESS, AI_ENABLED, AI_MODEL, AI_PROVIDER, AI_SEND_NAMES, Layers};
use zeroize::Zeroizing;

use super::{AiAnswer, AiRequest, AiTarget};
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The settings of AI as they are at one place.
#[derive(Clone, Debug)]
pub(crate) struct AiState {
    /// The organization has AI on.
    pub(crate) organization_on: bool,
    /// The place has AI on: its own switch, the one of the project above, or the organization's.
    pub(crate) place_on: bool,
    /// The kind of provider.
    pub(crate) kind: ProviderKind,
    /// The model.
    pub(crate) model: String,
    /// The address of an OpenAI-compatible model, as entered.
    pub(crate) address: String,
    /// Whether names of people go to an outside provider.
    pub(crate) send_names: bool,
}

impl AiState {
    /// The state the layers lead to.
    pub(crate) fn of(layers: &Layers) -> Self {
        let organization_only = Layers {
            organization: layers.organization.clone(),
            ..Layers::default()
        };
        Self {
            organization_on: organization_only.yes(&AI_ENABLED),
            place_on: layers.yes(&AI_ENABLED),
            kind: ProviderKind::from_code(&layers.text(&AI_PROVIDER))
                .unwrap_or(ProviderKind::Anthropic),
            model: layers.text(&AI_MODEL),
            address: layers.text(&AI_ADDRESS),
            send_names: layers.yes(&AI_SEND_NAMES),
        }
    }

    /// AI counts at this place (AI-02): the one answer every way in asks.
    pub(crate) const fn counts(&self) -> bool {
        counts_here(self.organization_on, self.place_on)
    }

    /// The address of the model, when the kind of provider has one and it is a good one.
    pub(crate) fn model_address(&self) -> Result<Option<ModelAddress>, ErrorCode> {
        match self.kind {
            ProviderKind::Anthropic => Ok(None),
            ProviderKind::OpenAiCompatible => {
                ModelAddress::parse(&self.address)
                    .map(Some)
                    .map_err(|problem| match problem {
                        bb_core::ai::AddressProblem::NotAnAddress => ErrorCode::AiAddressInvalid,
                        bb_core::ai::AddressProblem::NotEncrypted => {
                            ErrorCode::AiAddressNotEncrypted
                        }
                    })
            }
        }
    }

    /// Where the data of a call goes, when the address is a good one.
    pub(crate) fn destination(&self) -> Result<Destination, ErrorCode> {
        Ok(destination_of(self.kind, self.model_address()?.as_ref()))
    }
}

/// Whether the agreement that is stored covers the destination, or none is needed.
pub(crate) fn agreement_holds(destination: Destination, agreement: Option<&Agreement>) -> bool {
    !destination.row().needs_agreement
        || agreement.is_some_and(|agreement| agreement.destination == destination)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The settings of AI at a place: a project, or the organization.
    pub(crate) async fn ai_state_at(&self, place: Option<ProjectId>) -> Result<AiState, AppError> {
        let layers = self
            .store()
            .setting_layers(place, None)
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(AiState::of(&layers))
    }

    /// The agreement an administrator confirmed, if there is one.
    pub(crate) async fn ai_agreement(&self) -> Result<Option<Agreement>, AppError> {
        self.store()
            .ai_agreement()
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Where and how to call for these settings, with the key. Nothing is called here.
    ///
    /// # Errors
    /// `ai.agreement-needed` while an outside provider has no agreement, `ai.address-invalid` and
    /// `ai.address-not-encrypted` for an address that is not usable, `ai.not-set-up` while the
    /// model or the key that the provider needs is missing.
    pub(crate) async fn ai_target(&self, state: &AiState) -> Result<AiTarget, AppError> {
        let address = state.model_address()?;
        let destination = destination_of(state.kind, address.as_ref());
        if !agreement_holds(destination, self.ai_agreement().await?.as_ref()) {
            return Err(ErrorCode::AiAgreementNeeded.into());
        }
        let key = self
            .store()
            .ai_key()
            .await
            .map_err(|error| internal::<S>(&error))?
            .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
            .unwrap_or_default();
        if state.model.is_empty() || (state.kind == ProviderKind::Anthropic && key.is_empty()) {
            return Err(ErrorCode::AiNotSetUp.into());
        }
        Ok(AiTarget {
            kind: state.kind,
            endpoint: address.map(|address| address.endpoint()),
            model: state.model.clone(),
            key: Zeroizing::new(key),
        })
    }

    /// Calls the provider for a way in that works at `place`, once. The answer to a failure is one
    /// sentence; what the provider sent back is never passed on (AI-01).
    ///
    /// # Errors
    /// `ai.off` while AI does not count at the place, the errors of [`Self::ai_target`], and
    /// `ai.unavailable` when the provider gave no usable answer.
    pub async fn ask_ai(
        &self,
        place: Option<ProjectId>,
        request: &AiRequest,
    ) -> Result<AiAnswer, AppError> {
        let state = self.ai_state_at(place).await?;
        if !state.counts() {
            return Err(ErrorCode::AiOff.into());
        }
        self.call_provider(&state, request).await
    }

    /// The call itself, for settings that were checked: with the target of the settings and the
    /// limit of output.
    pub(crate) async fn call_provider(
        &self,
        state: &AiState,
        request: &AiRequest,
    ) -> Result<AiAnswer, AppError> {
        let target = self.ai_target(state).await?;
        let request = AiRequest {
            max_tokens: request.max_tokens.min(ANSWER_TOKENS_MAX),
            ..request.clone()
        };
        self.services
            .ai
            .complete(&target, &request)
            .await
            .map_err(|_| ErrorCode::AiUnavailable.into())
    }
}
