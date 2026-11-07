//! The sign-in of AI apps in the in-memory store (AI-22, ADR-0042).

use bb_core::id::{GrantId, OAuthClientId, UserId};
use bb_core::oauth::{GrantEnd, OAuthClient, OAuthCode, OAuthToken, OAuthWebAddress, TokenKind};
use bb_core::security_log::Digest;
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, Tx};
use crate::auth::{GrantRecord, GrantWithApp, OAuthCleaned, OAuthStore};

/// What the in-memory store keeps of the sign-in of apps.
#[derive(Clone, Default)]
pub(crate) struct OAuthState {
    pub(crate) clients: Vec<OAuthClient>,
    pub(crate) grants: Vec<GrantRecord>,
    pub(crate) codes: Vec<OAuthCode>,
    pub(crate) tokens: Vec<OAuthToken>,
    pub(crate) web_addresses: Vec<OAuthWebAddress>,
}

/// Ends a grant that has not ended, and takes away its codes and tokens.
fn end(state: &mut super::fake::State, id: GrantId, how: GrantEnd, at: Timestamp) -> bool {
    let Some(found) = state
        .oauth
        .grants
        .iter_mut()
        .find(|found| found.grant.id == id && found.grant.ended_at.is_none())
    else {
        return false;
    };
    found.grant.ended_at = Some(at);
    found.grant.ended = Some(how);
    state.oauth.codes.retain(|code| code.grant != id);
    state.oauth.tokens.retain(|token| token.grant != id);
    true
}

impl OAuthStore for FakeStore {
    async fn oauth_client(&self, id: OAuthClientId) -> Result<Option<OAuthClient>, FakeError> {
        Ok(self.read(|state| state.oauth.clients.iter().find(|c| c.id == id).cloned()))
    }

    async fn oauth_grant(&self, id: GrantId) -> Result<Option<GrantRecord>, FakeError> {
        Ok(self.read(|state| {
            state
                .oauth
                .grants
                .iter()
                .find(|found| found.grant.id == id)
                .cloned()
        }))
    }

    async fn grants_of(
        &self,
        person: UserId,
        now: Timestamp,
        limit: u64,
    ) -> Result<Vec<GrantWithApp>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<GrantRecord> = state
                .oauth
                .grants
                .iter()
                .filter(|found| found.grant.person == person && found.grant.works_at(now))
                .cloned()
                .collect();
            found.sort_by_key(|each| std::cmp::Reverse(each.grant.allowed_at));
            found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
            found
                .into_iter()
                .filter_map(|record| {
                    state
                        .oauth
                        .clients
                        .iter()
                        .find(|client| client.id == record.grant.client)
                        .cloned()
                        .map(|client| GrantWithApp { record, client })
                })
                .collect()
        }))
    }

    async fn grant_count_of(&self, person: UserId, now: Timestamp) -> Result<i64, FakeError> {
        Ok(self.read(|state| {
            let held = state
                .oauth
                .grants
                .iter()
                .filter(|found| found.grant.person == person && found.grant.works_at(now))
                .count();
            i64::try_from(held).unwrap_or(i64::MAX)
        }))
    }

    async fn oauth_token(&self, hash: &Digest) -> Result<Option<OAuthToken>, FakeError> {
        Ok(self.read(|state| {
            state
                .oauth
                .tokens
                .iter()
                .find(|token| token.secret_hash == *hash)
                .cloned()
        }))
    }

    async fn oauth_code(&self, hash: &Digest) -> Result<Option<OAuthCode>, FakeError> {
        Ok(self.read(|state| {
            state
                .oauth
                .codes
                .iter()
                .find(|code| code.secret_hash == *hash)
                .cloned()
        }))
    }

    async fn oauth_web_addresses(&self) -> Result<Vec<OAuthWebAddress>, FakeError> {
        Ok(self.read(|state| {
            let mut found = state.oauth.web_addresses.clone();
            found.sort_by(|a, b| a.address.cmp(&b.address));
            found
        }))
    }

    async fn touch_grant(&self, id: GrantId, at: Timestamp) -> Result<(), FakeError> {
        let mut shared = self.lock();
        if let Some(found) = shared
            .state
            .oauth
            .grants
            .iter_mut()
            .find(|found| found.grant.id == id)
        {
            found.grant.last_used_at = Some(at);
        }
        Ok(())
    }

    async fn insert_oauth_client(tx: &mut Tx, client: &OAuthClient) -> Result<(), FakeError> {
        tx.state.oauth.clients.push(client.clone());
        Ok(())
    }

    async fn insert_grant(tx: &mut Tx, record: &GrantRecord) -> Result<(), FakeError> {
        tx.state.oauth.grants.push(record.clone());
        Ok(())
    }

    async fn insert_oauth_code(tx: &mut Tx, code: &OAuthCode) -> Result<(), FakeError> {
        tx.state.oauth.codes.push(code.clone());
        Ok(())
    }

    async fn use_oauth_code(tx: &mut Tx, hash: &Digest, at: Timestamp) -> Result<bool, FakeError> {
        let code = tx
            .state
            .oauth
            .codes
            .iter_mut()
            .find(|code| code.secret_hash == *hash && code.used_at.is_none());
        Ok(code.map(|code| code.used_at = Some(at)).is_some())
    }

    async fn insert_oauth_token(tx: &mut Tx, token: &OAuthToken) -> Result<(), FakeError> {
        tx.state.oauth.tokens.push(token.clone());
        Ok(())
    }

    async fn use_oauth_token(tx: &mut Tx, hash: &Digest, at: Timestamp) -> Result<bool, FakeError> {
        let token = tx
            .state
            .oauth
            .tokens
            .iter_mut()
            .find(|token| token.secret_hash == *hash && token.used_at.is_none());
        Ok(token.map(|token| token.used_at = Some(at)).is_some())
    }

    async fn end_grant(
        tx: &mut Tx,
        id: GrantId,
        how: GrantEnd,
        at: Timestamp,
    ) -> Result<bool, FakeError> {
        Ok(end(&mut tx.state, id, how, at))
    }

    async fn end_grants_of(
        tx: &mut Tx,
        person: UserId,
        how: GrantEnd,
        at: Timestamp,
    ) -> Result<Vec<GrantId>, FakeError> {
        let ids: Vec<GrantId> = tx
            .state
            .oauth
            .grants
            .iter()
            .filter(|found| found.grant.person == person && found.grant.ended_at.is_none())
            .map(|found| found.grant.id)
            .collect();
        for id in &ids {
            end(&mut tx.state, *id, how, at);
        }
        Ok(ids)
    }

    async fn insert_oauth_web_address(
        tx: &mut Tx,
        address: &OAuthWebAddress,
    ) -> Result<bool, FakeError> {
        if tx
            .state
            .oauth
            .web_addresses
            .iter()
            .any(|held| held.address == address.address)
        {
            return Ok(false);
        }
        tx.state.oauth.web_addresses.push(address.clone());
        Ok(true)
    }

    async fn delete_oauth_web_address(tx: &mut Tx, address: &str) -> Result<bool, FakeError> {
        let before = tx.state.oauth.web_addresses.len();
        tx.state
            .oauth
            .web_addresses
            .retain(|held| held.address != address);
        Ok(tx.state.oauth.web_addresses.len() < before)
    }

    async fn pause_unused_tokens(tx: &mut Tx, now: Timestamp) -> Result<u64, FakeError> {
        let mut moved = 0;
        for token in &mut tx.state.oauth.tokens {
            if token.kind == TokenKind::Refresh && token.used_at.is_none() && token.expires_at > now
            {
                let waited = now.duration_since(token.paused_at.unwrap_or(now));
                token.expires_at = token
                    .expires_at
                    .checked_add(waited)
                    .unwrap_or(token.expires_at);
                token.paused_at = Some(now);
                moved += 1;
            }
        }
        Ok(moved)
    }

    async fn clean_oauth(
        tx: &mut Tx,
        now: Timestamp,
        ended_before: Timestamp,
        registered_before: Timestamp,
    ) -> Result<OAuthCleaned, FakeError> {
        let count = |before: usize, after: usize| u64::try_from(before - after).unwrap_or(0);
        let oauth = &mut tx.state.oauth;
        for token in &mut oauth.tokens {
            token.paused_at = None;
        }
        let secrets = oauth.codes.len() + oauth.tokens.len();
        oauth.codes.retain(|code| code.expires_at > now);
        oauth.tokens.retain(|token| token.expires_at > now);
        let secrets = count(secrets, oauth.codes.len() + oauth.tokens.len());
        let grants = oauth.grants.len();
        oauth.grants.retain(|found| {
            found
                .grant
                .ended_at
                .is_none_or(|ended| ended > ended_before)
                && found.grant.ends_at > ended_before
        });
        let grants = count(grants, oauth.grants.len());
        let clients = oauth.clients.len();
        let held: Vec<OAuthClientId> = oauth
            .grants
            .iter()
            .map(|found| found.grant.client)
            .collect();
        oauth
            .clients
            .retain(|client| client.registered_at > registered_before || held.contains(&client.id));
        Ok(OAuthCleaned {
            secrets,
            grants,
            clients: count(clients, oauth.clients.len()),
        })
    }
}
