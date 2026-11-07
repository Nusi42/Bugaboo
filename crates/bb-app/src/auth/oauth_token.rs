//! The swap of a code or a renewing token of an AI app for tokens (AI-22, ADR-0042, decision 3):
//! checking the secret and the proof, using it up in the same change that writes the new tokens, and
//! ending the whole grant when a code or an old renewing token is used again.

use bb_auth::{OAuthKind, OAuthSecret, pkce_proves};
use bb_core::event::RecordKind;
use bb_core::id::GrantId;
use bb_core::oauth::{
    GrantEnd, OAuthGrant, OAuthProblem, OAuthToken, REFRESH_UNUSED_DAYS, TokenKind,
    access_token_life,
};
use bb_core::sign_in::SecurityAction;
use bb_core::time::days;
use jiff::Timestamp;
use serde_json::json;

use super::ctx::Ctx;
use super::error::{AppError, Stops, change_failed, internal};
use super::events::{by, entity, set_to};
use super::oauth_store::GrantRecord;
use super::oauth_types::{TokenAnswer, TokenOutcome, TokenRequest};
use super::store::AuthStore;

/// The purpose under which the failed tries at the token endpoint are counted per network.
const SWAPPING: &str = "oauth-token";

/// What a swap was asked for, once it has been checked: the code or the renewing token that is
/// used up by it, and the grant it belongs to.
struct Verified {
    secret: bb_core::security_log::Digest,
    kind: TokenKind,
    grant: OAuthGrant,
    /// The end a second use of this secret brings to the grant.
    reuse: GrantEnd,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Ends a grant because a code or a renewing token was used again, and writes it down.
    async fn end_for_reuse(&self, grant: GrantId, how: GrantEnd) -> Result<(), AppError> {
        let Some(found) = self
            .store()
            .oauth_grant(grant)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(());
        };
        let person = found.grant.person;
        let now = self.now();
        let event = self.event(
            by(person),
            entity(RecordKind::ConnectedApp, grant),
            vec![set_to("ended", json!(how.code()))],
        )?;
        let ended = self
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move { S::end_grant(transaction, grant, how, now).await })
            })
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
        if ended {
            self.note(
                SecurityAction::OAuthGrantEndedByReuse,
                Some(person),
                Some(entity(RecordKind::ConnectedApp, grant)),
            )
            .await;
        }
        Ok(())
    }

    /// The grant a swap belongs to, if it works now and is for the app that asks.
    async fn working_grant(
        &self,
        grant: GrantId,
        client_id: &str,
        now: Timestamp,
    ) -> Result<Option<GrantRecord>, AppError> {
        let Some(found) = self
            .store()
            .oauth_grant(grant)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(None);
        };
        if !found.grant.works_at(now) || found.grant.client.to_string() != client_id {
            return Ok(None);
        }
        Ok(Some(found))
    }

    /// The tokens of a grant, to be written, and the answer to give once.
    fn new_tokens(
        grant: &OAuthGrant,
        now: Timestamp,
    ) -> Result<([OAuthToken; 2], TokenAnswer), AppError> {
        let access = OAuthSecret::generate(OAuthKind::Access)?;
        let refresh = OAuthSecret::generate(OAuthKind::Refresh)?;
        let soon = |life: jiff::SignedDuration| {
            now.checked_add(life)
                .map(|moment| moment.min(grant.ends_at))
                .map_err(|error| AppError::Internal(error.to_string()))
        };
        let access_row = OAuthToken {
            secret_hash: access.hash(),
            grant: grant.id,
            kind: TokenKind::Access,
            created_at: now,
            expires_at: soon(access_token_life())?,
            used_at: None,
        };
        let refresh_row = OAuthToken {
            secret_hash: refresh.hash(),
            grant: grant.id,
            kind: TokenKind::Refresh,
            created_at: now,
            expires_at: soon(days(REFRESH_UNUSED_DAYS))?,
            used_at: None,
        };
        let answer = TokenAnswer {
            access_token: access.encode().into(),
            expires_in: access_row.expires_at.as_second() - now.as_second(),
            refresh_token: refresh.encode().into(),
            scope: grant.scope,
        };
        Ok(([access_row, refresh_row], answer))
    }

    /// Checks a code and the proof of the app. Nothing is used up here.
    async fn verify_code(
        &self,
        request: &TokenRequest,
        now: Timestamp,
    ) -> Result<Result<Verified, OAuthProblem>, AppError> {
        use OAuthProblem::{InvalidGrant, InvalidRequest};
        let Some(code) = OAuthSecret::parse(OAuthKind::Code, &request.code) else {
            return Ok(Err(InvalidGrant));
        };
        let hash = code.hash();
        let Some(stored) = self
            .store()
            .oauth_code(&hash)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(Err(InvalidGrant));
        };
        if stored.used_at.is_some() {
            self.end_for_reuse(stored.grant, GrantEnd::CodeReused)
                .await?;
            return Ok(Err(InvalidGrant));
        }
        let Some(found) = self
            .working_grant(stored.grant, &request.client_id, now)
            .await?
        else {
            return Ok(Err(InvalidGrant));
        };
        if stored.expires_at <= now || stored.return_address != request.return_address {
            return Ok(Err(InvalidGrant));
        }
        if request.verifier.is_empty() {
            return Ok(Err(InvalidRequest));
        }
        if !pkce_proves(&stored.challenge, &request.verifier) {
            self.note(
                SecurityAction::OAuthRefused,
                Some(found.grant.person),
                Some(entity(RecordKind::ConnectedApp, found.grant.id)),
            )
            .await;
            return Ok(Err(InvalidGrant));
        }
        Ok(Ok(Verified {
            secret: hash,
            kind: TokenKind::Access,
            grant: found.grant,
            reuse: GrantEnd::CodeReused,
        }))
    }

    /// Checks a renewing token; an old one that is used again ends the whole grant.
    async fn verify_refresh(
        &self,
        request: &TokenRequest,
        now: Timestamp,
    ) -> Result<Result<Verified, OAuthProblem>, AppError> {
        use OAuthProblem::InvalidGrant;
        let Some(secret) = OAuthSecret::parse(OAuthKind::Refresh, &request.refresh_token) else {
            return Ok(Err(InvalidGrant));
        };
        let hash = secret.hash();
        let Some(stored) = self
            .store()
            .oauth_token(&hash)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|token| token.kind == TokenKind::Refresh)
        else {
            return Ok(Err(InvalidGrant));
        };
        if stored.used_at.is_some() {
            // Two parties hold the token: the whole grant ends.
            self.end_for_reuse(stored.grant, GrantEnd::TokenReused)
                .await?;
            return Ok(Err(InvalidGrant));
        }
        let Some(found) = self
            .working_grant(stored.grant, &request.client_id, now)
            .await?
        else {
            return Ok(Err(InvalidGrant));
        };
        if stored.expires_at <= now {
            return Ok(Err(InvalidGrant));
        }
        Ok(Ok(Verified {
            secret: hash,
            kind: TokenKind::Refresh,
            grant: found.grant,
            reuse: GrantEnd::TokenReused,
        }))
    }

    /// What a swap asks for, checked.
    async fn verified_swap(
        &self,
        request: &TokenRequest,
        now: Timestamp,
    ) -> Result<Result<Verified, OAuthProblem>, AppError> {
        if !self.apps_may_sign_in().await? {
            return Ok(Err(OAuthProblem::TemporarilyUnavailable));
        }
        if !request.resource.is_empty() && request.resource != self.mcp_resource() {
            return Ok(Err(OAuthProblem::InvalidTarget));
        }
        match request.grant_type.as_str() {
            "authorization_code" => self.verify_code(request, now).await,
            "refresh_token" => self.verify_refresh(request, now).await,
            _ => Ok(Err(OAuthProblem::UnsupportedGrantType)),
        }
    }
}

/// Swaps a code or a renewing token for tokens (RFC 6749 §4.1.3, §6). The code or the old renewing
/// token is used up in the same change that writes the new tokens, so two requests with one secret
/// cannot both get tokens: the second one ends the grant.
///
/// # Errors
/// A limit when a network has failed too often.
pub async fn exchange_token<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: TokenRequest,
) -> Result<TokenOutcome, AppError> {
    let slot = ctx.reserve_network(SWAPPING)?;
    let now = ctx.now();
    let verified = match ctx.verified_swap(&input, now).await {
        Ok(Ok(verified)) => verified,
        Ok(Err(problem)) => {
            ctx.slot_failed(slot, None).await;
            return Ok(TokenOutcome::Refused(problem));
        }
        Err(error) => {
            ctx.slot_given_back(&slot);
            return Err(error);
        }
    };
    ctx.slot_given_back(&slot);
    let (rows, answer) = Ctx::<S>::new_tokens(&verified.grant, now)?;
    let (secret, kind, grant) = (verified.secret, verified.kind, verified.grant.id);
    let first = ctx
        .store()
        .record_changes(&[], move |transaction| {
            Box::pin(async move {
                let first = match kind {
                    TokenKind::Access => S::use_oauth_code(transaction, &secret, now).await?,
                    TokenKind::Refresh => S::use_oauth_token(transaction, &secret, now).await?,
                };
                if first {
                    for row in &rows {
                        S::insert_oauth_token(transaction, row).await?;
                    }
                }
                Ok(first)
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    if !first {
        // Two requests with one secret at the same moment: one of them is not the app.
        ctx.end_for_reuse(grant, verified.reuse).await?;
        return Ok(TokenOutcome::Refused(OAuthProblem::InvalidGrant));
    }
    Ok(TokenOutcome::Issued(answer))
}
