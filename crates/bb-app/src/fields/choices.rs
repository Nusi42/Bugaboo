//! The choices of a *Pick one* or *Pick several* field (CF-04, CF-07): add, change, reorder,
//! *Combine with another choice* and *Retire*.
//!
//! The choices belong to their field: every change counts the version of the field up and is made
//! on the version the person looked at. *Combine with another choice* moves every value in one
//! change and writes a record into the history of every item it touches (CF-07).

use std::collections::{HashMap, HashSet};

use bb_core::ErrorCode;
use bb_core::event::{EntityKind, EntityRef, FieldChange, RecordKind};
use bb_core::fields::FieldKey;
use bb_core::id::{ChoiceId, EntityId, FieldId, UserId};
use bb_core::issue::IssueIcon;
use bb_core::own_fields::{CHOICES_MAX, ChoiceColor, Edit, FieldChoice, FieldType};
use bb_core::own_fields_value::FieldValue;
use bb_core::version::{counted_up, next};
use bb_policy::{Authorized, Permission, Place};
use jiff::Timestamp;
use serde_json::{Value, json};

use super::member;
use super::types::{
    AddChoice, ChangeChoice, CombineChoices, Definition, FieldView, ReorderChoices,
};
use crate::auth::{AppError, AuthStore, Ctx, EventMaker, by, entity, set_to};
use crate::input::clean_name;
use crate::ports::{BoxFuture, ChangeStore};
use crate::projects::Why;

/// The name of the member of the event of a choice that changed: `choice-changed.<id>.name`.
fn changed_member(choice: ChoiceId, part: &str) -> String {
    format!("{}.{choice}.{part}", member::CHOICE_CHANGED)
}

/// Whether another choice than `except` has the name, without regard to capitals.
fn name_taken(choices: &[FieldChoice], name: &str, except: Option<ChoiceId>) -> bool {
    let lower = name.to_lowercase();
    choices
        .iter()
        .any(|found| Some(found.id) != except && found.name.to_lowercase() == lower)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The field with choices that the person manages, at the version they looked at.
    async fn field_to_manage(
        &self,
        actor: &crate::access::Actor,
        field: FieldId,
        version: i64,
    ) -> Result<(std::sync::Arc<bb_policy::Evaluation>, Definition), AppError> {
        let held = self
            .require(actor, Permission::ManageFields, Place::Organization)
            .await?;
        let definition = self
            .field_catalogue()
            .await?
            .iter()
            .find(|found| found.field.id == field)
            .cloned()
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        if !definition.field.kind.has_choices() {
            return Err(ErrorCode::InvalidRequest.into());
        }
        if definition.field.version != version {
            return Err(AppError::stale(
                entity(RecordKind::Field, field),
                Place::Organization,
                actor.person.id,
            ));
        }
        Ok((held, definition))
    }

    /// The field after a change of its choices, as the person sees it.
    async fn field_after(
        &self,
        actor: &crate::access::Actor,
        held: &bb_policy::Evaluation,
        mut definition: Definition,
        counted: bool,
    ) -> Result<FieldView, AppError> {
        if counted {
            definition.field.version = next(definition.field.version);
        }
        definition
            .choices
            .sort_by_key(|choice| (choice.position, choice.id));
        self.field_shown(actor, held, &definition).await
    }
}

/// The write of a change of choices, which the command hands to `record_change`: it waits for the
/// policy lock, counts the version of the field up from the one that was looked at, and runs
/// `write`.
type Write<S> = Box<
    dyn for<'t> FnOnce(
            &'t mut <S as ChangeStore>::Transaction,
        ) -> BoxFuture<'t, Result<(), <S as ChangeStore>::Error>>
        + Send,
>;

/// Guards a write of choices (see [`Write`]). The `Why` tells afterwards why a change stopped.
fn guarded<S, W>(field: FieldId, version: i64, write: W) -> (Why, Write<S>)
where
    S: AuthStore,
    W: for<'t> FnOnce(&'t mut S::Transaction, Why) -> BoxFuture<'t, Result<(), S::Error>>
        + Send
        + 'static,
{
    let why = Why::default();
    let stopped = why.clone();
    let run: Write<S> = Box::new(move |transaction| {
        Box::pin(async move {
            S::lock_policy(transaction).await?;
            if !S::bump_field(transaction, field, version).await? {
                return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
            }
            write(transaction, stopped).await
        })
    });
    (why, run)
}

/// The answer for a change of choices that did not go through.
fn refused<S: AuthStore>(
    why: &Why,
    error: &S::Error,
    record: &EntityRef,
    person: UserId,
) -> AppError {
    why.answer::<S>(error, ErrorCode::NameTaken)
        .about(record, Place::Organization, person)
}

/// Adds a choice at the end.
///
/// # Errors
/// `access.forbidden` without *Manage fields*, `resource.not_found` for a field that is not there,
/// `request.invalid` for a field without choices or a name that cannot be one, `name.taken` for a
/// name another choice has, `edit.version_changed` when somebody changed the field meanwhile.
pub async fn add_choice<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AddChoice,
) -> Result<Authorized<FieldView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (held, mut definition) = ctx
        .field_to_manage(&actor, input.field, input.version)
        .await?;
    let name = clean_name(&input.name)?;
    if name_taken(&definition.choices, &name, None) {
        return Err(ErrorCode::NameTaken.into());
    }
    if definition.choices.len() >= CHOICES_MAX {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let choice = FieldChoice {
        id: ChoiceId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        field: input.field,
        name,
        color: input
            .color
            .unwrap_or_else(|| ChoiceColor::nth(definition.choices.len())),
        icon: input.icon,
        position: definition
            .choices
            .iter()
            .map(|found| found.position)
            .max()
            .map_or(0, |last| last + 1),
        retired_at: None,
    };
    let record = entity(RecordKind::Field, input.field);
    let event = ctx.event(
        by(actor.person.id),
        record.clone(),
        vec![
            set_to(
                member::CHOICE_ADDED,
                json!({ "id": choice.id, "name": choice.name, "color": choice.color.code() }),
            ),
            counted_up(input.version),
        ],
    )?;
    let stored = choice.clone();
    let (why, write) = guarded::<S, _>(input.field, input.version, move |transaction, stopped| {
        Box::pin(async move {
            let known = S::field_definitions_in(transaction).await?;
            let taken = known
                .iter()
                .find(|found| found.field.id == stored.field)
                .is_some_and(|found| name_taken(&found.choices, &stored.name, None));
            if taken {
                return Err(stopped.stop::<S>(ErrorCode::NameTaken));
            }
            S::insert_choice(transaction, &stored).await
        })
    });
    ctx.store()
        .record_change(&event, write)
        .await
        .map_err(|error| refused::<S>(&why, &error, &record, actor.person.id))?;
    definition.choices.push(choice);
    Ok(held.authorize(ctx.field_after(&actor, &held, definition, true).await?))
}

/// Changes the name, color, icon or retired state of a choice: what is `Some` is changed.
///
/// # Errors
/// As [`add_choice`], and `resource.not_found` for a choice the field does not have.
pub async fn change_choice<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeChoice,
) -> Result<Authorized<FieldView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    if input.name.is_none()
        && input.color.is_none()
        && input.icon.is_keep()
        && input.retired.is_none()
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let (held, mut definition) = ctx
        .field_to_manage(&actor, input.field, input.version)
        .await?;
    let position = definition
        .choices
        .iter()
        .position(|found| found.id == input.choice)
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let before = definition.choices[position].clone();
    let mut after = before.clone();
    if let Some(name) = &input.name {
        after.name = clean_name(name)?;
        if name_taken(&definition.choices, &after.name, Some(after.id)) {
            return Err(ErrorCode::NameTaken.into());
        }
    }
    if let Some(color) = input.color {
        after.color = color;
    }
    match input.icon {
        Edit::Keep => {}
        Edit::Clear => after.icon = None,
        Edit::Set(icon) => after.icon = Some(icon),
    }
    if let Some(retired) = input.retired {
        after.retired_at = if retired {
            before.retired_at.or(Some(ctx.now()))
        } else {
            None
        };
    }
    let mut changes: Vec<FieldChange> = Vec::new();
    let mut add = |part: &str, was: Value, now: Value| {
        if was != now {
            changes.push(FieldChange {
                field: changed_member(after.id, part),
                before: Some(was),
                after: Some(now),
            });
        }
    };
    add(member::choice::NAME, json!(before.name), json!(after.name));
    add(
        member::choice::COLOR,
        json!(before.color.code()),
        json!(after.color.code()),
    );
    add(
        member::choice::ICON,
        json!(before.icon.map(IssueIcon::code)),
        json!(after.icon.map(IssueIcon::code)),
    );
    add(
        member::choice::RETIRED,
        json!(before.retired_at.is_some()),
        json!(after.retired_at.is_some()),
    );
    if changes.is_empty() {
        return Ok(held.authorize(ctx.field_after(&actor, &held, definition, false).await?));
    }
    changes.push(counted_up(input.version));
    let record = entity(RecordKind::Field, input.field);
    let event = ctx.event(by(actor.person.id), record.clone(), changes)?;
    let stored = after.clone();
    let (why, write) = guarded::<S, _>(input.field, input.version, move |transaction, stopped| {
        Box::pin(async move {
            let known = S::field_definitions_in(transaction).await?;
            let taken = known
                .iter()
                .find(|found| found.field.id == stored.field)
                .is_some_and(|found| name_taken(&found.choices, &stored.name, Some(stored.id)));
            if taken {
                return Err(stopped.stop::<S>(ErrorCode::NameTaken));
            }
            S::update_choice(transaction, &stored).await
        })
    });
    ctx.store()
        .record_change(&event, write)
        .await
        .map_err(|error| refused::<S>(&why, &error, &record, actor.person.id))?;
    definition.choices[position] = after;
    Ok(held.authorize(ctx.field_after(&actor, &held, definition, true).await?))
}

/// Puts the choices in a new order: `order` names every choice of the field once.
///
/// # Errors
/// As [`add_choice`], and `request.invalid` when `order` is not every choice once.
pub async fn reorder_choices<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ReorderChoices,
) -> Result<Authorized<FieldView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (held, mut definition) = ctx
        .field_to_manage(&actor, input.field, input.version)
        .await?;
    let mut known: Vec<ChoiceId> = definition.choices.iter().map(|choice| choice.id).collect();
    let before = known.clone();
    let mut asked = input.order.clone();
    known.sort();
    asked.sort();
    if known != asked || asked.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if before == input.order {
        return Ok(held.authorize(ctx.field_after(&actor, &held, definition, false).await?));
    }
    let record = entity(RecordKind::Field, input.field);
    let event = ctx.event(
        by(actor.person.id),
        record.clone(),
        vec![
            FieldChange {
                field: member::CHOICES_ORDER.to_owned(),
                before: Some(json!(before)),
                after: Some(json!(input.order)),
            },
            counted_up(input.version),
        ],
    )?;
    let (field, order) = (input.field, input.order.clone());
    let (why, write) = guarded::<S, _>(input.field, input.version, move |transaction, _stopped| {
        Box::pin(async move { S::order_choices(transaction, field, &order).await })
    });
    ctx.store()
        .record_change(&event, write)
        .await
        .map_err(|error| refused::<S>(&why, &error, &record, actor.person.id))?;
    for choice in &mut definition.choices {
        if let Some(at) = input.order.iter().position(|id| *id == choice.id) {
            choice.position = i64::try_from(at).unwrap_or(i64::MAX);
        }
    }
    Ok(held.authorize(ctx.field_after(&actor, &held, definition, true).await?))
}

/// What a combine of two choices moves, for the history.
struct Combined<'a> {
    key: &'a str,
    kind: FieldType,
    field: FieldId,
    from: ChoiceId,
    into: ChoiceId,
    at: Timestamp,
}

/// Moves every value of a choice to another one, inside the change of the field, and appends a
/// record to the history of every item it touched (and counts its version up). The reads and the
/// moves are one for each kind of item, never one for each item; the events are one each.
async fn record_combined<S: AuthStore>(
    transaction: &mut S::Transaction,
    events: &EventMaker,
    args: Combined<'_>,
) -> Result<(), S::Error> {
    // What each item held, to say what changed in its history: one read for each kind of
    // item before and one after.
    let holders = S::items_with_choice_in(transaction, args.field, args.from).await?;
    let held_by = |kind: &EntityKind| -> Vec<EntityId> {
        holders
            .iter()
            .filter(|(each, _)| each == kind)
            .map(|(_, item)| *item)
            .collect()
    };
    let kinds: Vec<EntityKind> = {
        let mut kinds: Vec<EntityKind> = Vec::new();
        for (kind, _) in &holders {
            if !kinds.contains(kind) {
                kinds.push(kind.clone());
            }
        }
        kinds
    };
    let mut before = HashMap::new();
    for kind in &kinds {
        for stored in S::field_values_in(transaction, kind.clone(), held_by(kind)).await? {
            if stored.field == args.field {
                before.insert(
                    stored.entity,
                    FieldValue::from_cells(args.kind, &stored.cells),
                );
            }
        }
    }
    let touched: HashSet<EntityId> =
        S::combine_choice_values(transaction, args.field, args.from, args.into)
            .await?
            .into_iter()
            .map(|(_, item)| item)
            .collect();
    let mut after = HashMap::new();
    for kind in &kinds {
        let items: Vec<EntityId> = held_by(kind)
            .into_iter()
            .filter(|item| touched.contains(item))
            .collect();
        for stored in S::field_values_in(transaction, kind.clone(), items).await? {
            if stored.field == args.field {
                after.insert(
                    stored.entity,
                    FieldValue::from_cells(args.kind, &stored.cells),
                );
            }
        }
    }
    for (item_kind, item) in holders {
        if !touched.contains(&item) {
            continue;
        }
        let version = S::touch_item(transaction, &item_kind, item, args.at).await?;
        let mut changes = vec![FieldChange {
            field: args.key.to_owned(),
            before: before
                .remove(&item)
                .flatten()
                .and_then(|value| value.to_json(args.kind)),
            after: after
                .remove(&item)
                .flatten()
                .and_then(|value| value.to_json(args.kind)),
        }];
        if let Some(version) = version {
            changes.push(counted_up(version));
        }
        let event = events
            .make(
                EntityRef {
                    kind: item_kind,
                    id: item,
                },
                changes,
            )
            .map_err(|_| S::aborted())?;
        S::append_follow_up_event(transaction, &event).await?;
    }
    Ok(())
}

/// Moves every value of a choice to another choice and retires the first one (*Combine with
/// another choice*). Every item it touches gets a record in its history and a new version.
///
/// # Errors
/// As [`add_choice`], and `request.invalid` for two choices that are the same, or a choice to
/// combine into that is retired.
pub async fn combine_choices<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CombineChoices,
) -> Result<Authorized<FieldView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (held, mut definition) = ctx
        .field_to_manage(&actor, input.field, input.version)
        .await?;
    let find = |id: ChoiceId| {
        definition
            .choices
            .iter()
            .find(|found| found.id == id)
            .cloned()
    };
    let from = find(input.choice).ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let into = find(input.into).ok_or(AppError::Refused(ErrorCode::NotFound))?;
    if from.id == into.id || into.retired_at.is_some() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let record = entity(RecordKind::Field, input.field);
    let event = ctx.event(
        by(actor.person.id),
        record.clone(),
        vec![
            FieldChange {
                field: member::CHOICE_COMBINED.to_owned(),
                before: Some(json!(from.id)),
                after: Some(json!(into.id)),
            },
            counted_up(input.version),
        ],
    )?;
    let (kind_of, field_id) = (definition.field.kind, input.field);
    let key = FieldKey::Own(field_id).code().into_owned();
    let events = ctx.event_maker(by(actor.person.id));
    let (retired_at, now) = (ctx.now(), ctx.now());
    let retired = FieldChoice {
        retired_at: Some(retired_at),
        ..from.clone()
    };
    let (from_id, into_id) = (from.id, into.id);
    let (why, write) = guarded::<S, _>(input.field, input.version, move |transaction, _stopped| {
        Box::pin(async move {
            record_combined::<S>(
                transaction,
                &events,
                Combined {
                    key: &key,
                    kind: kind_of,
                    field: field_id,
                    from: from_id,
                    into: into_id,
                    at: now,
                },
            )
            .await?;
            S::update_choice(transaction, &retired).await
        })
    });
    ctx.store()
        .record_change(&event, write)
        .await
        .map_err(|error| refused::<S>(&why, &error, &record, actor.person.id))?;
    if let Some(found) = definition
        .choices
        .iter_mut()
        .find(|found| found.id == from.id)
    {
        found.retired_at = Some(retired_at);
    }
    Ok(held.authorize(ctx.field_after(&actor, &held, definition, true).await?))
}
