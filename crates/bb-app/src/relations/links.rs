//! Adding and taking away a relation between two issues (ISS-04). A relation is stored once and
//! shown on both issues, each in its own words; both histories record it (the history reads the
//! two issues from the event). The person has to see both issues, so a relation to a hidden issue
//! cannot be made or taken away by someone who does not know the issue; they need the right to
//! change the issue they start from.

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::RelationId;
use bb_core::relation::{Direction, RELATIONS_MAX, Relation};
use bb_policy::Authorized;
use serde_json::json;

use super::types::{AddRelation, RelationView, RemoveRelation};
use crate::auth::{AppError, AuthStore, Ctx, Done, by, entity, internal};
use crate::member::relation as member;
use crate::projects::Why;

/// What a relation says in the event, as the ID of each thing it names.
fn named(field: &str, was: Option<String>, now: Option<String>) -> FieldChange {
    FieldChange {
        field: field.to_owned(),
        before: was.map(|text| json!(text)),
        after: now.map(|text| json!(text)),
    }
}

/// The three members of the event of a relation, as they were added (`adding`) or as they were
/// taken away.
fn relation_changes(relation: &Relation, adding: bool) -> Vec<FieldChange> {
    let shown = |text: String| {
        if adding {
            (None, Some(text))
        } else {
            (Some(text), None)
        }
    };
    [
        (member::KIND, relation.kind.to_string()),
        (member::FROM, relation.from.to_string()),
        (member::TO, relation.to.to_string()),
    ]
    .into_iter()
    .map(|(field, text)| {
        let (was, now) = shown(text);
        named(field, was, now)
    })
    .collect()
}

/// Adds a relation: `issue` reads one name of the kind and `other` the other one, as `direction`
/// says.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist or that the person may not see, or a kind
/// that is not there, `access.forbidden` without the right to change `issue`, `request.invalid`
/// for the same issue twice, a retired kind or an issue that has too many relations,
/// `relation.exists` for a relation that is there already.
pub async fn add_relation<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AddRelation,
) -> Result<Authorized<RelationView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (first, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let (second, _) = ctx.visible_issue(&actor, &input.other).await?;
    if !held.may_change_issue(first.reported_by) {
        return Err(ErrorCode::Forbidden.into());
    }
    if first.id == second.id {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let kinds = ctx
        .store()
        .relation_kinds()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let kind = kinds
        .iter()
        .find(|found| found.id == input.kind)
        .ok_or(AppError::Refused(ErrorCode::NotFound))?
        .clone();
    if kind.retired_at.is_some() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let (from, to) = match input.direction {
        Direction::Forward => (first.id, second.id),
        Direction::Backward => (second.id, first.id),
    };
    let relation = Relation {
        id: RelationId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        kind: kind.id,
        from,
        to,
        added_by: actor.person.id,
        added_at: ctx.now(),
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Relation, relation.id),
        relation_changes(&relation, true),
    )?;
    let why = Why::default();
    let (stopped, stored, symmetric) = (why.clone(), relation.clone(), kind.is_symmetric());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_issue_relations(transaction).await?;
                // Judged again on what is stored now: the same relation may have been added meanwhile
                // (the unique key stops two at the same moment), and a kind that reads the same from
                // both ends is one relation whichever issue it starts from.
                let there =
                    S::relation_between_in(transaction, stored.kind, stored.from, stored.to)
                        .await?
                        .is_some()
                        || (symmetric
                            && S::relation_between_in(
                                transaction,
                                stored.kind,
                                stored.to,
                                stored.from,
                            )
                            .await?
                            .is_some());
                if there {
                    return Err(stopped.stop::<S>(ErrorCode::RelationExists));
                }
                // An issue holds only so many relations, counted on what is stored now and with
                // every relation, also those to issues the person may not see: it is a limit of
                // storage, and the refusal names no issue.
                for issue in [stored.from, stored.to] {
                    if S::relations_of_in(transaction, issue).await?.len() >= RELATIONS_MAX {
                        return Err(stopped.stop::<S>(ErrorCode::LimitReached));
                    }
                }
                S::insert_relation(transaction, &stored).await
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::RelationExists))?;
    let mut maker = ctx.line_maker().await?;
    let line = ctx
        .line_of(&actor, &mut maker, &second)
        .await?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    Ok(held.authorize(RelationView {
        id: relation.id,
        kind: kind.id,
        direction: input.direction,
        name: kind.name(input.direction).to_owned(),
        issue: line,
    }))
}

/// Takes a relation away. The person has to see both issues and may change one of them.
///
/// # Errors
/// `resource.not_found` for a relation that does not exist or whose issues the person may not
/// both see, `access.forbidden` without the right to change either issue.
pub async fn remove_relation<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RemoveRelation,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let relation = ctx
        .store()
        .relation(input.relation)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let (from, held_from) = ctx
        .visible_issue(&actor, &relation.from.to_string())
        .await?;
    let (to, held_to) = ctx.visible_issue(&actor, &relation.to.to_string()).await?;
    if !held_from.may_change_issue(from.reported_by) && !held_to.may_change_issue(to.reported_by) {
        return Err(ErrorCode::Forbidden.into());
    }
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Relation, relation.id),
        relation_changes(&relation, false),
    )?;
    let (id, why) = (relation.id, Why::default());
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::delete_relation(transaction, id).await? {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::NotFound))
                }
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NotFound))?;
    Ok(Done)
}
