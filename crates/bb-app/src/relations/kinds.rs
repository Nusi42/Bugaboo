//! Kinds of relation (ISS-04): the list, making one, changing one, and the four of the first start.
//!
//! Kinds are the same in every project. Changing them needs *Manage issue types, kinds of relation
//! and layouts* for the whole organization (M3-Q6). A kind is never deleted: it is retired, so no
//! new relations have it while the old ones keep it. A name is used once, without regard to
//! capitals, in either direction of any other kind; that is judged inside the change, after the
//! policy lock, so two people who make the same name at the same moment cannot both succeed.

use bb_core::ErrorCode;
use bb_core::event::{Actor, FieldChange, RecordKind};
use bb_core::id::RelationKindId;
use bb_core::message::Message;
use bb_core::paging::window;
use bb_core::relation::{RelationKind, STARTING_RELATION_KINDS};
use bb_core::version::{FIRST_VERSION, counted_up, next};
use bb_policy::{Authorized, Permission, Place};
use jiff::Timestamp;
use serde_json::json;

use super::types::{
    ChangeRelationKind, MakeRelationKind, MakeStartingRelationKinds, RelationKindPage,
    RelationKindView, RelationKindsQuery,
};
use crate::auth::{
    AppError, AuthStore, Ctx, Stops, by, change_failed, entity, fixed_entity, internal, set_to,
};
use crate::input::clean_name;
use crate::member::relation_kind as member;
use crate::projects::Why;

/// Whether a name of the pair is one that another kind has already, in either direction.
fn name_taken(kinds: &[RelationKind], names: [&str; 2], except: Option<RelationKindId>) -> bool {
    kinds
        .iter()
        .filter(|found| Some(found.id) != except)
        .any(|found| {
            names.iter().any(|name| {
                let lower = name.to_lowercase();
                found.forward.to_lowercase() == lower || found.backward.to_lowercase() == lower
            })
        })
}

/// Lists the kinds of relation in the order they are offered in, retired ones too. Any signed-in
/// person may: the names of the kinds are not hidden from anybody who works here.
///
/// # Errors
/// `request.invalid` for a cursor that is not one.
pub async fn relation_kinds<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RelationKindsQuery,
) -> Result<Authorized<RelationKindPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let kinds = ctx
        .store()
        .relation_kinds()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let page = window(kinds.len(), input.cursor.as_deref(), input.limit)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    Ok(held.authorize(RelationKindPage {
        items: kinds[page.start..page.end]
            .iter()
            .map(RelationKindView::from)
            .collect(),
        next_cursor: page.next,
    }))
}

/// Makes a kind of relation, at the end of the order.
///
/// # Errors
/// `access.forbidden` without *Manage issue types, kinds of relation and layouts* for the whole
/// organization, `request.invalid` for a name that cannot be one, `name.taken` for a name another
/// kind has.
pub async fn make_relation_kind<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MakeRelationKind,
) -> Result<Authorized<RelationKindView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx
        .require(&actor, Permission::ManageIssueTypes, Place::Organization)
        .await?;
    let (forward, backward) = (clean_name(&input.forward)?, clean_name(&input.backward)?);
    let kinds = ctx
        .store()
        .relation_kinds()
        .await
        .map_err(|error| internal::<S>(&error))?;
    if name_taken(&kinds, [&forward, &backward], None) {
        return Err(ErrorCode::NameTaken.into());
    }
    let kind = RelationKind {
        id: RelationKindId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        forward,
        backward,
        position: kinds
            .iter()
            .map(|found| found.position)
            .max()
            .map_or(0, |last| last + 1),
        retired_at: None,
        version: FIRST_VERSION,
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::RelationKind, kind.id),
        vec![
            set_to(member::FORWARD, json!(kind.forward)),
            set_to(member::BACKWARD, json!(kind.backward)),
        ],
    )?;
    let why = Why::default();
    let (stopped, stored) = (why.clone(), kind.clone());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                let kinds = S::relation_kinds_in(transaction).await?;
                if name_taken(&kinds, [&stored.forward, &stored.backward], None) {
                    return Err(stopped.stop::<S>(ErrorCode::NameTaken));
                }
                S::insert_relation_kind(transaction, &stored).await
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NameTaken))?;
    Ok(held.authorize(RelationKindView::from(&kind)))
}

/// What a change of a kind changes, before and after, for the event.
fn differences(
    current: &RelationKind,
    forward: &str,
    backward: &str,
    retired_at: Option<Timestamp>,
) -> Vec<FieldChange> {
    let mut differences = Vec::new();
    if forward != current.forward {
        differences.push(FieldChange {
            field: member::FORWARD.to_owned(),
            before: Some(json!(current.forward)),
            after: Some(json!(forward)),
        });
    }
    if backward != current.backward {
        differences.push(FieldChange {
            field: member::BACKWARD.to_owned(),
            before: Some(json!(current.backward)),
            after: Some(json!(backward)),
        });
    }
    if retired_at.is_some() != current.retired_at.is_some() {
        differences.push(FieldChange {
            field: member::RETIRED.to_owned(),
            before: Some(json!(current.retired_at.is_some())),
            after: Some(json!(retired_at.is_some())),
        });
    }
    differences
}

/// Changes the names or the retired state of a kind: what is `Some` is changed.
///
/// # Errors
/// `access.forbidden` without the permission, `resource.not_found` for a kind that is not there,
/// `request.invalid` for a name that cannot be one or when nothing is asked, `name.taken` for a
/// name another kind has, `edit.version_changed` when somebody changed the kind since it was
/// looked at.
pub async fn change_relation_kind<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeRelationKind,
) -> Result<Authorized<RelationKindView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx
        .require(&actor, Permission::ManageIssueTypes, Place::Organization)
        .await?;
    if input.forward.is_none() && input.backward.is_none() && input.retired.is_none() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let named = |text: &Option<String>| text.as_deref().map(clean_name).transpose();
    let (forward, backward) = (named(&input.forward)?, named(&input.backward)?);
    let kinds = ctx
        .store()
        .relation_kinds()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let current = kinds
        .iter()
        .find(|found| found.id == input.kind)
        .ok_or(AppError::Refused(ErrorCode::NotFound))?
        .clone();
    let record = entity(RecordKind::RelationKind, current.id);
    if current.version != input.version {
        return Err(AppError::stale(
            record,
            Place::Organization,
            actor.person.id,
        ));
    }
    let forward = forward.unwrap_or_else(|| current.forward.clone());
    let backward = backward.unwrap_or_else(|| current.backward.clone());
    if name_taken(&kinds, [&forward, &backward], Some(current.id)) {
        return Err(ErrorCode::NameTaken.into());
    }
    let now = ctx.now();
    let retired_at = match input.retired {
        None => current.retired_at,
        Some(true) => current.retired_at.or(Some(now)),
        Some(false) => None,
    };
    let mut differences = differences(&current, &forward, &backward, retired_at);
    if differences.is_empty() {
        return Ok(held.authorize(RelationKindView::from(&current)));
    }
    let after = RelationKind {
        forward,
        backward,
        retired_at,
        version: next(current.version),
        ..current.clone()
    };
    differences.push(counted_up(current.version));
    let event = ctx.event(by(actor.person.id), record.clone(), differences)?;
    let why = Why::default();
    let (stopped, written, version) = (why.clone(), after.clone(), current.version);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                let kinds = S::relation_kinds_in(transaction).await?;
                if name_taken(
                    &kinds,
                    [&written.forward, &written.backward],
                    Some(written.id),
                ) {
                    return Err(stopped.stop::<S>(ErrorCode::NameTaken));
                }
                let updated = S::update_relation_kind(
                    transaction,
                    written.id,
                    &written.forward,
                    &written.backward,
                    written.retired_at,
                    version,
                )
                .await?;
                if updated {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::VersionChanged))
                }
            })
        })
        .await
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                &record,
                Place::Organization,
                actor.person.id,
            )
        })?;
    Ok(held.authorize(RelationKindView::from(&after)))
}

/// Makes the kinds of relation of the first start if there are none yet: *blocks, duplicate of,
/// causes, related to*, from the one definition in `bb-core`, with an event. It runs at every
/// start and does only what is still to do, so it can be stopped half way and run again.
///
/// # Errors
/// Fails when the store fails.
pub async fn make_starting_relation_kinds<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    _input: MakeStartingRelationKinds,
) -> Result<(), AppError> {
    let existing = ctx
        .store()
        .relation_kinds()
        .await
        .map_err(|error| internal::<S>(&error))?;
    if !existing.is_empty() {
        return Ok(());
    }
    let mut kinds = Vec::with_capacity(STARTING_RELATION_KINDS.len());
    for (position, starting) in (0_i64..).zip(STARTING_RELATION_KINDS) {
        kinds.push(RelationKind {
            id: RelationKindId::generate(&mut ctx.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?,
            forward: Message::from(starting.forward).render(),
            backward: Message::from(starting.backward).render(),
            position,
            retired_at: None,
            version: FIRST_VERSION,
        });
    }
    let names: Vec<serde_json::Value> = kinds
        .iter()
        .map(|found| json!([found.forward, found.backward]))
        .collect();
    let event = ctx.event(
        Actor::System,
        fixed_entity(RecordKind::RelationKind, "starting")?,
        vec![set_to(member::STARTING, serde_json::Value::Array(names))],
    )?;
    let result = ctx
        .store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                // Two servers that start together both saw "none" above; the one that gets the
                // lock second finds them made and stops, which is not a failure.
                if !S::relation_kinds_in(transaction).await?.is_empty() {
                    return Err(S::aborted());
                }
                for kind in &kinds {
                    S::insert_relation_kind(transaction, kind).await?;
                }
                Ok(())
            })
        })
        .await;
    match result {
        Err(error) if S::is_aborted(&error) => Ok(()),
        Err(error) => Err(change_failed::<S>(&error, Stops::NONE)),
        Ok(()) => Ok(()),
    }
}
