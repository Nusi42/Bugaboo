//! The sign-in part of the in-memory store: people, sessions, the second step, invitations and
//! access keys (the errors of a change and the settings of signing in are in `fake_auth_store.rs`).

use bb_core::id::{AccessKeyId, InvitationId, PasskeyId, RoleId, SessionId, UserId};
use bb_core::person::Person;
use bb_core::security_log::Digest;
use bb_core::settings::{SECURITY_TWO_STEP_REQUIRED, Scope, SettingValue};
use bb_core::sign_in::{
    Invitation, SessionStage, SetupCode, SignInSession, StoredPasskey, TotpStep, TwoStep,
};
use jiff::Timestamp;
use zeroize::Zeroizing;

use crate::auth::{
    AccessKeyRecord, AccessKeyStore, Account, AccountStore, InvitationStore, PeoplePosition,
    ProjectChoice, SearchIn, SecondStepStore, SessionRecord, SessionStore,
};

use super::fake::*;

impl AccountStore for FakeStore {
    async fn account_by_email(&self, email: &str) -> Result<Option<Account>, FakeError> {
        Ok(self.read(|state| {
            state
                .people
                .iter()
                .find(|(person, _)| person.email == email)
                .map(|(person, hash)| Account {
                    person: person.clone(),
                    password_hash: hash.clone(),
                })
        }))
    }

    async fn person(&self, id: UserId) -> Result<Option<Person>, FakeError> {
        Ok(self.people_of(&[id]).await?.pop())
    }

    async fn people_of(&self, ids: &[UserId]) -> Result<Vec<Person>, FakeError> {
        let all = self.read(|state| state.people.clone());
        Ok(all
            .into_iter()
            .map(|(person, _)| person)
            .filter(|person| ids.contains(&person.id))
            .collect())
    }

    async fn people_page(
        &self,
        search: Option<&str>,
        look_at: SearchIn,
        after: Option<&PeoplePosition>,
        limit: u64,
    ) -> Result<Vec<Person>, FakeError> {
        let key = |name: &str, id: UserId| (bb_core::names::name_key(name), id);
        Ok(self.read(|state| {
            let mut people: Vec<Person> = state
                .people
                .iter()
                .map(|(person, _)| person.clone())
                .filter(|person| super::fake_people::holds(person, search, look_at))
                .collect();
            people.sort_by_key(|person| key(&person.display_name, person.id));
            people
                .into_iter()
                .filter(|person| {
                    after.is_none_or(|position| {
                        key(&person.display_name, person.id) > key(&position.name, position.id)
                    })
                })
                .take(usize::try_from(limit).unwrap_or(usize::MAX))
                .collect()
        }))
    }

    async fn insert_person(
        tx: &mut Tx,
        person: &Person,
        password_hash: &str,
        _at: Timestamp,
    ) -> Result<(), FakeError> {
        if tx
            .state
            .people
            .iter()
            .any(|(known, _)| known.email == person.email)
        {
            return Err(FakeError::Duplicate);
        }
        tx.state
            .people
            .push((person.clone(), Some(password_hash.to_owned())));
        Ok(())
    }

    async fn set_password(
        tx: &mut Tx,
        person: UserId,
        password_hash: &str,
        _at: Timestamp,
    ) -> Result<(), FakeError> {
        for (known, hash) in &mut tx.state.people {
            if known.id == person {
                *hash = Some(password_hash.to_owned());
            }
        }
        Ok(())
    }
}

impl SessionStore for FakeStore {
    async fn session_by_hash(&self, hash: &Digest) -> Result<Option<SessionRecord>, FakeError> {
        Ok(self.read(|state| {
            let session = state
                .sessions
                .iter()
                .find(|session| session.token_hash == *hash)?;
            let person = state
                .people
                .iter()
                .find(|(person, _)| person.id == session.person)?;
            Some(SessionRecord {
                session: session.clone(),
                person: person.0.clone(),
            })
        }))
    }

    async fn sessions_of(&self, person: UserId) -> Result<Vec<SignInSession>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<SignInSession> = state
                .sessions
                .iter()
                .filter(|session| session.person == person)
                .cloned()
                .collect();
            found.sort_by_key(|row| std::cmp::Reverse(row.created_at));
            found
        }))
    }

    async fn touch_session(&self, id: SessionId, at: Timestamp) -> Result<(), FakeError> {
        let mut shared = self.lock();
        if let Some(session) = shared
            .state
            .sessions
            .iter_mut()
            .find(|session| session.id == id)
        {
            session.last_seen_at = at;
        }
        Ok(())
    }

    async fn insert_session(tx: &mut Tx, session: &SignInSession) -> Result<(), FakeError> {
        tx.state.sessions.push(session.clone());
        Ok(())
    }

    async fn delete_session(tx: &mut Tx, id: SessionId) -> Result<bool, FakeError> {
        let before = tx.state.sessions.len();
        tx.state.sessions.retain(|session| session.id != id);
        Ok(tx.state.sessions.len() < before)
    }

    async fn delete_sessions_of(
        tx: &mut Tx,
        person: UserId,
        except: Option<SessionId>,
    ) -> Result<u64, FakeError> {
        let before = tx.state.sessions.len();
        tx.state
            .sessions
            .retain(|session| session.person != person || Some(session.id) == except);
        Ok(u64::try_from(before - tx.state.sessions.len()).unwrap_or(0))
    }

    async fn purge_sessions(
        tx: &mut Tx,
        idle_before: Timestamp,
        absolute_before: Timestamp,
        pending_before: Timestamp,
    ) -> Result<u64, FakeError> {
        let before = tx.state.sessions.len();
        tx.state.sessions.retain(|session| {
            let stale = match session.stage {
                SessionStage::Active => {
                    session.last_seen_at < idle_before || session.created_at < absolute_before
                }
                SessionStage::SecondStepPending => session.created_at < pending_before,
            };
            !stale
        });
        Ok(u64::try_from(before - tx.state.sessions.len()).unwrap_or(0))
    }
}

impl SecondStepStore for FakeStore {
    async fn two_step(&self, person: UserId) -> Result<Option<TwoStep>, FakeError> {
        Ok(self.read(|state| {
            state
                .two_steps
                .iter()
                .find(|step| step.person == person)
                .cloned()
        }))
    }

    async fn totp_secret(&self, person: UserId) -> Result<Option<Zeroizing<Vec<u8>>>, FakeError> {
        Ok(self.read(|state| {
            state
                .totp_secrets
                .iter()
                .find(|(owner, _)| *owner == person)
                .map(|(_, secret)| Zeroizing::new(secret.clone()))
        }))
    }

    async fn backup_codes_left(&self, person: UserId) -> Result<i64, FakeError> {
        Ok(self.read(|state| {
            i64::try_from(
                state
                    .backup_codes
                    .iter()
                    .filter(|(owner, _, used)| *owner == person && used.is_none())
                    .count(),
            )
            .unwrap_or(i64::MAX)
        }))
    }

    async fn passkeys_of(&self, person: UserId) -> Result<Vec<StoredPasskey>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<StoredPasskey> = state
                .passkeys
                .iter()
                .filter(|passkey| passkey.person == person)
                .cloned()
                .collect();
            found.sort_by_key(|passkey| passkey.created_at);
            found
        }))
    }

    async fn second_step_required(&self, person: UserId) -> Result<bool, FakeError> {
        Ok(self.read(|state| {
            let everybody = state.settings.iter().any(|row| {
                row.scope == Scope::Organization
                    && row.key == SECURITY_TWO_STEP_REQUIRED.definition().key()
                    && row.value == SettingValue::Bool(true)
            });
            let groups: Vec<_> = state
                .members
                .iter()
                .filter(|member| member.person == person)
                .map(|member| member.group)
                .collect();
            everybody
                || state.assignments.iter().any(|row| {
                    let mine = row.person == Some(person)
                        || row.group.is_some_and(|group| groups.contains(&group));
                    mine && row.role.is_some_and(|role| {
                        state
                            .roles
                            .iter()
                            .any(|record| record.role.id == role && record.role.two_step)
                    })
                })
        }))
    }

    async fn has_second_step(&self, person: UserId) -> Result<bool, FakeError> {
        Ok(self.read(|state| {
            state
                .two_steps
                .iter()
                .any(|step| step.person == person && step.confirmed)
                || state
                    .passkeys
                    .iter()
                    .any(|passkey| passkey.person == person)
        }))
    }

    async fn people_without_second_step(&self, role: RoleId) -> Result<Vec<UserId>, FakeError> {
        Ok(self.read(|state| {
            let mut holders: Vec<UserId> = Vec::new();
            for row in state
                .assignments
                .iter()
                .filter(|row| row.role == Some(role))
            {
                holders.extend(row.person);
                if let Some(group) = row.group {
                    holders.extend(
                        state
                            .members
                            .iter()
                            .filter(|member| member.group == group)
                            .map(|member| member.person),
                    );
                }
            }
            holders.sort();
            holders.dedup();
            holders
                .into_iter()
                .filter(|person| {
                    !state
                        .two_steps
                        .iter()
                        .any(|step| step.person == *person && step.confirmed)
                        && !state
                            .passkeys
                            .iter()
                            .any(|passkey| passkey.person == *person)
                })
                .collect()
        }))
    }

    async fn put_totp_secret(
        &self,
        person: UserId,
        secret: &[u8],
        _at: Timestamp,
    ) -> Result<(), FakeError> {
        let mut shared = self.lock();
        shared
            .state
            .totp_secrets
            .retain(|(owner, _)| *owner != person);
        shared.state.totp_secrets.push((person, secret.to_vec()));
        Ok(())
    }

    async fn delete_totp_secret(&self, person: UserId) -> Result<(), FakeError> {
        self.lock()
            .state
            .totp_secrets
            .retain(|(owner, _)| *owner != person);
        Ok(())
    }

    async fn upsert_two_step(tx: &mut Tx, step: &TwoStep) -> Result<(), FakeError> {
        tx.state
            .two_steps
            .retain(|known| known.person != step.person);
        tx.state.two_steps.push(step.clone());
        Ok(())
    }

    async fn confirm_two_step(tx: &mut Tx, person: UserId) -> Result<(), FakeError> {
        for step in &mut tx.state.two_steps {
            if step.person == person {
                step.confirmed = true;
            }
        }
        Ok(())
    }

    async fn delete_two_step(tx: &mut Tx, person: UserId) -> Result<(), FakeError> {
        tx.state.two_steps.retain(|step| step.person != person);
        tx.state
            .backup_codes
            .retain(|(owner, _, _)| *owner != person);
        Ok(())
    }

    async fn accept_totp_step(
        tx: &mut Tx,
        person: UserId,
        step: TotpStep,
    ) -> Result<bool, FakeError> {
        for known in &mut tx.state.two_steps {
            if known.person == person && known.last_step < step {
                known.last_step = step;
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn replace_backup_codes(
        tx: &mut Tx,
        person: UserId,
        hashes: &[Digest],
    ) -> Result<(), FakeError> {
        tx.state
            .backup_codes
            .retain(|(owner, _, _)| *owner != person);
        for hash in hashes {
            tx.state.backup_codes.push((person, *hash, None));
        }
        Ok(())
    }

    async fn use_backup_code(
        tx: &mut Tx,
        person: UserId,
        hash: &Digest,
        at: Timestamp,
    ) -> Result<bool, FakeError> {
        for (owner, known, used) in &mut tx.state.backup_codes {
            if *owner == person && known == hash && used.is_none() {
                *used = Some(at);
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn insert_passkey(tx: &mut Tx, passkey: &StoredPasskey) -> Result<(), FakeError> {
        tx.state.passkeys.push(passkey.clone());
        Ok(())
    }

    async fn use_passkey(
        tx: &mut Tx,
        id: PasskeyId,
        data: Option<&str>,
        at: Timestamp,
    ) -> Result<(), FakeError> {
        for passkey in &mut tx.state.passkeys {
            if passkey.id == id {
                passkey.last_used_at = Some(at);
                if let Some(data) = data {
                    passkey.data = data.to_owned();
                }
            }
        }
        Ok(())
    }

    async fn delete_passkey(tx: &mut Tx, person: UserId, id: PasskeyId) -> Result<bool, FakeError> {
        let before = tx.state.passkeys.len();
        tx.state
            .passkeys
            .retain(|passkey| !(passkey.id == id && passkey.person == person));
        Ok(tx.state.passkeys.len() < before)
    }
}

impl InvitationStore for FakeStore {
    async fn setup_code(&self) -> Result<Option<SetupCode>, FakeError> {
        Ok(self.read(|state| state.setup_code.clone()))
    }

    async fn invitation_by_hash(&self, hash: &Digest) -> Result<Option<Invitation>, FakeError> {
        let made = self.read(|state| {
            state
                .invitations
                .iter()
                .find(|invitation| invitation.token_hash == *hash)
                .cloned()
        });
        Ok(made.or_else(|| self.sign_up_link_by_hash(hash)))
    }

    async fn open_invitations(
        &self,
        now: Timestamp,
        after: Option<(Timestamp, InvitationId)>,
        limit: u64,
    ) -> Result<Vec<Invitation>, FakeError> {
        Ok(self.read(|state| {
            let mut open: Vec<Invitation> = state
                .invitations
                .iter()
                .filter(|invitation| invitation.used_at.is_none() && invitation.expires_at > now)
                .cloned()
                .collect();
            open.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.id.cmp(&b.id)));
            let start = after.map_or(0, |(created, id)| {
                open.iter()
                    .position(|invitation| (invitation.created_at, invitation.id) == (created, id))
                    .map_or(0, |at| at + 1)
            });
            open.into_iter()
                .skip(start)
                .take(usize::try_from(limit).unwrap_or(usize::MAX))
                .collect()
        }))
    }

    async fn insert_invitation(tx: &mut Tx, invitation: &Invitation) -> Result<(), FakeError> {
        tx.state.invitations.push(invitation.clone());
        Ok(())
    }

    async fn delete_invitation(tx: &mut Tx, id: InvitationId) -> Result<bool, FakeError> {
        let before = tx.state.invitations.len();
        tx.state
            .invitations
            .retain(|invitation| invitation.id != id);
        Ok(tx.state.invitations.len() < before)
    }

    async fn use_invitation(
        tx: &mut Tx,
        id: InvitationId,
        now: Timestamp,
    ) -> Result<bool, FakeError> {
        for invitation in &mut tx.state.invitations {
            if invitation.id == id && invitation.used_at.is_none() && invitation.expires_at > now {
                invitation.used_at = Some(now);
                return Ok(true);
            }
        }
        for link in &mut tx.state.sign_up_invitations {
            if link.id == id && link.used_at.is_none() && link.expires_at > now {
                link.used_at = Some(now);
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn put_setup_code(tx: &mut Tx, code: &SetupCode) -> Result<(), FakeError> {
        tx.state.setup_code = Some(code.clone());
        Ok(())
    }

    async fn take_setup_code(tx: &mut Tx, hash: &Digest) -> Result<bool, FakeError> {
        if tx
            .state
            .setup_code
            .as_ref()
            .is_some_and(|code| code.code_hash == *hash)
        {
            tx.state.setup_code = None;
            return Ok(true);
        }
        Ok(false)
    }

    async fn clear_setup_code(tx: &mut Tx) -> Result<(), FakeError> {
        tx.state.setup_code = None;
        Ok(())
    }
}

impl AccessKeyStore for FakeStore {
    async fn access_key_by_hash(
        &self,
        hash: &Digest,
    ) -> Result<Option<AccessKeyRecord>, FakeError> {
        Ok(self.read(|state| {
            state
                .access_keys
                .iter()
                .find(|record| record.key.secret_hash == *hash)
                .cloned()
        }))
    }

    async fn access_keys_of(
        &self,
        owner: UserId,
        after: Option<(Timestamp, AccessKeyId)>,
        limit: u64,
    ) -> Result<Vec<AccessKeyRecord>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<AccessKeyRecord> = state
                .access_keys
                .iter()
                .filter(|record| record.key.owner == owner)
                .cloned()
                .collect();
            found.sort_by(|a, b| {
                b.key
                    .created_at
                    .cmp(&a.key.created_at)
                    .then(a.key.id.cmp(&b.key.id))
            });
            found
                .into_iter()
                .filter(|record| {
                    after.is_none_or(|(created, id)| {
                        record.key.created_at < created
                            || (record.key.created_at == created && record.key.id > id)
                    })
                })
                .take(usize::try_from(limit).unwrap_or(usize::MAX))
                .collect()
        }))
    }

    async fn access_key_count(&self, owner: UserId) -> Result<i64, FakeError> {
        Ok(self.read(|state| {
            let held = state
                .access_keys
                .iter()
                .filter(|record| record.key.owner == owner)
                .count();
            i64::try_from(held).unwrap_or(i64::MAX)
        }))
    }

    async fn projects_after(
        &self,
        after: Option<&str>,
        skip: u64,
        limit: u64,
    ) -> Result<Vec<ProjectChoice>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<ProjectChoice> = state
                .projects
                .iter()
                .map(|project| ProjectChoice {
                    id: project.id,
                    short_name: project.short_name.to_string(),
                    name: project.name.clone(),
                })
                .filter(|project| after.is_none_or(|after| project.short_name.as_str() > after))
                .collect();
            found.sort_by(|a, b| a.short_name.cmp(&b.short_name));
            found
                .into_iter()
                .skip(usize::try_from(skip).unwrap_or(usize::MAX))
                .take(usize::try_from(limit).unwrap_or(usize::MAX))
                .collect()
        }))
    }

    async fn touch_access_key(&self, id: AccessKeyId, at: Timestamp) -> Result<(), FakeError> {
        let mut shared = self.lock();
        if let Some(record) = shared
            .state
            .access_keys
            .iter_mut()
            .find(|record| record.key.id == id)
            && record.key.last_used_at.is_none_or(|last| last < at)
        {
            record.key.last_used_at = Some(at);
        }
        Ok(())
    }

    async fn insert_access_key(tx: &mut Tx, record: &AccessKeyRecord) -> Result<(), FakeError> {
        // A test can make the store say that the public prefix of the next key is taken.
        if CLASH_ONCE.replace(false) {
            return Err(FakeError::Duplicate);
        }
        let used = tx.state.access_keys.iter().any(|other| {
            other.key.prefix == record.key.prefix || other.key.secret_hash == record.key.secret_hash
        });
        if used {
            return Err(FakeError::Duplicate);
        }
        tx.state.access_keys.push(record.clone());
        Ok(())
    }

    async fn delete_access_key(
        tx: &mut Tx,
        owner: UserId,
        id: AccessKeyId,
    ) -> Result<bool, FakeError> {
        let before = tx.state.access_keys.len();
        tx.state
            .access_keys
            .retain(|record| !(record.key.id == id && record.key.owner == owner));
        Ok(tx.state.access_keys.len() < before)
    }

    async fn delete_access_keys_of(
        tx: &mut Tx,
        owner: UserId,
    ) -> Result<Vec<AccessKeyId>, FakeError> {
        let gone: Vec<AccessKeyId> = tx
            .state
            .access_keys
            .iter()
            .filter(|record| record.key.owner == owner)
            .map(|record| record.key.id)
            .collect();
        tx.state
            .access_keys
            .retain(|record| record.key.owner != owner);
        Ok(gone)
    }
}
