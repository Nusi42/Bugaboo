//! Invitations and the rule for signing up (AUTH-06): what administrators do. People who use an
//! invitation or sign up are in `account.rs`.

use bb_auth::{EmailAddress, Purpose, SecretToken, listed_domains};
use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::id::InvitationId;
use bb_core::paging::{Cut, page_len, read_len, read_time_cursor, write_time_cursor};
use bb_core::settings::{AUTH_SIGN_UP_DOMAINS, AUTH_SIGN_UP_ENABLED, SettingValue};
use bb_core::sign_in::{Invitation, SecurityAction};
use bb_policy::{Authorized, Permission, Place};
use jiff::Timestamp;
use serde_json::json;

use super::ctx::Ctx;
use super::error::{AppError, Stops, change_failed, internal};
use super::events::{by, changed, entity, fixed_entity, set_to};
use super::request_session::INVITATION_SECONDS;
use super::store::AuthStore;
use super::types::{
    ChangeSignUpPolicy, CreateInvitation, CreatedInvitation, Done, InvitationLookup,
    InvitationLookupQuery, InvitationPage, InvitationView, InvitationsQuery, RevokeInvitation,
    SignUpPolicy, SignUpPolicyQuery,
};
use crate::input::cursor_or_invalid;

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
/// `access.forbidden` without *Invite people*, `request.invalid` for an address that cannot be one
/// or that has an account already.
pub async fn create_invitation<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateInvitation,
) -> Result<Authorized<CreatedInvitation>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManageInvitations, Place::Organization)
        .await?;
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
        invited_by: actor.person.id,
        created_at: now,
        expires_at,
        used_at: None,
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Invitation, invitation.id),
        vec![set_to("email", json!(invitation.email))],
    )?;
    let (store, stored) = (ctx.store(), invitation.clone());
    store
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::insert_invitation(transaction, &stored).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(
        SecurityAction::InvitationCreated,
        Some(actor.person.id),
        Some(entity(RecordKind::Invitation, invitation.id)),
    )
    .await;
    Ok(access.authorize(CreatedInvitation {
        invitation: view(&invitation),
        link_path: format!("/invitation#{}", secret.encode()),
    }))
}

/// Takes an invitation back.
///
/// # Errors
/// `access.forbidden` without *Invite people*, `resource.not_found` for an unknown invitation.
pub async fn revoke_invitation<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RevokeInvitation,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.require(&actor, Permission::ManageInvitations, Place::Organization)
        .await?;
    let id = input.invitation_id;
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Invitation, id),
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
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::NotFound)))?;
    ctx.note(
        SecurityAction::InvitationRevoked,
        Some(actor.person.id),
        Some(entity(RecordKind::Invitation, id)),
    )
    .await;
    Ok(Done)
}

/// The open invitations, one page at a time (newest first).
///
/// # Errors
/// `access.forbidden` without *Invite people*, `request.invalid` for a cursor that is not one.
pub async fn invitations<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: InvitationsQuery,
) -> Result<Authorized<InvitationPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManageInvitations, Place::Organization)
        .await?;
    let len = page_len(Some(input.limit));
    let after = cursor_or_invalid(input.cursor.as_deref(), read_time_cursor)?;
    let page = Cut::of(
        ctx.store()
            .open_invitations(ctx.now(), after, read_len(len))
            .await
            .map_err(|error| internal::<S>(&error))?,
        len,
    );
    let next_cursor = page.next_cursor(|last| write_time_cursor(last.created_at, last.id));
    Ok(access.authorize(InvitationPage {
        items: page.items.iter().map(view).collect(),
        next_cursor,
    }))
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

/// Who may sign up on their own.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*.
pub async fn sign_up_policy<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SignUpPolicyQuery,
) -> Result<Authorized<SignUpPolicy>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let settings = ctx.settings().await?;
    Ok(access.authorize(SignUpPolicy {
        enabled: settings.sign_up_enabled,
        domains: listed_domains(&settings.sign_up_domains).collect(),
    }))
}

/// Changes who may sign up on their own. An empty list of domains lets every address sign up
/// while signing up is on.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*, `request.invalid` for a domain that
/// cannot be one.
pub async fn change_sign_up_policy<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeSignUpPolicy,
) -> Result<Authorized<SignUpPolicy>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let mut domains: Vec<String> = Vec::new();
    for domain in input
        .policy
        .domains
        .iter()
        .flat_map(|item| listed_domains(item))
    {
        let plain = domain.contains('.')
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
    let policy = SignUpPolicy {
        enabled: input.policy.enabled,
        domains,
    };
    let list = policy.domains.join(",");
    // The registry says how long the list and its items may be (one place for the numbers).
    if !AUTH_SIGN_UP_DOMAINS.definition().list_fits(&list) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let event = ctx.event(
        by(actor.person.id),
        fixed_entity(RecordKind::Setting, "auth.sign-up-policy")?,
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
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(
        SecurityAction::SignUpPolicyChanged,
        Some(actor.person.id),
        None,
    )
    .await;
    Ok(access.authorize(policy))
}
