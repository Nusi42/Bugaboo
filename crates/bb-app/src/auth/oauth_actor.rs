//! The actor behind the access token of an AI app (AI-22, AI-23, ADR-0042): the one way from a token
//! to an actor, beside the ways from a cookie and from an access key (`Ctx::actor`).
//!
//! The token only finds the grant. What the app may do is never kept with the grant: at every call
//! it is the person's evaluation narrowed by the scope and the projects of the grant, through the
//! same function as for an access key (`narrowing_for`). So a role taken away counts from the next
//! call, and what no key holds no grant holds.

use bb_auth::{OAuthKind, OAuthSecret};
use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::id::ProjectId;
use bb_core::oauth::{OAuthGrant, TokenKind};
use bb_core::person::Person;
use bb_core::sign_in::SecurityAction;

use super::access_keys::{narrowing_for, to_the_minute};
use super::ctx::Ctx;
use super::error::{AppError, internal};
use super::events::entity;
use super::store::AuthStore;
use crate::access::{Actor, CredentialSource, KeyInUse};

/// The purpose under which guessing tokens is counted per network.
const GUESSING: &str = "oauth-token-guess";

/// A token that works now: its grant, the projects of the grant, the name of its app and its person.
struct WorkingToken {
    grant: OAuthGrant,
    projects: Vec<ProjectId>,
    name: String,
    owner: Person,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The person behind the access token of an AI app, narrowed by what they allowed it.
    ///
    /// A token that does not work (unknown, run out, taken back, a grant that ended, AI apps
    /// switched off) gets the answer of any key that does not work, and guessing is limited per
    /// network. While the person has to set up a second step, a grant is refused like a key.
    ///
    /// # Errors
    /// `auth.not-signed-in` for a token that does not work, 429 for a network that guessed too
    /// often, `auth.second-step-required` while the person has to set up a second step.
    pub(crate) async fn grant_actor(&self, token_text: &str) -> Result<Actor, AppError> {
        // AI apps off is no guess of a network: the answer is the one of any token that does not
        // work, and nothing is counted.
        if !self.apps_may_sign_in().await? {
            return Err(AppError::Refused(ErrorCode::NotSignedIn));
        }
        let Some(found) = self.working_token(token_text).await? else {
            return Err(self
                .credential_guessed(GUESSING, SecurityAction::OAuthRefused)
                .await);
        };
        let WorkingToken {
            grant,
            projects,
            name,
            owner,
        } = found;
        self.owner_may_use_credential(
            owner.id,
            SecurityAction::OAuthRefused,
            entity(RecordKind::ConnectedApp, grant.id),
        )
        .await?;
        self.enter_mcp();
        // A busy app does not write at all within the minute it was last used in.
        let minute = to_the_minute(self.now());
        if grant.last_used_at != Some(minute) {
            self.store()
                .touch_grant(grant.id, minute)
                .await
                .map_err(|error| internal::<S>(&error))?;
        }
        Ok(Actor {
            person: owner,
            key: Some(KeyInUse {
                source: CredentialSource::App {
                    grant: grant.id,
                    name,
                },
                narrowing: narrowing_for(grant.scope, &[], &projects),
                agent: None,
            }),
        })
    }

    /// The grant, its projects, the name of its app and its person, for a token that works now.
    async fn working_token(&self, token_text: &str) -> Result<Option<WorkingToken>, AppError> {
        let Some(secret) = OAuthSecret::parse(OAuthKind::Access, token_text) else {
            return Ok(None);
        };
        let now = self.now();
        let Some(token) = self
            .store()
            .oauth_token(&secret.hash())
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|token| token.kind == TokenKind::Access && token.expires_at > now)
        else {
            return Ok(None);
        };
        let Some(found) = self
            .store()
            .oauth_grant(token.grant)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|found| found.grant.works_at(now))
        else {
            return Ok(None);
        };
        let Some(client) = self
            .store()
            .oauth_client(found.grant.client)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(None);
        };
        let Some(owner) = self.person_known(found.grant.person).await? else {
            return Ok(None);
        };
        Ok(Some(WorkingToken {
            grant: found.grant,
            projects: found.projects,
            name: client.name,
            owner,
        }))
    }
}
