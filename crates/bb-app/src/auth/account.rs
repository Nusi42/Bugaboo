//! Making accounts and changing passwords (AUTH-01, AUTH-06, AUTH-09): the first administrator
//! with the setup code, signing up, accepting an invitation, and changing the password.

use bb_auth::{
    EmailAddress, HumanCode, PasswordProblem, Purpose, SecretToken, check_policy, code_hash,
    domain_is_listed, same_digest,
};
use bb_core::ErrorCode;
use bb_core::event::Actor;
use bb_core::id::UserId;
use bb_core::person::Person;
use bb_core::sign_in::{SecurityAction, SessionStage, SetupCode};
use serde_json::json;
use zeroize::Zeroizing;

use super::ctx::{
    AppError, Ctx, by, change_failed, changed, entity, fixed_entity, internal, set_to,
};
use super::store::AuthStore;
use super::types::{
    AcceptInvitation, ChangePassword, CompleteSetup, IssueSetupCode, SignUp, Started,
};

/// The longest display name, in characters.
const DISPLAY_NAME_MAX: usize = 100;

/// The password as a new password: the rules of AUTH-01, as the error a person can read.
pub(crate) fn check_new_password(password: &str) -> Result<(), AppError> {
    check_policy(password).map_err(|problem| {
        AppError::Refused(match problem {
            PasswordProblem::TooShort => ErrorCode::PasswordTooShort,
            PasswordProblem::Breached => ErrorCode::PasswordBreached,
            PasswordProblem::TooLong => ErrorCode::InvalidRequest,
        })
    })
}

/// The name as it is kept: trimmed, not empty, not too long, no control characters.
pub(crate) fn clean_display_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty()
        || name.chars().count() > DISPLAY_NAME_MAX
        || name.chars().any(char::is_control)
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    Ok(name.to_owned())
}

impl<S: AuthStore> Ctx<'_, S> {
    /// A new person (not stored yet).
    fn new_person(
        &self,
        display_name: String,
        email: &EmailAddress,
        administrator: bool,
    ) -> Person {
        Person {
            id: UserId::generate(&mut self.ids()),
            display_name,
            email: email.as_str().to_owned(),
            created_at: self.now(),
            administrator,
        }
    }
}

/// Makes the setup code of the first start, when no administrator exists. Called by the server
/// at every start; the code printed before stops working. When setup is over, a code that is
/// left over is removed and nothing is returned.
///
/// # Errors
/// Fails when the store fails.
pub async fn issue_setup_code<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    _input: IssueSetupCode,
) -> Result<Option<HumanCode>, AppError> {
    let administrators = ctx
        .store()
        .administrators()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let waiting = ctx
        .store()
        .setup_code()
        .await
        .map_err(|error| internal::<S>(&error))?
        .is_some();
    let code = if administrators == 0 {
        Some(HumanCode::generate()?)
    } else if waiting {
        None
    } else {
        return Ok(None);
    };
    let entry = code.as_ref().map(|code| SetupCode {
        code_hash: code.hash(Purpose::SetupCode),
        created_at: ctx.now(),
    });
    let event = ctx.event(
        Actor::System,
        fixed_entity("setup_code", "first-start")?,
        vec![changed("setup_code")],
    )?;
    let store = ctx.store();
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                match &entry {
                    Some(entry) => S::put_setup_code(transaction, entry).await,
                    None => S::clear_setup_code(transaction).await,
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, None))?;
    Ok(code)
}

/// Makes the first administrator with the setup code and signs them in. The setup code is used
/// up in the same change, so it works once, and setup is closed for good afterwards.
///
/// # Errors
/// `auth.wrong_setup_code` for a wrong code and when setup is over (it does not say which),
/// the password errors, 429 after too many wrong codes.
pub async fn complete_setup<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CompleteSetup,
) -> Result<Started, AppError> {
    // Wrong codes are counted per network, so nobody far away can lock the person at the console out.
    let slot = ctx.reserve(format!(
        "setup:{}",
        ctx.request
            .network
            .as_ref()
            .map_or_else(|| "-".to_owned(), ToString::to_string)
    ))?;
    let wrong = || AppError::Refused(ErrorCode::WrongSetupCode);
    let stored = ctx
        .store()
        .setup_code()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let administrators = ctx
        .store()
        .administrators()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let typed = code_hash(Purpose::SetupCode, &input.code);
    let right = administrators == 0
        && matches!(
            (&stored, &typed),
            (Some(stored), Some(typed)) if same_digest(&stored.code_hash, typed)
        );
    let (true, Some(typed)) = (right, typed) else {
        ctx.slot_failed(slot, None).await;
        ctx.note(SecurityAction::SetupFailed, None, None).await;
        return Err(wrong());
    };
    // The code is right: whatever is wrong with the rest is no guess at the code.
    ctx.slot_succeeded(slot);
    let email = EmailAddress::parse(&input.email).ok_or(ErrorCode::InvalidRequest)?;
    let display_name = clean_display_name(&input.display_name)?;
    check_new_password(&input.password)?;
    let hash = ctx
        .services
        .hasher
        .hash(Zeroizing::new(input.password))
        .await?;
    let person = ctx.new_person(display_name, &email, true);
    let old = ctx
        .previous_session(input.previous_session.as_deref())
        .await?;
    let new = ctx.new_session(person.id, SessionStage::Active)?;
    let settings = ctx.settings().await?;
    let event = ctx.event(
        by(person.id),
        entity("person", person.id)?,
        vec![changed("account"), set_to("administrator", json!(true))],
    )?;
    let (store, stored_person, session, now) =
        (ctx.store(), person.clone(), new.session.clone(), ctx.now());
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if !S::take_setup_code(transaction, &typed).await? {
                    return Err(S::aborted());
                }
                if let Some(old) = old {
                    S::delete_session(transaction, old).await?;
                }
                S::insert_person(transaction, &stored_person, &hash, now).await?;
                S::insert_session(transaction, &session).await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Some(ErrorCode::WrongSetupCode)))?;
    ctx.note(
        SecurityAction::SetupCompleted,
        Some(person.id),
        Some(entity("person", person.id)?),
    )
    .await;
    ctx.started(&new, &person, &settings, None)
}

/// Makes an account without an invitation, when an administrator allows it, and signs the
/// person in.
///
/// # Errors
/// `auth.sign_up_refused` when signing up is off, when the domain of the address is not on the
/// list, and when the address has an account already (it does not say which).
pub async fn sign_up<S: AuthStore>(ctx: &Ctx<'_, S>, input: SignUp) -> Result<Started, AppError> {
    let refused = || AppError::Refused(ErrorCode::SignUpRefused);
    let settings = ctx.settings().await?;
    let email = EmailAddress::parse(&input.email).ok_or(ErrorCode::InvalidRequest)?;
    let listed = settings.sign_up_domains.trim().is_empty()
        || domain_is_listed(&email, &settings.sign_up_domains);
    if !settings.sign_up_enabled || !listed {
        ctx.note(SecurityAction::SignUpRefused, None, None).await;
        return Err(refused());
    }
    let display_name = clean_display_name(&input.display_name)?;
    check_new_password(&input.password)?;
    let hash = ctx
        .services
        .hasher
        .hash(Zeroizing::new(input.password))
        .await?;
    let person = ctx.new_person(display_name, &email, false);
    let old = ctx
        .previous_session(input.previous_session.as_deref())
        .await?;
    let new = ctx.new_session(person.id, SessionStage::Active)?;
    let event = ctx.event(
        by(person.id),
        entity("person", person.id)?,
        vec![changed("account")],
    )?;
    let (store, stored_person, session, now) =
        (ctx.store(), person.clone(), new.session.clone(), ctx.now());
    let result = store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if let Some(old) = old {
                    S::delete_session(transaction, old).await?;
                }
                S::insert_person(transaction, &stored_person, &hash, now).await?;
                S::insert_session(transaction, &session).await
            })
        })
        .await;
    if let Err(error) = result {
        let error = change_failed::<S>(&error, Some(ErrorCode::SignUpRefused));
        if matches!(error, AppError::Refused(_)) {
            ctx.note(SecurityAction::SignUpRefused, None, None).await;
        }
        return Err(error);
    }
    ctx.note(
        SecurityAction::SignUpAccepted,
        Some(person.id),
        Some(entity("person", person.id)?),
    )
    .await;
    ctx.started(&new, &person, &settings, None)
}

/// Makes an account from an invitation link and signs the person in. The invitation is used up
/// in the same change.
///
/// # Errors
/// `auth.invitation_invalid` for a link that is unknown, used, taken back or run out (it does not
/// say which), and the password errors.
pub async fn accept_invitation<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AcceptInvitation,
) -> Result<Started, AppError> {
    let invalid = || AppError::Refused(ErrorCode::InvitationInvalid);
    let now = ctx.now();
    let found = match SecretToken::parse(&input.secret) {
        Some(secret) => ctx
            .store()
            .invitation_by_hash(&secret.hash(Purpose::Invitation))
            .await
            .map_err(|error| internal::<S>(&error))?,
        None => None,
    };
    let Some(invitation) =
        found.filter(|invitation| invitation.used_at.is_none() && invitation.expires_at > now)
    else {
        ctx.note(SecurityAction::InvitationRefused, None, None)
            .await;
        return Err(invalid());
    };
    let email = EmailAddress::parse(&invitation.email).ok_or_else(invalid)?;
    let display_name = clean_display_name(&input.display_name)?;
    check_new_password(&input.password)?;
    let hash = ctx
        .services
        .hasher
        .hash(Zeroizing::new(input.password))
        .await?;
    let person = ctx.new_person(display_name, &email, false);
    let old = ctx
        .previous_session(input.previous_session.as_deref())
        .await?;
    let new = ctx.new_session(person.id, SessionStage::Active)?;
    let settings = ctx.settings().await?;
    let event = ctx.event(
        by(person.id),
        entity("person", person.id)?,
        vec![changed("account")],
    )?;
    let (store, stored_person, session, invitation_id) = (
        ctx.store(),
        person.clone(),
        new.session.clone(),
        invitation.id,
    );
    let result = store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if !S::use_invitation(transaction, invitation_id, now).await? {
                    return Err(S::aborted());
                }
                if let Some(old) = old {
                    S::delete_session(transaction, old).await?;
                }
                S::insert_person(transaction, &stored_person, &hash, now).await?;
                S::insert_session(transaction, &session).await
            })
        })
        .await;
    if let Err(error) = result {
        let error = change_failed::<S>(&error, Some(ErrorCode::InvitationInvalid));
        if matches!(error, AppError::Refused(_)) {
            ctx.note(SecurityAction::InvitationRefused, None, None)
                .await;
        }
        return Err(error);
    }
    ctx.note(
        SecurityAction::InvitationAccepted,
        Some(person.id),
        Some(entity("invitation", invitation.id)?),
    )
    .await;
    ctx.started(&new, &person, &settings, None)
}

/// Changes the password. The person types the current one again; all other sessions end and the
/// session of the request gets a new secret.
///
/// # Errors
/// `auth.wrong_credentials` for a wrong current password, the password errors for the new one.
pub async fn change_password<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangePassword,
) -> Result<Started, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person;
    ctx.confirm_password(&person, input.current_password)
        .await?;
    check_new_password(&input.new_password)?;
    let hash = ctx
        .services
        .hasher
        .hash(Zeroizing::new(input.new_password))
        .await?;
    let new = ctx.new_session(person.id, SessionStage::Active)?;
    let settings = ctx.settings().await?;
    let event = ctx.event(
        by(person.id),
        entity("person", person.id)?,
        vec![changed("password")],
    )?;
    let (store, session, person_id, now) = (ctx.store(), new.session.clone(), person.id, ctx.now());
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::set_password(transaction, person_id, &hash, now).await?;
                S::delete_sessions_of(transaction, person_id, None).await?;
                S::insert_session(transaction, &session).await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, None))?;
    ctx.note(SecurityAction::PasswordChanged, Some(person.id), None)
        .await;
    ctx.started(&new, &person, &settings, None)
}
