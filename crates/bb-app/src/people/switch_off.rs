//! *Switch off* and *Switch on* (PRIV-04, ADR-0044, decision 5, M9-Q4, M9-Q5).
//!
//! An administrator with *Manage people* switches a person off at once, in one change: their
//! sessions end, their access keys and the grants of AI apps are taken back, their agents stop
//! with their hand-overs and standing rules, and the invitations they made are taken back. Their
//! assignments and project memberships stay until somebody changes them. A switched-off person
//! signs in to nothing, is offered in no pick of people and is told nothing, and keeps every
//! record. *Switch on* gives back the sign-in and nothing else. Both are written to the security
//! log, and a switch-off to the replay list, so a restore does not undo it.

use bb_core::ErrorCode;
use bb_core::backup::replay::ReplayKind;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::UserId;
use bb_core::oauth::GrantEnd;
use bb_core::person::{PersonStanding, Standing};
use bb_core::sign_in::SecurityAction;
use bb_policy::guards::ensure_a_manager_remains;
use bb_policy::{Permission, Place};
use serde_json::json;

use crate::agents::{agents_stopped, stop_agents_in};
use crate::auth::{AppError, AuthStore, Ctx, Done, Stops, by, change_failed, end_ways_in, entity};
use crate::member::person::STANDING;
use crate::privacy::replay::{line, taken_back_in};

/// Switch a person off.
pub struct SwitchOff {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The person.
    pub person: UserId,
}

/// Switch a switched-off person on again.
pub struct SwitchOn {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The person.
    pub person: UserId,
}

/// The change of the standing of a person, as their events record it.
pub(crate) fn state_change(before: Standing, after: Standing) -> FieldChange {
    FieldChange {
        field: STANDING.to_owned(),
        before: Some(json!(before.code())),
        after: Some(json!(after.code())),
    }
}

/// Switches a person off at once (ADR-0044, decision 5).
///
/// # Errors
/// `access.forbidden` without *Manage people* for the whole organization, `resource.not-found` for
/// nobody, `request.invalid` for oneself (nobody locks themselves out), `edit.version-changed`
/// for a person who is switched off or removed already.
pub async fn switch_off<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SwitchOff,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.require(&actor, Permission::ManagePeople, Place::Organization)
        .await?;
    let person = input.person;
    if person == actor.person.id {
        return Err(AppError::Refused(ErrorCode::InvalidRequest));
    }
    if !ctx.known_person_or_missing(person).await?.active() {
        return Err(AppError::Refused(ErrorCode::VersionChanged));
    }
    let now = ctx.now();
    let standing = PersonStanding {
        person,
        standing: Standing::SwitchedOff,
        since: now,
        by: actor.person.id,
        label: None,
        hold_name: None,
        hold_reason: None,
        hold_until: None,
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Person, person),
        vec![
            state_change(Standing::Active, Standing::SwitchedOff),
            agents_stopped(),
        ],
    )?;
    let (taken, invitations) = ctx
        .store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::standing_in(transaction, person).await?.is_some() {
                    return Err(S::aborted());
                }
                // Nobody switches off the last person who manages roles.
                S::lock_policy(transaction).await?;
                let before = S::organization_state_in(transaction).await?;
                S::set_standing(transaction, &standing).await?;
                let after = S::organization_state_in(transaction).await?;
                ensure_a_manager_remains(&before, &after).map_err(S::refused)?;
                let taken = end_ways_in::<S>(transaction, person, GrantEnd::TakenBack, now).await?;
                // The keys of the agents go too, so an account that was taken over cannot set its
                // agents to work again with old keys.
                for agent in stop_agents_in::<S>(transaction, person).await? {
                    let keys = S::delete_keys_of_agent(transaction, agent).await?;
                    taken_back_in::<S, _>(transaction, now, ReplayKind::AccessKey, &keys).await?;
                }
                let invitations = S::delete_open_invitations_by(transaction, person).await?;
                S::wait_to_replay(
                    transaction,
                    &[line(now, ReplayKind::SwitchOff, person, None)],
                )
                .await?;
                Ok((taken, invitations))
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)))?;
    ctx.note(
        SecurityAction::PersonSwitchedOff,
        Some(actor.person.id),
        Some(entity(RecordKind::Person, person)),
    )
    .await;
    ctx.note_taken_back(person, &taken).await;
    // One entry for each invitation the person made that was taken back.
    for _ in 0..invitations {
        ctx.note(SecurityAction::InvitationRevoked, Some(person), None)
            .await;
    }
    Ok(Done)
}

/// Switches a switched-off person on again: they may sign in. What *Switch off* took back stays
/// taken back (ADR-0044, decision 5).
///
/// # Errors
/// `access.forbidden` without *Manage people* for the whole organization, `resource.not-found` for
/// nobody, `edit.version-changed` for a person who is not switched off (active or removed).
pub async fn switch_on<S: AuthStore>(ctx: &Ctx<'_, S>, input: SwitchOn) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.require(&actor, Permission::ManagePeople, Place::Organization)
        .await?;
    let person = input.person;
    let switched_off =
        ctx.known_person_or_missing(person).await?.standing() == Standing::SwitchedOff;
    if !switched_off {
        return Err(AppError::Refused(ErrorCode::VersionChanged));
    }
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Person, person),
        vec![state_change(Standing::SwitchedOff, Standing::Active)],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::clear_standing(transaction, person).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)))?;
    ctx.note(
        SecurityAction::PersonSwitchedOn,
        Some(actor.person.id),
        Some(entity(RecordKind::Person, person)),
    )
    .await;
    Ok(Done)
}
