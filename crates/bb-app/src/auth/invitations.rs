//! Invitations and the rule for signing up (AUTH-06): what administrators do. People who use an
//! invitation or sign up are in `account.rs`.

use bb_auth::{EmailAddress, Purpose, SecretToken};
use bb_core::ErrorCode;
use bb_core::id::InvitationId;
use bb_core::settings::{AUTH_SIGN_UP_DOMAINS, AUTH_SIGN_UP_ENABLED, SettingValue};
use bb_core::sign_in::{Invitation, SecurityAction};
use jiff::Timestamp;
use serde_json::json;

use super::ctx::{
    AppError, Ctx, INVITATION_SECONDS, by, change_failed, changed, entity, fixed_entity, internal,
    set_to,
};
use super::store::AuthStore;
use super::types::{
    ChangeSignUpPolicy, CreateInvitation, CreatedInvitation, Done, InvitationLookup,
    InvitationLookupQuery, InvitationPage, InvitationView, InvitationsQuery, MAX_PAGE,
    RevokeInvitation, SignUpPolicy, SignUpPolicyQuery,
};

/// The longest domain name (RFC 1035).
const DOMAIN_MAX: usize = 253;
/// The most domains the rule for signing up can list (the whole list is the value of one setting).
const MAX_SIGN_UP_DOMAINS: usize = 100;

fn view(invitation: &Invitation) -> InvitationView {
    InvitationView {
        id: invitation.id,
        email: invitation.email.clone(),
        created_at: invitation.created_at,
        expires_at: invitation.expires_at,
    }
}

/// Makes an invitation. The link with its secret is returned once; only a hash is kept.
///
/// # Errors
/// `access.forbidden` for people who are not administrators, `request.invalid` for an address
/// that cannot be one or that has an account already.
pub async fn create_invitation<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateInvitation,
) -> Result<CreatedInvitation, AppError> {
    let record = ctx.administrator(&input.session_token).await?;
    let email = EmailAddress::parse(&input.email).ok_or(ErrorCode::InvalidRequest)?;
    if ctx
        .store()
        .account_by_email(email.as_str())
        .await
        .map_err(|error| internal::<S>(&error))?
        .is_some()
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let secret = SecretToken::generate()?;
    let now = ctx.now();
    let expires_at = Timestamp::from_second(now.as_second().saturating_add(INVITATION_SECONDS))
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let invitation = Invitation {
        id: InvitationId::generate(&mut ctx.ids()),
        token_hash: secret.hash(Purpose::Invitation),
        email: email.as_str().to_owned(),
        invited_by: record.person.id,
        created_at: now,
        expires_at,
        used_at: None,
    };
    let event = ctx.event(
        by(record.person.id),
        entity("invitation", invitation.id)?,
        vec![set_to("email", json!(invitation.email))],
    )?;
    let (store, stored) = (ctx.store(), invitation.clone());
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::insert_invitation(transaction, &stored).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, None))?;
    ctx.note(
        SecurityAction::InvitationCreated,
        Some(record.person.id),
        Some(entity("invitation", invitation.id)?),
    )
    .await;
    Ok(CreatedInvitation {
        invitation: view(&invitation),
        link_path: format!("/invitation#{}", secret.encode()),
    })
}

/// Takes an invitation back.
///
/// # Errors
/// `access.forbidden` for people who are not administrators, `resource.not_found` for an unknown
/// invitation.
pub async fn revoke_invitation<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RevokeInvitation,
) -> Result<Done, AppError> {
    let record = ctx.administrator(&input.session_token).await?;
    let id = input.invitation_id;
    let event = ctx.event(
        by(record.person.id),
        entity("invitation", id)?,
        vec![changed("invitation")],
    )?;
    let store = ctx.store();
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::delete_invitation(transaction, id).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Some(ErrorCode::NotFound)))?;
    ctx.note(
        SecurityAction::InvitationRevoked,
        Some(record.person.id),
        Some(entity("invitation", id)?),
    )
    .await;
    Ok(Done)
}

/// Where a page ends, as text: the time the last invitation was made (microseconds) and its ID.
fn write_cursor(last: &Invitation) -> String {
    format!("{}.{}", last.created_at.as_microsecond(), last.id)
}

fn read_cursor(text: &str) -> Result<(Timestamp, InvitationId), ()> {
    let (micros, id) = text.split_once('.').ok_or(())?;
    let created = Timestamp::from_microsecond(micros.parse().map_err(|_| ())?).map_err(|_| ())?;
    Ok((created, id.parse().map_err(|_| ())?))
}

/// The open invitations, one page at a time (newest first).
///
/// # Errors
/// `access.forbidden` for people who are not administrators, `request.invalid` for a cursor that
/// is not one.
pub async fn invitations<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: InvitationsQuery,
) -> Result<InvitationPage, AppError> {
    ctx.administrator(&input.session_token).await?;
    let limit = u64::from(input.limit.clamp(1, MAX_PAGE));
    let after = input
        .cursor
        .as_deref()
        .map(read_cursor)
        .transpose()
        .map_err(|()| AppError::Refused(ErrorCode::InvalidRequest))?;
    // One more than the page, to know whether there is a next one.
    let mut found = ctx
        .store()
        .open_invitations(ctx.now(), after, limit + 1)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let more = u64::try_from(found.len()).is_ok_and(|count| count > limit);
    found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    let next_cursor = more.then(|| found.last().map(write_cursor)).flatten();
    Ok(InvitationPage {
        items: found.iter().map(view).collect(),
        next_cursor,
    })
}

/// Which address an invitation link is for, so the page can show it.
///
/// # Errors
/// `auth.invitation_invalid` for a link that does not work (it does not say why).
pub async fn invitation_lookup<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: InvitationLookupQuery,
) -> Result<InvitationLookup, AppError> {
    let invalid = || AppError::Refused(ErrorCode::InvitationInvalid);
    let secret = SecretToken::parse(&input.secret).ok_or_else(invalid)?;
    let now = ctx.now();
    ctx.store()
        .invitation_by_hash(&secret.hash(Purpose::Invitation))
        .await
        .map_err(|error| internal::<S>(&error))?
        .filter(|invitation| invitation.used_at.is_none() && invitation.expires_at > now)
        .map(|invitation| InvitationLookup {
            email: invitation.email,
        })
        .ok_or_else(invalid)
}

fn domains_of(list: &str) -> Vec<String> {
    list.split(',')
        .map(|item| item.trim().trim_start_matches('@').to_lowercase())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Who may sign up on their own.
///
/// # Errors
/// `access.forbidden` for people who are not administrators.
pub async fn sign_up_policy<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SignUpPolicyQuery,
) -> Result<SignUpPolicy, AppError> {
    ctx.administrator(&input.session_token).await?;
    let settings = ctx.settings().await?;
    Ok(SignUpPolicy {
        enabled: settings.sign_up_enabled,
        domains: domains_of(&settings.sign_up_domains),
    })
}

/// Changes who may sign up on their own. An empty list of domains lets every address sign up
/// while signing up is on.
///
/// # Errors
/// `access.forbidden` for people who are not administrators, `request.invalid` for a domain that
/// cannot be one.
pub async fn change_sign_up_policy<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeSignUpPolicy,
) -> Result<SignUpPolicy, AppError> {
    let record = ctx.administrator(&input.session_token).await?;
    let mut domains: Vec<String> = Vec::new();
    for domain in input
        .policy
        .domains
        .iter()
        .flat_map(|item| domains_of(item))
    {
        let plain = domain.len() <= DOMAIN_MAX
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
            && !domain.contains("..")
            && !domain
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || c == '@');
        if !plain {
            return Err(ErrorCode::InvalidRequest.into());
        }
        if !domains.contains(&domain) {
            domains.push(domain);
        }
    }
    if domains.len() > MAX_SIGN_UP_DOMAINS {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let policy = SignUpPolicy {
        enabled: input.policy.enabled,
        domains,
    };
    let list = policy.domains.join(",");
    let event = ctx.event(
        by(record.person.id),
        fixed_entity("setting", "auth.sign-up-policy")?,
        vec![
            set_to("auth_sign_up_enabled", json!(policy.enabled)),
            set_to("auth_sign_up_domains", json!(list)),
        ],
    )?;
    let (store, enabled, now) = (ctx.store(), policy.enabled, ctx.now());
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::put_setting(
                    transaction,
                    AUTH_SIGN_UP_ENABLED.definition().key(),
                    &SettingValue::Bool(enabled),
                    now,
                )
                .await?;
                S::put_setting(
                    transaction,
                    AUTH_SIGN_UP_DOMAINS.definition().key(),
                    &SettingValue::Text(list),
                    now,
                )
                .await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, None))?;
    ctx.note(
        SecurityAction::SignUpPolicyChanged,
        Some(record.person.id),
        None,
    )
    .await;
    Ok(policy)
}
