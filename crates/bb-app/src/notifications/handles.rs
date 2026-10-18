//! Handles (M3-F70, M4-Q9, ADR-0033 §6): a person's short unique name for mentions, `@ana.brandt`.
//!
//! The handle is made from the name when the account is made (in the same change), is unique among
//! all handles, and can be changed in *My settings*. A mention stores the person and not the
//! handle, and the handle a person had before stays with them, so the `@name` of an old text still
//! names them and nobody else can take it. A handle is people information and follows *Who sees
//! people*: it is offered to a writer only where they see names (`mention_choices`).

use std::future::Future;

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::handle::{CANDIDATES_MAX, base_of, candidate, change_every, is_handle};
use bb_core::id::UserId;
use bb_core::person::Person;
use bb_policy::{Authorized, Masked, Place};
use jiff::Timestamp;
use serde::Serialize;
use serde_json::json;

use crate::auth::{
    AccountStore, AppError, AuthStore, Ctx, Stops, by, change_failed, entity, internal,
};
use crate::member;
use crate::projects::Why;
use crate::store_macros::{reads, writes};
use bb_core::event::{Actor as EventActor, FieldChange};

/// How many people without a handle one round of making them reads.
const MISSING_AT_ONCE: u64 = 100;

/// What the handles need from storage.
pub trait HandleStore: AccountStore + 'static {
    reads! {
        /// The handle a person has now.
        fn handle_of_person(person: UserId) -> Option<String>;
        /// When the person last changed their handle, as a read that takes no lock: none while
        /// they still have the one they were given with the account. The change judges the same
        /// rule again under a lock (`handle_changed_in`).
        fn handle_changed_at(person: UserId) -> Option<Timestamp>;
        /// The person who has this handle, or had it: a handle nobody else can take.
        fn person_with_handle(handle: &str) -> Option<UserId>;
        /// The person who has this handle now. A handle a person gave up names nobody in a new
        /// text.
        fn person_with_current_handle(handle: &str) -> Option<UserId>;
        /// People who have no handle yet, at most `limit`: the people made before handles existed.
        fn people_without_handle(limit: u64) -> Vec<Person>;
        /// The people whose handle now starts with `start`, with it, by handle, at most `limit`.
        fn handle_choices(start: &str, limit: u64) -> Vec<(Person, String)>;
    }

    writes! {
        /// When the person last changed their handle, as the change that needs to know sees it:
        /// none while they still have the one they were given with the account. It takes a lock
        /// on the handles of the person that lasts to the end of the change, so two changes of one
        /// person at the same moment cannot both pass the rule of how often a handle changes.
        fn handle_changed_in(person: UserId) -> Option<Timestamp>;
        /// Whether somebody has this handle, or had it (a former handle is not free), as the change
        /// that needs a free one sees it.
        fn handle_taken_in(handle: &str) -> bool;
        /// Gives a person this handle as the one they have now. The handle they had stays theirs
        /// as a former one. The change stops itself when somebody else has the handle.
        fn put_handle(person: UserId, handle: &str, at: Timestamp) -> ();
    }
}

/// Whether a person who changed their handle at `changed` has to wait yet: the one rule of how
/// often a handle changes, read before anything is said about a handle and judged again inside the
/// change.
fn changed_lately(changed: Option<Timestamp>, now: Timestamp) -> bool {
    changed
        .and_then(|changed| changed.checked_add(change_every()).ok())
        .is_some_and(|again| again > now)
}

/// The handle of the person who asks.
pub struct MyHandleQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// Change the handle of the person who asks.
pub struct ChangeHandle {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The new handle: lower case, words joined by dots, like `ana.brandt`; without the `@`.
    pub handle: String,
}

/// Makes the handle of every person who has none (what the server does at every start).
pub struct CreateMissingHandles;

/// The handle of a person.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MyHandle {
    /// The handle, without the `@`.
    pub handle: String,
}

impl Masked for MyHandle {}

/// Gives a person the first free handle made from their name, inside the change that needs it.
/// Called inside the change that makes the account (the person is stored first), and by the making
/// of the handles of the people who have none. `reserved` are the handles of the groups and the
/// roles, which a person does not get.
///
/// # Errors
/// The error of the store, or its "aborted" error when no free handle was found.
pub(crate) async fn give_handle<S: AuthStore>(
    transaction: &mut S::Transaction,
    person: &Person,
    at: Timestamp,
    reserved: &[String],
) -> Result<(), S::Error> {
    let base = base_of(&person.display_name);
    for number in 1..=CANDIDATES_MAX {
        let found = candidate(&base, number);
        if !reserved.contains(&found) && !S::handle_taken_in(transaction, &found).await? {
            return S::put_handle(transaction, person.id, &found, at).await;
        }
    }
    Err(S::aborted())
}

/// The handle of the person who asks.
///
/// # Errors
/// `resource.not_found` for a person who has none (every person has one from the first start on).
pub async fn my_handle<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MyHandleQuery,
) -> Result<Authorized<MyHandle>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let handle = ctx
        .store()
        .handle_of_person(actor.person.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    Ok(held.authorize(MyHandle { handle }))
}

/// Changes the handle of the person who asks. The one they had stays theirs: nobody else can take
/// it, and the old texts keep naming them, but a new text that uses it names nobody. A person
/// changes their handle once in `CHANGE_EVERY_DAYS` days.
///
/// # Errors
/// `request.invalid` for a text that is no handle (it has a capital, a space, an `@`, a letter
/// that is not ASCII, or is too long), `name.taken` when somebody has it or had it, or when it is
/// the handle of a group or a role, `handle.changed_lately` when the person changed it a short
/// time ago, `rate_limit.exceeded` for somebody who keeps asking for handles that are taken.
///
/// What is said about a handle is judged in this order: the days first (a person who has to wait
/// learns nothing), then the tries (a person has a few refused tries and then waits), then whether
/// the handle is taken. The one try left, once for each change, tells a person who may change their
/// handle that a handle is taken (12-security, accepted).
pub async fn change_handle<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeHandle,
) -> Result<Authorized<MyHandle>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let handle = input.handle;
    if !is_handle(&handle) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let person = actor.person.id;
    let failed = |error| internal::<S>(&error);
    // Asking for the handle one has is no change, and says nothing about anybody else.
    let before = ctx.store().handle_of_person(person).await.map_err(failed)?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    if before.as_deref() == Some(handle.as_str()) {
        return Ok(held.authorize(MyHandle { handle }));
    }
    let changed = ctx
        .store()
        .handle_changed_at(person)
        .await
        .map_err(failed)?;
    if changed_lately(changed, ctx.now()) {
        return Err(ErrorCode::HandleChangedLately.into());
    }
    let slot = ctx.reserve_for_person("handle", person)?;
    let owner = ctx
        .store()
        .person_with_handle(&handle)
        .await
        .map_err(failed)?;
    if owner.is_some_and(|owner| owner != person) || ctx.reserved_handles().await?.contains(&handle)
    {
        // The refused try stays counted: nothing gives it back.
        return Err(ErrorCode::NameTaken.into());
    }
    // A handle that is free is no probe.
    ctx.slot_succeeded(slot);
    let event = ctx.event(
        by(person),
        entity(RecordKind::Person, person),
        vec![FieldChange {
            field: member::handle::FIELD.to_owned(),
            before: before.map(|before| json!(before)),
            after: Some(json!(handle)),
        }],
    )?;
    let (now, put) = (ctx.now(), handle.clone());
    let why = Why::default();
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // Judged inside the change, under a lock: two changes of one person at the same
                // moment cannot both pass.
                let changed = S::handle_changed_in(transaction, person).await?;
                if changed_lately(changed, now) {
                    return Err(stopped.stop::<S>(ErrorCode::HandleChangedLately));
                }
                // Somebody may have taken it since it was looked at: the store refuses the same
                // handle twice.
                S::put_handle(transaction, person, &put, now).await
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NameTaken))?;
    Ok(held.authorize(MyHandle { handle }))
}

/// Makes the handle of every person who has none, one change for each person. The people made
/// before handles existed are the only ones; a person made since has one from the start. Safe to
/// run again. Returns how many it made.
///
/// # Errors
/// The error of the store.
pub async fn create_missing_handles<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    _input: CreateMissingHandles,
) -> Result<u64, AppError> {
    let mut made = 0_u64;
    let reserved = ctx.reserved_handles().await?;
    loop {
        let people = ctx
            .store()
            .people_without_handle(MISSING_AT_ONCE)
            .await
            .map_err(|error| internal::<S>(&error))?;
        if people.is_empty() {
            return Ok(made);
        }
        for person in people {
            let event = ctx.event(
                EventActor::System,
                entity(RecordKind::Person, person.id),
                vec![FieldChange {
                    field: member::handle::FIELD.to_owned(),
                    before: None,
                    after: None,
                }],
            )?;
            let (now, reserved) = (ctx.now(), reserved.clone());
            ctx.store()
                .record_change(&event, move |transaction| {
                    Box::pin(
                        async move { give_handle::<S>(transaction, &person, now, &reserved).await },
                    )
                })
                .await
                .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
            made += 1;
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Refuses the name of a group or a role whose handle a person has or had: a mention of the
    /// handle would name two things. A person gets no handle that a group or a role has either
    /// (`change_handle`, `give_handle`), so this is the other half of the same rule.
    ///
    /// # Errors
    /// `name.taken` when a person holds the handle of the name.
    pub(crate) async fn refuse_a_persons_handle(&self, name: &str) -> Result<(), AppError> {
        let Some(handle) = bb_core::comment::handle_of(name) else {
            return Ok(());
        };
        let held = self
            .store()
            .person_with_handle(&handle)
            .await
            .map_err(|error| internal::<S>(&error))?;
        if held.is_some() {
            return Err(ErrorCode::NameTaken.into());
        }
        Ok(())
    }
}
