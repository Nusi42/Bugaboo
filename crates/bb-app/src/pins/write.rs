//! Pinning, unpinning and ordering (CFG-05, ADR-0050, decision 12). Every change runs under the lock
//! of the person's pins and counts them in the same change, so two at once cannot pass the limit.

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::pin::{PINS_MAX, PinKind, PinTarget, holds, next_place};
use serde_json::{Value, json};

use super::types::{OrderPins, PinRequest};
use crate::auth::{AppError, AuthStore, Ctx, Done, Stops, by, change_failed, entity};
use crate::member::pin as member;
use crate::projects::Why;

/// The target a request names, when its ID is one of the kind it names.
fn target_of(kind: PinKind, text: &str) -> Option<PinTarget> {
    let id = text.parse().ok()?;
    PinTarget::of(kind, id)
}

/// A target as one value of an event.
fn target_json(target: PinTarget) -> Value {
    json!({"kind": target.kind().code(), "target": target.record().to_string()})
}

/// Pins a project or a saved search that the person may open to the end of their pins. Pinning what
/// is pinned changes nothing and writes no event.
///
/// # Errors
/// `resource.not-found` for something that is not there or that the person may not open,
/// `limit.reached` when they have as many pins as they may have.
pub async fn pin_to_sidebar<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PinRequest,
) -> Result<Done, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let target = target_of(input.kind, &input.target).ok_or(ErrorCode::NotFound)?;
    if ctx.viewed(&actor, &[target]).await?.remove(0).is_none() {
        return Err(ErrorCode::NotFound.into());
    }
    let person = actor.person.id;
    let pinned = ctx.pins_of_actor(&actor).await?;
    if holds(&pinned, target).is_some() {
        return Ok(Done);
    }
    let event = ctx.event(
        by(person),
        entity(RecordKind::Pin, person),
        vec![FieldChange {
            field: member::PIN.to_owned(),
            before: None,
            after: Some(target_json(target)),
        }],
    )?;
    let why = Why::default();
    let stop = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_pins_of(transaction, person).await?;
                let have = S::pins_of_in(transaction, person).await?;
                // Pinned at the same moment by another request: nothing left to do.
                if holds(&have, target).is_some() {
                    return Ok(());
                }
                if have.len() >= PINS_MAX {
                    return Err(stop.stop::<S>(ErrorCode::LimitReached));
                }
                S::insert_pin(transaction, person, target, next_place(&have)).await
            })
        })
        .await
        .map_err(|error| why.said_or_failed::<S>(&error))?;
    Ok(Done)
}

/// Takes a pin away. Taking away what is not pinned changes nothing and writes no event.
///
/// # Errors
/// `auth.access-key-not-allowed` for an access key: pins are the person's own.
pub async fn unpin<S: AuthStore>(ctx: &Ctx<'_, S>, input: PinRequest) -> Result<Done, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let person = actor.person.id;
    let Some(target) = target_of(input.kind, &input.target) else {
        return Ok(Done);
    };
    let pinned = ctx.pins_of_actor(&actor).await?;
    if holds(&pinned, target).is_none() {
        return Ok(Done);
    }
    let event = ctx.event(
        by(person),
        entity(RecordKind::Pin, person),
        vec![FieldChange {
            field: member::PIN.to_owned(),
            before: Some(target_json(target)),
            after: None,
        }],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_pins_of(transaction, person).await?;
                S::delete_pin(transaction, person, target).await.map(|_| ())
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    Ok(Done)
}

/// Puts the pins named in the order given: they take, in that order, the places the same pins held,
/// so the pins the person does not see (and so cannot name) keep theirs.
///
/// # Errors
/// `request.invalid` for a pin named twice or one the person did not pin.
pub async fn order_pins<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: OrderPins,
) -> Result<Done, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let person = actor.person.id;
    let named: Vec<PinTarget> = input
        .pins
        .iter()
        .map(|pin| target_of(pin.kind, &pin.target).ok_or(ErrorCode::InvalidRequest))
        .collect::<Result<_, _>>()?;
    // Only what the person sees can be named: a pin to something they may no longer open counts as
    // not pinned, so a refused order tells nothing of why (ADR-0050, decision 12).
    if ctx
        .viewed(&actor, &named)
        .await?
        .iter()
        .any(Option::is_none)
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let event = ctx.event(
        by(person),
        entity(RecordKind::Pin, person),
        vec![FieldChange {
            field: member::ORDER.to_owned(),
            before: None,
            after: Some(json!(
                named.iter().copied().map(target_json).collect::<Vec<_>>()
            )),
        }],
    )?;
    let why = Why::default();
    let stop = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_pins_of(transaction, person).await?;
                let have = S::pins_of_in(transaction, person).await?;
                let mut places = Vec::with_capacity(named.len());
                for target in &named {
                    match holds(&have, *target) {
                        Some(pin) if !places.iter().any(|(each, _)| each == target) => {
                            places.push((*target, pin.place));
                        }
                        _ => return Err(stop.stop::<S>(ErrorCode::InvalidRequest)),
                    }
                }
                let mut slots: Vec<i64> = places.iter().map(|(_, place)| *place).collect();
                slots.sort_unstable();
                let moved: Vec<(PinTarget, i64)> = named.iter().copied().zip(slots).collect();
                S::put_pin_places(transaction, person, &moved).await
            })
        })
        .await
        .map_err(|error| why.said_or_failed::<S>(&error))?;
    Ok(Done)
}
