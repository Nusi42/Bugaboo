//! The second step and the passkeys (AUTH-02, AUTH-03): setting up and switching off the code
//! app, backup codes, adding and removing passkeys, and signing in with a passkey alone.

use bb_auth::{ChallengeId, HumanCode, Purpose, TotpSecret};
use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::id::PasskeyId;
use bb_core::sign_in::{SecurityAction, SessionStage, StoredPasskey, TotpStep, TwoStep};
use bb_core::text::LABEL_MAX;
use serde_json::json;

use super::ctx::Ctx;
use super::error::{AppError, Stops, change_failed, internal};
use super::events::{by, changed, entity, set_to};
use super::store::AuthStore;
use super::types::{
    AddPasskey, BackupCodes, CodeAppSetup, ConfirmCodeApp, Done, MAX_PASSKEYS_LISTED,
    PasskeyChallenge, PasskeyRegistrationQuery, PasskeySecondStepQuery, PasskeySignIn,
    PasskeySignInQuery, PasskeyView, RemoveCodeApp, RemovePasskey, RenewBackupCodes, StartCodeApp,
    Started, TwoStepQuery, TwoStepStatus,
};
use crate::input::clean_text;

/// How many backup codes a person gets.
pub const BACKUP_CODE_COUNT: usize = 10;
/// How many passkeys one person can have.
pub const MAX_PASSKEYS: usize = 20;

fn view(passkey: &StoredPasskey) -> PasskeyView {
    PasskeyView {
        id: passkey.id,
        label: passkey.label.clone(),
        created_at: passkey.created_at,
        last_used_at: passkey.last_used_at,
    }
}

/// How the second step of the signed-in person is set up.
///
/// # Errors
/// `auth.not-signed-in` without a session.
pub async fn two_step<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: TwoStepQuery,
) -> Result<TwoStepStatus, AppError> {
    let person = ctx.active_to_set_up(&input.session_token).await?.person.id;
    let code_app = ctx
        .store()
        .two_step(person)
        .await
        .map_err(|error| internal::<S>(&error))?
        .is_some_and(|step| step.confirmed);
    let backup_codes_left = ctx
        .store()
        .backup_codes_left(person)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let passkeys = ctx
        .store()
        .passkeys_of(person)
        .await
        .map_err(|error| internal::<S>(&error))?;
    Ok(TwoStepStatus {
        code_app,
        backup_codes_left,
        passkeys: passkeys
            .iter()
            .take(MAX_PASSKEYS_LISTED)
            .map(view)
            .collect(),
        truncated: passkeys.len() > MAX_PASSKEYS_LISTED,
    })
}

/// A challenge is only as good as the ceremony behind it; a full table of waiting challenges
/// means too many tries, everything else is a problem of the server.
fn challenge_failed(error: bb_auth::AuthError) -> AppError {
    match error {
        bb_auth::AuthError::TooManyChallenges => AppError::Limited { retry_after: 60 },
        bb_auth::AuthError::Passkey | bb_auth::AuthError::Challenge => {
            AppError::Refused(ErrorCode::PasskeyRejected)
        }
        other => other.into(),
    }
}

/// Starts checking a passkey as the second step of signing in.
///
/// # Errors
/// `auth.not-signed-in` without a half finished sign-in; `auth.passkey-rejected` when the person
/// has no passkey.
pub async fn passkey_second_step_options<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PasskeySecondStepQuery,
) -> Result<PasskeyChallenge, AppError> {
    let pending = ctx
        .session(&input.session_token, SessionStage::SecondStepPending)
        .await?;
    let (_, credentials) = ctx.credentials(pending.person.id).await?;
    if credentials.is_empty() {
        return Err(ErrorCode::PasskeyRejected.into());
    }
    let (id, options) = ctx
        .passkeys()?
        .start_authentication(
            ctx.now(),
            ctx.network_name().as_deref(),
            pending.person.id,
            &credentials,
        )
        .map_err(challenge_failed)?;
    Ok(PasskeyChallenge {
        challenge_id: id.encode(),
        options,
    })
}

/// Starts signing in with a passkey alone: the device offers its passkeys for this server, so
/// nothing is typed and nothing says whether an address has an account.
///
/// # Errors
/// 429 when too many challenges are waiting.
pub async fn passkey_sign_in_options<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    _input: PasskeySignInQuery,
) -> Result<PasskeyChallenge, AppError> {
    let (id, options) = ctx
        .passkeys()?
        .start_discoverable(ctx.now(), ctx.network_name().as_deref())
        .map_err(challenge_failed)?;
    Ok(PasskeyChallenge {
        challenge_id: id.encode(),
        options,
    })
}

/// Signs in with a passkey alone. A passkey counts as both steps, so the session is active at
/// once.
///
/// # Errors
/// `auth.passkey-rejected` for anything that does not check out, without saying why.
pub async fn passkey_sign_in<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PasskeySignIn,
) -> Result<Started, AppError> {
    let rejected = || AppError::Refused(ErrorCode::PasskeyRejected);
    let outcome = async {
        let id = ChallengeId::parse(&input.challenge_id).ok_or_else(rejected)?;
        let named = ctx
            .passkeys()?
            .identify(&input.credential)
            .map_err(challenge_failed)?;
        Ok::<_, AppError>((id, named))
    }
    .await;
    let (challenge, named) = match outcome {
        Ok(found) => found,
        // Only what does not check out is written down and answered alike; a failure of the
        // server is not turned into a rejection.
        Err(AppError::Refused(ErrorCode::PasskeyRejected)) => {
            ctx.note(SecurityAction::PasskeySignInFailed, None, None)
                .await;
            return Err(rejected());
        }
        Err(error) => return Err(error),
    };
    let (stored, credentials) = ctx.credentials(named.person).await?;
    let check =
        ctx.passkeys()?
            .finish_discoverable(ctx.now(), challenge, &input.credential, &credentials);
    let used = match check {
        Ok(check) => stored
            .iter()
            .find(|passkey| passkey.credential_id == check.credential_id)
            .map(|passkey| (passkey.id, check.updated.map(|updated| updated.data))),
        Err(bb_auth::AuthError::Challenge | bb_auth::AuthError::Passkey) => None,
        Err(error) => return Err(error.into()),
    };
    let (Some((used_id, data)), Some(person)) = (
        used,
        ctx.store()
            .person(named.person)
            .await
            .map_err(|error| internal::<S>(&error))?,
    ) else {
        ctx.note(SecurityAction::PasskeySignInFailed, None, None)
            .await;
        return Err(rejected());
    };

    let new = ctx.new_session(person.id, SessionStage::Active)?;
    let settings = ctx.settings().await?;
    let old = ctx
        .previous_session(input.previous_session.as_deref())
        .await?;
    let event = ctx.event(
        by(person.id),
        entity(RecordKind::Session, new.session.id),
        vec![set_to("stage", json!(SessionStage::Active.code()))],
    )?;
    let (store, session, now) = (ctx.store(), new.session.clone(), ctx.now());
    let purge_settings = settings.clone();
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::use_passkey(transaction, used_id, data.as_deref(), now).await?;
                super::session::purge::<S>(transaction, &purge_settings, now).await?;
                if let Some(old) = old {
                    S::delete_session(transaction, old).await?;
                }
                S::insert_session(transaction, &session).await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(
        SecurityAction::PasskeySignInSucceeded,
        Some(person.id),
        Some(entity(RecordKind::Session, new.session.id)),
    )
    .await;
    ctx.started(&new, &person, &settings, None)
}

/// Starts setting up the code app: makes the shared secret, keeps it (encrypted) and returns
/// what the setup screen shows. The code app is not asked for until the first code is confirmed.
///
/// # Errors
/// `request.invalid` when a code app is set up already (switch it off first).
pub async fn start_code_app<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: StartCodeApp,
) -> Result<CodeAppSetup, AppError> {
    let record = ctx.active_to_set_up(&input.session_token).await?;
    ctx.confirm_password(&record.person, input.password).await?;
    let person = record.person;
    if ctx
        .store()
        .two_step(person.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .is_some_and(|step| step.confirmed)
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let secret = TotpSecret::generate()?;
    let now = ctx.now();
    ctx.store()
        .put_totp_secret(person.id, secret.as_bytes(), now)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let step = TwoStep {
        person: person.id,
        confirmed: false,
        last_step: TotpStep::NONE,
        created_at: now,
    };
    let event = ctx.event(
        by(person.id),
        entity(RecordKind::Person, person.id),
        vec![changed("two_step")],
    )?;
    let store = ctx.store();
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::upsert_two_step(transaction, &step).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    let enrolment = secret.enrolment(&ctx.services.product_name, &person.email)?;
    Ok(CodeAppSetup {
        otpauth_url: (*enrolment.otpauth_url).clone(),
        qr_png_base64: enrolment.qr_png_base64.clone(),
        secret: (*enrolment.secret_base32).clone(),
    })
}

/// Ten new backup codes: the text to show once, and the hashes to keep.
fn new_backup_codes() -> Result<(Vec<String>, Vec<bb_core::security_log::Digest>), AppError> {
    let mut shown = Vec::with_capacity(BACKUP_CODE_COUNT);
    let mut hashes = Vec::with_capacity(BACKUP_CODE_COUNT);
    for _ in 0..BACKUP_CODE_COUNT {
        let code = HumanCode::generate()?;
        hashes.push(code.hash(Purpose::BackupCode));
        shown.push(code.as_str().to_owned());
    }
    Ok((shown, hashes))
}

/// Finishes setting up the code app with the first code it shows, and makes the backup codes
/// (shown once).
///
/// # Errors
/// `auth.wrong-second-step-code` when the code is wrong, `request.invalid` when setup was not
/// started or is finished already.
pub async fn confirm_code_app<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ConfirmCodeApp,
) -> Result<BackupCodes, AppError> {
    let record = ctx.active_to_set_up(&input.session_token).await?;
    ctx.confirm_password(&record.person, input.password).await?;
    let person = record.person.id;
    let two_step = ctx
        .store()
        .two_step(person)
        .await
        .map_err(|error| internal::<S>(&error))?
        .filter(|step| !step.confirmed)
        .ok_or(ErrorCode::InvalidRequest)?;
    let secret = ctx
        .store()
        .totp_secret(person)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(ErrorCode::InvalidRequest)?;
    let typed: String = input.code.chars().filter(|c| !c.is_whitespace()).collect();
    let slot = ctx.reserve(format!("code-app:{person}"))?;
    let checked = TotpSecret::from_bytes(&secret)
        .check(&typed, ctx.now(), two_step.last_step)
        .map_err(AppError::from);
    let Some(step) = ctx.unless_internal(&slot, checked)? else {
        ctx.slot_failed(slot, Some(person)).await;
        return Err(ErrorCode::WrongSecondStepCode.into());
    };
    let (shown, hashes) = ctx.unless_internal(&slot, new_backup_codes())?;
    let event = ctx.event(
        by(person),
        entity(RecordKind::Person, person),
        vec![changed("two_step")],
    )?;
    let store = ctx.store();
    let written = store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if !S::accept_totp_step(transaction, person, step).await? {
                    return Err(S::aborted());
                }
                S::confirm_two_step(transaction, person).await?;
                S::replace_backup_codes(transaction, person, &hashes).await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::WrongSecondStepCode)));
    ctx.unless_internal(&slot, written)?;
    ctx.slot_succeeded(slot);
    ctx.note(SecurityAction::TwoStepEnabled, Some(person), None)
        .await;
    Ok(BackupCodes(shown))
}

/// Switches the code app off; the person types their password again for this.
///
/// # Errors
/// `auth.wrong-credentials` for a wrong password.
pub async fn remove_code_app<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RemoveCodeApp,
) -> Result<Done, AppError> {
    let record = ctx.active(&input.session_token).await?;
    ctx.confirm_password(&record.person, input.password).await?;
    let person = record.person.id;
    let event = ctx.event(
        by(person),
        entity(RecordKind::Person, person),
        vec![changed("two_step")],
    )?;
    let store = ctx.store();
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::delete_two_step(transaction, person).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.store()
        .delete_totp_secret(person)
        .await
        .map_err(|error| internal::<S>(&error))?;
    ctx.note(SecurityAction::TwoStepDisabled, Some(person), None)
        .await;
    Ok(Done)
}

/// Creates new backup codes; the old ones stop working. The person types their password again.
///
/// # Errors
/// `auth.wrong-credentials` for a wrong password, `request.invalid` without a code app.
pub async fn renew_backup_codes<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RenewBackupCodes,
) -> Result<BackupCodes, AppError> {
    let record = ctx.active(&input.session_token).await?;
    ctx.confirm_password(&record.person, input.password).await?;
    let person = record.person.id;
    if !ctx
        .store()
        .two_step(person)
        .await
        .map_err(|error| internal::<S>(&error))?
        .is_some_and(|step| step.confirmed)
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let (shown, hashes) = new_backup_codes()?;
    let event = ctx.event(
        by(person),
        entity(RecordKind::Person, person),
        vec![changed("backup_codes")],
    )?;
    let store = ctx.store();
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::replace_backup_codes(transaction, person, &hashes).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(SecurityAction::BackupCodesRenewed, Some(person), None)
        .await;
    Ok(BackupCodes(shown))
}

/// Starts adding a passkey.
///
/// # Errors
/// `auth.not-signed-in` without a session.
pub async fn passkey_registration_options<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PasskeyRegistrationQuery,
) -> Result<PasskeyChallenge, AppError> {
    let record = ctx.active_to_set_up(&input.session_token).await?;
    ctx.confirm_password(&record.person, input.password).await?;
    let (_, credentials) = ctx.credentials(record.person.id).await?;
    let (id, options) = ctx
        .passkeys()?
        .start_registration(
            ctx.now(),
            ctx.network_name().as_deref(),
            record.person.id,
            &record.person.email,
            &record.person.display_name,
            &credentials,
        )
        .map_err(challenge_failed)?;
    Ok(PasskeyChallenge {
        challenge_id: id.encode(),
        options,
    })
}

/// Adds a passkey.
///
/// # Errors
/// `auth.passkey-rejected` when the answer of the device does not check out,
/// `request.invalid` for a name that is empty or too long, or when the person has too many.
pub async fn add_passkey<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AddPasskey,
) -> Result<PasskeyView, AppError> {
    let record = ctx.active_to_set_up(&input.session_token).await?;
    ctx.confirm_password(&record.person, input.password).await?;
    let person = record.person.id;
    let label = clean_text(&input.label, LABEL_MAX)?;
    let (existing, _) = ctx.credentials(person).await?;
    if existing.len() >= MAX_PASSKEYS {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let id = ChallengeId::parse(&input.challenge_id)
        .ok_or(AppError::Refused(ErrorCode::PasskeyRejected))?;
    let credential = ctx
        .passkeys()?
        .finish_registration(ctx.now(), id, person, &input.credential)
        .map_err(challenge_failed)?;
    let passkey = StoredPasskey {
        id: PasskeyId::generate(&mut ctx.ids()),
        person,
        credential_id: credential.credential_id,
        data: credential.data,
        label,
        created_at: ctx.now(),
        last_used_at: None,
    };
    let event = ctx.event(
        by(person),
        entity(RecordKind::Passkey, passkey.id),
        vec![changed("passkey")],
    )?;
    let (store, stored) = (ctx.store(), passkey.clone());
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::insert_passkey(transaction, &stored).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::PasskeyRejected)))?;
    ctx.note(
        SecurityAction::PasskeyAdded,
        Some(person),
        Some(entity(RecordKind::Passkey, passkey.id)),
    )
    .await;
    Ok(view(&passkey))
}

/// Removes a passkey; the person types their password again for this.
///
/// # Errors
/// `auth.wrong-credentials` for a wrong password, `resource.not-found` for a passkey that is not
/// the person's.
pub async fn remove_passkey<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RemovePasskey,
) -> Result<Done, AppError> {
    let record = ctx.active(&input.session_token).await?;
    ctx.confirm_password(&record.person, input.password).await?;
    let person = record.person.id;
    let id = input.passkey_id;
    let event = ctx.event(
        by(person),
        entity(RecordKind::Passkey, id),
        vec![changed("passkey")],
    )?;
    let store = ctx.store();
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::delete_passkey(transaction, person, id).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::NotFound)))?;
    ctx.note(
        SecurityAction::PasskeyRemoved,
        Some(person),
        Some(entity(RecordKind::Passkey, id)),
    )
    .await;
    Ok(Done)
}
