//! Signing in and out (AUTH-01, AUTH-03, SEC-12, SEC-14): password, second step, sessions.
//!
//! Rules that hold in every use case here:
//! - an unknown account and a wrong password get the same answer, after the same work;
//! - a session gets a new secret when the person signs in and when the second step is done, so
//!   a secret that was known earlier opens nothing;
//! - only failed tries count against the limit, and success clears it (M1-F27);
//! - every sign-in event goes to the security log.

use bb_auth::{EmailAddress, MAX_LENGTH, Purpose, SecretToken, TotpSecret, Verdict, code_hash};
use bb_core::ErrorCode;
use bb_core::id::UserId;
use bb_core::sign_in::{SecurityAction, SessionStage};
use jiff::Timestamp;
use serde_json::json;
use zeroize::Zeroizing;

use super::ctx::{AppError, Ctx, by, change_failed, changed, entity, internal, person_key, set_to};
use super::store::AuthStore;
use super::types::{
    AuthStatus, AuthenticateQuery, Authenticated, Done, EndSession, PasskeySecondStep, PersonView,
    SecondStep, SecondStepMethods, SessionView, SessionsQuery, SignIn, SignOut, SignOutEverywhere,
    Started, StatusQuery,
};

/// Long text in the key of a failure counter is cut here: the key only has to tell accounts
/// apart, and a megabyte of "address" must not use memory.
const KEY_CHARS: usize = 320;

/// What the app needs to know before anybody signs in.
///
/// # Errors
/// Fails when the store fails.
pub async fn status<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    _input: StatusQuery,
) -> Result<AuthStatus, AppError> {
    let administrators = ctx
        .store()
        .administrators()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let settings = ctx.settings().await?;
    Ok(AuthStatus {
        setup_needed: administrators == 0,
        sign_up_open: settings.sign_up_enabled,
        passkeys_available: ctx.services.passkeys.is_some(),
    })
}

/// Who is behind this session.
///
/// # Errors
/// `auth.not_signed_in` when the cookie is missing, unknown, not finished or run out.
pub async fn authenticate<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AuthenticateQuery,
) -> Result<Authenticated, AppError> {
    let record = ctx.active(&input.session_token).await?;
    Ok(Authenticated {
        person: PersonView::from(&record.person),
        session_id: record.session.id,
        started_at: record.session.created_at,
    })
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Which ways there are to finish signing in for a person, if they have any.
    pub(crate) async fn second_step_methods(
        &self,
        person: UserId,
    ) -> Result<Option<SecondStepMethods>, AppError> {
        let code_app = self
            .store()
            .two_step(person)
            .await
            .map_err(|error| internal::<S>(&error))?
            .is_some_and(|step| step.confirmed);
        let passkey = !self
            .store()
            .passkeys_of(person)
            .await
            .map_err(|error| internal::<S>(&error))?
            .is_empty();
        Ok((code_app || passkey).then_some(SecondStepMethods {
            code_app,
            passkey,
            backup_code: code_app,
        }))
    }
}

/// A moment `seconds` before `now`.
fn before(now: Timestamp, seconds: i64) -> Timestamp {
    Timestamp::from_second(now.as_second().saturating_sub(seconds)).unwrap_or(now)
}

/// Removes the sessions nobody can use any more, inside a change that is happening anyway.
pub(crate) async fn purge<S: AuthStore>(
    transaction: &mut S::Transaction,
    settings: &super::store::AuthSettings,
    now: Timestamp,
) -> Result<(), S::Error> {
    S::purge_sessions(
        transaction,
        before(now, settings.session_idle_minutes.saturating_mul(60)),
        before(now, settings.session_absolute_hours.saturating_mul(3600)),
        before(now, super::ctx::PENDING_SECONDS),
    )
    .await
    .map(|_| ())
}

/// Signs in with e-mail address and password. When the person has a second step, the session
/// that is made can do nothing but finish it.
///
/// # Errors
/// `auth.wrong_credentials` for an unknown address and a wrong password alike, 429 when the
/// account has used up its failed tries.
pub async fn sign_in<S: AuthStore>(ctx: &Ctx<'_, S>, input: SignIn) -> Result<Started, AppError> {
    let SignIn { email, password } = input;
    let key = format!(
        "account:{}",
        email
            .trim()
            .to_lowercase()
            .chars()
            .take(KEY_CHARS)
            .collect::<String>()
    );
    ctx.limit_open(&key)?;
    let account = match EmailAddress::parse(&email) {
        Some(address) => ctx
            .store()
            .account_by_email(address.as_str())
            .await
            .map_err(|error| internal::<S>(&error))?,
        None => None,
    };
    let password = Zeroizing::new(password);
    // Text that can never be a password is refused before any work is done; this tells nothing
    // about the account.
    let verdict = if password.chars().count() > MAX_LENGTH {
        Verdict {
            matches: false,
            needs_rehash: false,
        }
    } else {
        let stored = account
            .as_ref()
            .and_then(|account| account.password_hash.clone());
        ctx.services.hasher.verify(password.clone(), stored).await?
    };
    let Some(account) = account.filter(|_| verdict.matches) else {
        ctx.limit_failed(&key, None).await;
        ctx.note(SecurityAction::SignInFailed, None, None).await;
        return Err(ErrorCode::WrongCredentials.into());
    };
    ctx.services.limiter.succeeded(&key);

    let person = account.person;
    let methods = ctx.second_step_methods(person.id).await?;
    let stage = if methods.is_some() {
        SessionStage::SecondStepPending
    } else {
        SessionStage::Active
    };
    let new = ctx.new_session(person.id, stage)?;
    let settings = ctx.settings().await?;
    let rehash = if verdict.needs_rehash {
        Some(ctx.services.hasher.hash(password).await?)
    } else {
        None
    };
    let event = ctx.event(
        by(person.id),
        entity("session", new.session.id)?,
        vec![set_to("stage", json!(stage.code()))],
    )?;
    let (store, session, person_id, now) = (ctx.store(), new.session.clone(), person.id, ctx.now());
    let purge_settings = settings.clone();
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                purge::<S>(transaction, &purge_settings, now).await?;
                if let Some(hash) = &rehash {
                    S::set_password(transaction, person_id, hash, now).await?;
                }
                S::insert_session(transaction, &session).await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, None))?;
    let action = if methods.is_some() {
        SecurityAction::SignInSecondStepNeeded
    } else {
        SecurityAction::SignInSucceeded
    };
    ctx.note(
        action,
        Some(person.id),
        Some(entity("session", new.session.id)?),
    )
    .await;
    ctx.started(&new, &person, &settings, methods)
}

/// What a typed second-step code is: six digits are a code from the code app, anything else is
/// tried as a backup code.
fn is_app_code(typed: &str) -> bool {
    let digits: Vec<char> = typed.chars().filter(|c| !c.is_whitespace()).collect();
    digits.len() == 6 && digits.iter().all(char::is_ascii_digit)
}

/// Finishes signing in with a code from the code app or a backup code. The session of the
/// password step is replaced by a new one.
///
/// # Errors
/// `auth.wrong_second_step_code` when the code is wrong or was used already, 429 after too many
/// failed tries.
pub async fn second_step<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SecondStep,
) -> Result<Started, AppError> {
    let pending = ctx
        .session(&input.session_token, SessionStage::SecondStepPending)
        .await?;
    let person = pending.person;
    let key = person_key(person.id);
    ctx.limit_open(&key)?;
    let now = ctx.now();
    let wrong = || AppError::Refused(ErrorCode::WrongSecondStepCode);

    let two_step = ctx
        .store()
        .two_step(person.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .filter(|step| step.confirmed);
    let mut accepted_step = None;
    let mut backup = None;
    if let Some(two_step) = &two_step {
        if is_app_code(&input.code) {
            if let Some(secret) = ctx
                .store()
                .totp_secret(person.id)
                .await
                .map_err(|error| internal::<S>(&error))?
            {
                let secret = TotpSecret::from_bytes(&secret);
                accepted_step = secret.check(
                    &input
                        .code
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .collect::<String>(),
                    now,
                    two_step.last_step,
                )?;
            }
        } else {
            backup = code_hash(Purpose::BackupCode, &input.code);
        }
    }
    if accepted_step.is_none() && backup.is_none() {
        ctx.limit_failed(&key, Some(person.id)).await;
        ctx.note(SecurityAction::SecondStepFailed, Some(person.id), None)
            .await;
        return Err(wrong());
    }

    let new = ctx.new_session(person.id, SessionStage::Active)?;
    let settings = ctx.settings().await?;
    let event = ctx.event(
        by(person.id),
        entity("session", new.session.id)?,
        vec![set_to("stage", json!(SessionStage::Active.code()))],
    )?;
    let (store, session, pending_id) = (ctx.store(), new.session.clone(), pending.session.id);
    let person_id = person.id;
    let result = store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // The code is used up in the same transaction that opens the session, so two
                // requests with the same code cannot both succeed.
                let fresh = match (accepted_step, &backup) {
                    (Some(step), _) => S::accept_totp_step(transaction, person_id, step).await?,
                    (None, Some(hash)) => {
                        S::use_backup_code(transaction, person_id, hash, now).await?
                    }
                    (None, None) => false,
                };
                if !fresh {
                    return Err(S::aborted());
                }
                S::delete_session(transaction, pending_id).await?;
                S::insert_session(transaction, &session).await
            })
        })
        .await;
    if let Err(error) = result {
        let error = change_failed::<S>(&error, Some(ErrorCode::WrongSecondStepCode));
        if matches!(error, AppError::Refused(_)) {
            ctx.limit_failed(&key, Some(person.id)).await;
            ctx.note(SecurityAction::SecondStepFailed, Some(person.id), None)
                .await;
        }
        return Err(error);
    }
    ctx.services.limiter.succeeded(&key);
    if accepted_step.is_none() {
        ctx.note(SecurityAction::BackupCodeUsed, Some(person.id), None)
            .await;
    }
    ctx.note(
        SecurityAction::SecondStepSucceeded,
        Some(person.id),
        Some(entity("session", new.session.id)?),
    )
    .await;
    ctx.started(&new, &person, &settings, None)
}

/// Finishes signing in with a passkey after the password. The session of the password step is
/// replaced by a new one.
///
/// # Errors
/// `auth.passkey_rejected` when the answer of the device does not check out, 429 after too many
/// failed tries.
pub async fn passkey_second_step<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PasskeySecondStep,
) -> Result<Started, AppError> {
    let pending = ctx
        .session(&input.session_token, SessionStage::SecondStepPending)
        .await?;
    let person = pending.person;
    let key = person_key(person.id);
    ctx.limit_open(&key)?;
    let (stored, credentials) = ctx.credentials(person.id).await?;
    let rejected = || AppError::Refused(ErrorCode::PasskeyRejected);
    let check = match bb_auth::ChallengeId::parse(&input.challenge_id) {
        Some(id) => ctx.passkeys()?.finish_authentication(
            ctx.now(),
            id,
            person.id,
            &input.credential,
            &credentials,
        ),
        None => Err(bb_auth::AuthError::Challenge),
    };
    let check = match check {
        Ok(check) => check,
        Err(bb_auth::AuthError::Challenge | bb_auth::AuthError::Passkey) => {
            ctx.limit_failed(&key, Some(person.id)).await;
            ctx.note(SecurityAction::PasskeySignInFailed, Some(person.id), None)
                .await;
            return Err(rejected());
        }
        Err(error) => return Err(error.into()),
    };
    let used = stored
        .iter()
        .find(|passkey| passkey.credential_id == check.credential_id)
        .ok_or_else(rejected)?;
    let (used_id, data) = (used.id, check.updated.map(|updated| updated.data));

    let new = ctx.new_session(person.id, SessionStage::Active)?;
    let settings = ctx.settings().await?;
    let event = ctx.event(
        by(person.id),
        entity("session", new.session.id)?,
        vec![set_to("stage", json!(SessionStage::Active.code()))],
    )?;
    let (store, session, pending_id, now) = (
        ctx.store(),
        new.session.clone(),
        pending.session.id,
        ctx.now(),
    );
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::use_passkey(transaction, used_id, data.as_deref(), now).await?;
                S::delete_session(transaction, pending_id).await?;
                S::insert_session(transaction, &session).await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, None))?;
    ctx.services.limiter.succeeded(&key);
    ctx.note(
        SecurityAction::SecondStepSucceeded,
        Some(person.id),
        Some(entity("session", new.session.id)?),
    )
    .await;
    ctx.started(&new, &person, &settings, None)
}

/// Ends the session of the request. A cookie that opens nothing is fine: the person is signed
/// out either way.
///
/// # Errors
/// Fails when the store fails.
pub async fn sign_out<S: AuthStore>(ctx: &Ctx<'_, S>, input: SignOut) -> Result<Done, AppError> {
    let Some(token) = SecretToken::parse(&input.session_token) else {
        return Ok(Done);
    };
    let Some(record) = ctx
        .store()
        .session_by_hash(&token.hash(Purpose::Session))
        .await
        .map_err(|error| internal::<S>(&error))?
    else {
        return Ok(Done);
    };
    let (person, id) = (record.person.id, record.session.id);
    let event = ctx.event(by(person), entity("session", id)?, vec![changed("session")])?;
    let store = ctx.store();
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::delete_session(transaction, id).await.map(|_| ()) })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, None))?;
    ctx.note(
        SecurityAction::SessionEnded,
        Some(person),
        Some(entity("session", id)?),
    )
    .await;
    Ok(Done)
}

/// Ends every session of the person, the one of the request included.
///
/// # Errors
/// `auth.not_signed_in` without a session.
pub async fn sign_out_everywhere<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SignOutEverywhere,
) -> Result<Done, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let event = ctx.event(
        by(person),
        entity("person", person)?,
        vec![changed("sessions")],
    )?;
    let store = ctx.store();
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::delete_sessions_of(transaction, person, None)
                    .await
                    .map(|_| ())
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, None))?;
    ctx.note(SecurityAction::SessionsEndedAll, Some(person), None)
        .await;
    Ok(Done)
}

/// Ends one of the sessions of the signed-in person.
///
/// # Errors
/// `auth.not_signed_in` without a session; `resource.not_found` for a session that is not the
/// person's (it does not say whether it exists).
pub async fn end_session<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: EndSession,
) -> Result<Done, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let id = input.session_id;
    let owned = ctx
        .store()
        .sessions_of(person)
        .await
        .map_err(|error| internal::<S>(&error))?
        .iter()
        .any(|session| session.id == id);
    if !owned {
        return Err(ErrorCode::NotFound.into());
    }
    let event = ctx.event(by(person), entity("session", id)?, vec![changed("session")])?;
    let store = ctx.store();
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::delete_session(transaction, id).await.map(|_| ()) })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, None))?;
    ctx.note(
        SecurityAction::SessionEnded,
        Some(person),
        Some(entity("session", id)?),
    )
    .await;
    Ok(Done)
}

/// The sessions of the signed-in person, newest first, with the one of the request marked.
///
/// # Errors
/// `auth.not_signed_in` without a session.
pub async fn sessions<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SessionsQuery,
) -> Result<Vec<SessionView>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let settings = ctx.settings().await?;
    let now = ctx.now().as_second();
    let all = ctx
        .store()
        .sessions_of(record.person.id)
        .await
        .map_err(|error| internal::<S>(&error))?;
    Ok(all
        .into_iter()
        .filter(|session| {
            session.stage == SessionStage::Active
                && now.saturating_sub(session.created_at.as_second())
                    <= settings.session_absolute_hours.saturating_mul(3600)
                && now.saturating_sub(session.last_seen_at.as_second())
                    <= settings.session_idle_minutes.saturating_mul(60)
        })
        .map(|session| {
            let device = bb_auth::Device::from_stored(&session.device);
            SessionView {
                current: session.id == record.session.id,
                id: session.id,
                started_at: session.created_at,
                last_seen_at: session.last_seen_at,
                network: session.network.map(|network| network.to_string()),
                browser: device.browser.map(str::to_owned),
                system: device.system.map(str::to_owned),
            }
        })
        .collect())
}
