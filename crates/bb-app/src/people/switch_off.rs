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
use bb_core::search_mail::MailPause;
use bb_core::sign_in::SecurityAction;
use bb_policy::guards::ensure_a_manager_remains;
use bb_policy::{Permission, Place};
use jiff::Timestamp;
use serde_json::json;

use crate::agents::{agents_stopped, stop_agents_in};
use crate::auth::{
    AppError, AuthStore, Ctx, Done, Stops, TakenBack, by, change_failed, end_ways_in, entity,
};
use crate::member::person::STANDING;
use crate::privacy::replay::{Asked, line, taken_back_in};

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

/// What a switch-off took back, for the security log.
pub struct SwitchedOff {
    /// The ways in it ended.
    pub(crate) taken: TakenBack,
    /// How many invitations the person made were taken back.
    pub(crate) invitations: u64,
}

/// Switches a person off inside a change that is open (ADR-0044, decision 5): the one function the
/// use case and the replay call. Their standing is written, their sessions, keys and grants end,
/// their agents stop with the keys of the agents, the invitations they made are taken back, and
/// the line of the replay list waits. `None` when the replay finds the person switched off or
/// removed already, or nobody (a backup older than the account): nothing is done again.
///
/// # Errors
/// Aborts for a person who is switched off or removed already when an administrator asks, and
/// refuses to switch off the last person who decides roles; otherwise the error of the store.
pub async fn switch_off_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    person: UserId,
    at: Timestamp,
    asked: Asked,
) -> Result<Option<SwitchedOff>, S::Error> {
    if S::standing_in(transaction, person).await?.is_some() {
        return match asked {
            Asked::ByAdministrator(_) => Err(S::aborted()),
            Asked::ByReplay => Ok(None),
        };
    }
    if !S::person_exists(transaction, person).await? {
        return Ok(None);
    }
    let standing = PersonStanding {
        person,
        standing: Standing::SwitchedOff,
        since: at,
        by: asked.by(),
        label: None,
        hold_name: None,
        hold_reason: None,
        hold_until: None,
    };
    // Nobody switches off the last person who manages roles; the replay runs a switch-off that
    // passed this when it was made.
    S::lock_policy(transaction).await?;
    let before = S::organization_state_in(transaction).await?;
    S::set_standing(transaction, &standing).await?;
    let after = S::organization_state_in(transaction).await?;
    if asked != Asked::ByReplay {
        ensure_a_manager_remains(&before, &after).map_err(S::refused)?;
    }
    let taken = end_ways_in::<S>(transaction, person, GrantEnd::TakenBack, at).await?;
    // The keys of the agents go too, so an account that was taken over cannot set its agents to
    // work again with old keys.
    for agent in stop_agents_in::<S>(transaction, person).await? {
        let keys = S::delete_keys_of_agent(transaction, agent).await?;
        taken_back_in::<S, _>(transaction, at, ReplayKind::AccessKey, &keys).await?;
    }
    let invitations = S::delete_open_invitations_by(transaction, person).await?;
    // Their e-mailed searches wait for them; *Switch on* does not start them again (ADR-0044,
    // amendment 2026-11-24).
    S::pause_search_mails_of(transaction, person, MailPause::SwitchedOff).await?;
    S::wait_to_replay(
        transaction,
        &[line(at, ReplayKind::SwitchOff, person, None)],
    )
    .await?;
    Ok(Some(SwitchedOff { taken, invitations }))
}

/// Switches a person off at once (ADR-0044, decision 5).
///
/// # Errors
/// `access.forbidden` without *Manage people* for the whole organization, `resource.not-found` for
/// nobody, `request.invalid` for oneself (nobody locks themselves out), `person.not-active` for a
/// person who is switched off or removed already, `edit.version-changed` for one switched off
/// meanwhile, and `access.would-lock-out` for the last person who decides roles.
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
        return Err(AppError::Refused(ErrorCode::PersonNotActive));
    }
    let now = ctx.now();
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Person, person),
        vec![
            state_change(Standing::Active, Standing::SwitchedOff),
            agents_stopped(),
        ],
    )?;
    let asked = Asked::ByAdministrator(actor.person.id);
    let switched = ctx
        .store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                switch_off_in::<S>(transaction, person, now, asked)
                    .await?
                    .ok_or_else(S::aborted)
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)))?;
    let SwitchedOff { taken, invitations } = switched;
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
/// nobody, `person.not-switched-off` for a person who is not switched off (active or removed), and
/// `edit.version-changed` for one switched on meanwhile.
pub async fn switch_on<S: AuthStore>(ctx: &Ctx<'_, S>, input: SwitchOn) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.require(&actor, Permission::ManagePeople, Place::Organization)
        .await?;
    let person = input.person;
    let switched_off =
        ctx.known_person_or_missing(person).await?.standing() == Standing::SwitchedOff;
    if !switched_off {
        return Err(AppError::Refused(ErrorCode::PersonNotSwitchedOff));
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
