//! How an AI app signs in (AI-22, ADR-0042): registration, the consent step, the swap of a code or a
//! renewing token for tokens, *Connected apps* and *Take back*, and the web addresses an
//! administrator allowed.
//!
//! The rules that hold in every use case here:
//! - authorization code with PKCE `S256`, nothing else; an app is a public client and holds no
//!   secret of its own;
//! - an app on the person's own computer registers itself; an app on a web address only when an
//!   administrator allowed that address, checked at registration and again at every sign-in;
//! - a code works once, for 60 seconds, and only with its proof and the address it was given for; a
//!   code used again ends the grant it made;
//! - an access token lives 15 minutes; a renewing token turns over at each use, and an old one used
//!   again ends the whole grant;
//! - a grant holds *See only* or *See and change* and optionally projects, and only narrows the
//!   person (`narrowing_for`, the same function as for access keys);
//! - every code and token is stored as a hash and never written to a log;
//! - the failed tries of registration and of the token endpoint count per network like sign-in.

use std::collections::BTreeSet;

use bb_auth::{OAuthKind, OAuthSecret, is_pkce_challenge};
use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::event::RecordKind;
use bb_core::id::{GrantId, OAuthClientId, ProjectId};
use bb_core::oauth::{
    APP_NAME_LENGTH_MAX, GrantEnd, MCP_PATH, MOST_GRANT_PROJECTS, MOST_GRANTS,
    MOST_RETURN_ADDRESSES, OAuthClient, OAuthCode, OAuthGrant, OAuthProblem, PKCE_METHOD,
    ReturnAddress, ReturnKind, code_life, is_registered, with_query,
};
use bb_core::settings::{AI_APPS_ENABLED, AI_ENABLED};
use bb_core::sign_in::SecurityAction;
use bb_core::time::days;
use bb_policy::{Authorized, Place};
use serde_json::json;

use super::ctx::Ctx;
use super::error::{AppError, Stops, change_failed, internal};
use super::events::{by, entity, set_to};
use super::oauth_store::GrantRecord;
use super::oauth_types::{
    AuthorizationAnswer, AuthorizationQuery, AuthorizationRequest, AuthorizationView,
    ConnectedAppView, ConnectedApps, ConnectedAppsQuery, DecideAuthorization, RegisterApp,
    RegisterOutcome, RegisteredApp, TakeBack,
};
use super::store::AuthStore;
use super::types::Done;
use crate::access::Actor;
use crate::input::clean_text;

/// The purpose under which the failed tries of registration are counted per network.
const REGISTERING: &str = "oauth-register";

/// The scope an app asked for, as a choice of access keys. Empty means *See only*; anything that
/// is not one of the two choices an app can have is no scope.
fn asked_scope(text: &str) -> Option<KeyChoice> {
    if text.is_empty() {
        return Some(KeyChoice::SeeOnly);
    }
    KeyChoice::from_code(text.trim()).filter(|choice| choice.is_for_apps())
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether AI apps may register and sign in now: AI is on for the organization and an
    /// administrator has not switched apps off.
    pub(crate) async fn apps_may_sign_in(&self) -> Result<bool, AppError> {
        let layers = self.setting_layers_of(None).await?;
        Ok(layers.yes(&AI_ENABLED) && layers.yes(&AI_APPS_ENABLED))
    }

    /// The return addresses administrators allowed.
    async fn allowed_web_addresses(&self) -> Result<Vec<String>, AppError> {
        Ok(self
            .store()
            .oauth_web_addresses()
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|each| each.address)
            .collect())
    }

    /// Whether every address of the app can be used now: each is on this computer or a web address
    /// an administrator allowed.
    async fn addresses_can_be_used(&self, addresses: &[String]) -> Result<bool, AppError> {
        let mut allowed: Option<Vec<String>> = None;
        for address in addresses {
            match ReturnAddress::read(address).map(|read| read.kind()) {
                Some(ReturnKind::OnThisComputer) => {}
                Some(ReturnKind::Web) => {
                    if allowed.is_none() {
                        allowed = Some(self.allowed_web_addresses().await?);
                    }
                    if !allowed.as_deref().unwrap_or_default().contains(address) {
                        return Ok(false);
                    }
                }
                None => return Ok(false),
            }
        }
        Ok(true)
    }

    /// The app of a request, the address it comes back to and what it asked, checked as the consent
    /// step needs it: a known app, a registered address that can be used now, `code` with
    /// `S256`, a challenge, a scope an app can have and the resource of this server.
    async fn checked_request(
        &self,
        request: &AuthorizationRequest,
    ) -> Result<(OAuthClient, KeyChoice), AppError> {
        let client_id = request
            .client_id
            .parse::<OAuthClientId>()
            .map_err(|_| AppError::Refused(ErrorCode::NotFound))?;
        let client = self
            .store()
            .oauth_client(client_id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;

        // The address is judged first and alone: an app that did not register it is never sent
        // anything, and the person is told it is not a known app.
        if !is_registered(&client.return_addresses, &request.return_address)
            || !self
                .addresses_can_be_used(std::slice::from_ref(&request.return_address))
                .await?
        {
            self.note(SecurityAction::OAuthRefused, None, None).await;
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        let challenge_ok = request.challenge_method == PKCE_METHOD
            && is_pkce_challenge(&request.challenge)
            && request.response_type == "code";
        let scope = asked_scope(&request.scope);
        let resource_ok = request.resource.is_empty() || request.resource == self.mcp_resource();
        match (challenge_ok, scope, resource_ok) {
            (true, Some(scope), true) => Ok((client, scope)),
            _ => {
                self.note(SecurityAction::OAuthRefused, None, None).await;
                Err(AppError::Refused(ErrorCode::InvalidRequest))
            }
        }
    }

    /// The resource a token is for: the MCP server of this server.
    pub(crate) fn mcp_resource(&self) -> String {
        format!("{}{MCP_PATH}", self.services.public_url)
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The app a registration asks to make, or why it is turned away. Nothing is written here.
    async fn app_to_register(
        &self,
        input: &RegisterApp,
    ) -> Result<Result<OAuthClient, OAuthProblem>, AppError> {
        if !self.apps_may_sign_in().await? {
            return Ok(Err(OAuthProblem::TemporarilyUnavailable));
        }
        let name = input.name.trim();
        let Ok(name) = clean_text(name, APP_NAME_LENGTH_MAX) else {
            return Ok(Err(OAuthProblem::InvalidClientMetadata));
        };
        let addresses: BTreeSet<String> = input.return_addresses.iter().cloned().collect();
        let listed: Vec<String> = addresses.iter().cloned().collect();
        if addresses.is_empty()
            || addresses.len() > MOST_RETURN_ADDRESSES
            || !self.addresses_can_be_used(&listed).await?
        {
            return Ok(Err(OAuthProblem::InvalidRedirectUri));
        }
        Ok(Ok(OAuthClient {
            id: OAuthClientId::generate(&mut self.ids()),
            name,
            return_addresses: listed,
            registered_at: self.now(),
        }))
    }
}

/// Registers an app (RFC 7591).
///
/// # Errors
/// A limit when a network has failed too often.
pub async fn register_app<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RegisterApp,
) -> Result<RegisterOutcome, AppError> {
    let slot = ctx.reserve_network(REGISTERING)?;
    let client = match ctx.app_to_register(&input).await {
        Ok(Ok(client)) => client,
        Ok(Err(problem)) => {
            ctx.slot_failed(slot, None).await;
            ctx.note(SecurityAction::OAuthRefused, None, None).await;
            return Ok(RegisterOutcome::Refused(problem));
        }
        Err(error) => {
            ctx.slot_given_back(&slot);
            return Err(error);
        }
    };
    let event = ctx.event(
        bb_core::event::Actor::System,
        entity(RecordKind::OAuthClient, client.id),
        vec![set_to("name", json!(client.name))],
    )?;
    let stored = client.clone();
    let written = ctx
        .store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::insert_oauth_client(transaction, &stored).await })
        })
        .await;
    ctx.slot_given_back(&slot);
    written.map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(
        SecurityAction::OAuthClientRegistered,
        None,
        Some(entity(RecordKind::OAuthClient, client.id)),
    )
    .await;
    Ok(RegisterOutcome::Registered(RegisteredApp {
        client_id: client.id,
        name: client.name,
        return_addresses: client.return_addresses,
        registered_at: client.registered_at,
    }))
}

/// What the page that asks the person shows: the app, where it comes back to and what it asks.
///
/// # Errors
/// `resource.not-found` for an app or an address that is not known (the same answer for both),
/// `request.invalid` for a request that is not one, `ai.not-set-up`-free: while apps are off the
/// answer is `access.forbidden`.
pub async fn authorization<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AuthorizationQuery,
) -> Result<Authorized<AuthorizationView>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    if !ctx.apps_may_sign_in().await? {
        return Err(AppError::Refused(ErrorCode::Forbidden));
    }
    let (client, asked) = ctx.checked_request(&input.request).await?;
    let access = ctx.evaluate(record.person.id, Place::Organization).await?;
    Ok(access.authorize(AuthorizationView {
        app_name: client.name,
        on_this_computer: ReturnAddress::read(&input.request.return_address)
            .is_some_and(|read| read.kind() == ReturnKind::OnThisComputer),
        asked,
    }))
}

/// The person says *Allow* or *Cancel*. *Allow* makes the grant and a code; either way the browser
/// goes back to the app.
///
/// # Errors
/// What [`authorization`] answers; `request.invalid` for a scope wider than the app asked, or one
/// an app cannot have, or too many projects or grants; `resource.not-found` for a project the
/// person may not know about.
pub async fn decide_authorization<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DecideAuthorization,
) -> Result<Authorized<AuthorizationAnswer>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    if !ctx.apps_may_sign_in().await? {
        return Err(AppError::Refused(ErrorCode::Forbidden));
    }
    let (client, asked) = ctx.checked_request(&input.request).await?;
    let person = record.person.id;
    let access = ctx.evaluate(person, Place::Organization).await?;
    let state = [("state", input.request.state.as_str())];
    let state: &[(&str, &str)] = if input.request.state.is_empty() {
        &[]
    } else {
        &state
    };
    if !input.allow {
        let mut pairs = vec![("error", OAuthProblem::AccessDenied.code())];
        pairs.extend_from_slice(state);
        return Ok(access.authorize(AuthorizationAnswer {
            redirect_to: with_query(&input.request.return_address, &pairs),
        }));
    }
    // The person may narrow what the app asked and never widen it.
    if !input.scope.is_for_apps() || !asked.covers_choice(input.scope) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let projects: Vec<ProjectId> = input
        .projects
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if projects.len() > MOST_GRANT_PROJECTS {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let actor = Actor {
        person: record.person.clone(),
        key: None,
    };
    ctx.ensure_projects_known(&actor, &projects).await?;
    let held = ctx
        .store()
        .grant_count_of(person)
        .await
        .map_err(|error| internal::<S>(&error))?;
    if usize::try_from(held).map_or(true, |held| held >= MOST_GRANTS) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let settings = ctx.settings().await?;
    let now = ctx.now();
    let longest = now
        .checked_add(days(settings.access_key_longest_days))
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let grant = OAuthGrant {
        id: GrantId::generate(&mut ctx.ids()),
        client: client.id,
        person,
        scope: input.scope,
        allowed_at: now,
        last_used_at: None,
        ends_at: longest,
        ended_at: None,
        ended: None,
    };
    let code = OAuthSecret::generate(OAuthKind::Code)?;
    let stored_code = OAuthCode {
        secret_hash: code.hash(),
        grant: grant.id,
        return_address: input.request.return_address.clone(),
        challenge: input.request.challenge.clone(),
        expires_at: now
            .checked_add(code_life())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        used_at: None,
    };
    let event = ctx.event(
        by(person),
        entity(RecordKind::ConnectedApp, grant.id),
        vec![
            set_to("app", json!(client.name)),
            set_to("scope", json!(grant.scope.code())),
            set_to("ends_at", json!(grant.ends_at.to_string())),
        ],
    )?;
    let record_to_store = GrantRecord {
        grant: grant.clone(),
        projects,
    };
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::insert_grant(transaction, &record_to_store).await?;
                S::insert_oauth_code(transaction, &stored_code).await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(
        SecurityAction::OAuthGrantCreated,
        Some(person),
        Some(entity(RecordKind::ConnectedApp, grant.id)),
    )
    .await;
    let mut pairs = vec![("code", code.encode())];
    if !input.request.state.is_empty() {
        pairs.push(("state", input.request.state.clone()));
    }
    let pairs: Vec<(&str, &str)> = pairs.iter().map(|(a, b)| (*a, b.as_str())).collect();
    Ok(access.authorize(AuthorizationAnswer {
        redirect_to: with_query(&input.request.return_address, &pairs),
    }))
}

/// The apps the person allowed.
///
/// # Errors
/// `auth.not-signed-in` without a session.
pub async fn connected_apps<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ConnectedAppsQuery,
) -> Result<Authorized<ConnectedApps>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let actor = Actor {
        person: record.person,
        key: None,
    };
    let grants = ctx
        .store()
        .grants_of(person, MOST_GRANTS as u64)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let mut items = Vec::with_capacity(grants.len());
    for found in grants {
        let Some(client) = ctx
            .store()
            .oauth_client(found.grant.client)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            continue;
        };
        let mut projects = Vec::new();
        for project in &found.projects {
            projects.push(ctx.project_label(&actor, *project).await?);
        }
        items.push(ConnectedAppView {
            id: found.grant.id,
            name: client.name,
            on_this_computer: client.return_addresses.iter().all(|address| {
                ReturnAddress::read(address)
                    .is_some_and(|read| read.kind() == ReturnKind::OnThisComputer)
            }),
            scope: found.grant.scope,
            projects,
            allowed_at: found.grant.allowed_at,
            last_used_at: found.grant.last_used_at,
        });
    }
    let access = ctx.evaluate(person, Place::Organization).await?;
    Ok(access.authorize(ConnectedApps { items }))
}

/// Ends what the person allowed an app, at once: its tokens are dead at the next call.
///
/// # Errors
/// `resource.not-found` for a grant that is not the person's (it does not say whether it exists).
pub async fn take_back<S: AuthStore>(ctx: &Ctx<'_, S>, input: TakeBack) -> Result<Done, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let (person, id) = (record.person.id, input.id);
    let found = ctx
        .store()
        .oauth_grant(id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .filter(|found| found.grant.person == person && found.grant.ended_at.is_none())
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let now = ctx.now();
    let event = ctx.event(
        by(person),
        entity(RecordKind::ConnectedApp, found.grant.id),
        vec![set_to("ended", json!(GrantEnd::TakenBack.code()))],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::end_grant(transaction, id, GrantEnd::TakenBack, now).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| {
            change_failed::<S>(&error, Stops::new(ErrorCode::NotFound, ErrorCode::NotFound))
        })?;
    ctx.note(
        SecurityAction::OAuthGrantTakenBack,
        Some(person),
        Some(entity(RecordKind::ConnectedApp, id)),
    )
    .await;
    Ok(Done)
}
