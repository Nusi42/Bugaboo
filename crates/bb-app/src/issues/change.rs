//! Changing an issue (ISS-01): the fields the person may change, on the version they looked at.
//! The closed date follows the status group *Done* (ADR-0027, decision 5).

use bb_core::ErrorCode;
use bb_core::version::Version;

use bb_core::event::{EntityKind, FieldChange, RecordKind};
use bb_core::fields::Field;
use bb_core::given::SetTo;
use bb_core::id::{IssueId, IssueTypeId, ProjectId, UserId};
use bb_core::issue::{Issue, StatusGroup, assigned_rows};
use bb_core::plan::record::IssueTimeFactor;
use bb_core::version::next as next_version;
use bb_core::workflow::StatusKey;
use bb_policy::{Authorized, Evaluation, Place};
use serde_json::json;

use super::fields::{Given, clean_title, ensure_dates_in_order, ensure_may_change, member};
use super::types::{ChangeIssue, IssueView};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal};
use crate::fields::{Item, ItemValues, Making, PlannedValues};
use crate::follow::started_following;
use crate::offers::ensure_type_takes_issues_in;
use crate::plan::reset_with_first;
use crate::projects::Why;
use crate::projects::lock_chain_in;
use crate::workflows::{StatusCheck, ensure_written_status_in};
use std::sync::Arc;

/// What a change of an issue asks for, checked against what the person may do and what is
/// stored.
pub(crate) struct Plan {
    /// The issue as it is.
    pub(crate) current: Issue,
    /// The people it is assigned to now, in order.
    assigned_now: Vec<UserId>,
    /// The issue as it will be (the version is the one it has now).
    next: Issue,
    /// The people it will be assigned to.
    assigned_next: Vec<UserId>,
    /// The values of own fields the request sets.
    own: PlannedValues,
    /// The status group the issue enters, when its status changes: it decides the closed date.
    entering: Option<StatusGroup>,
    /// The time factor the issue has, which goes back to ×1 when the first person changes.
    factor: Option<IssueTimeFactor>,
}

impl Plan {
    /// The fields that change, before and after, for the event.
    pub(crate) fn field_changes(&self) -> Vec<FieldChange> {
        let (before, after) = (&self.current, &self.next);
        let mut changes = Vec::new();
        let mut add = |change: Option<FieldChange>| changes.extend(change);
        let difference = crate::event_members::difference;
        add(difference(
            member::ISSUE_TYPE,
            json!(before.issue_type.to_string()),
            json!(after.issue_type.to_string()),
        ));
        add(difference(
            member::TITLE,
            json!(before.title),
            json!(after.title),
        ));
        add(crate::event_members::status_member(
            &before.status,
            &after.status,
        ));
        add(difference(
            member::PRIORITY,
            json!(before.priority.code()),
            json!(after.priority.code()),
        ));
        add(difference(
            member::START_DATE,
            json!(before.start_date),
            json!(after.start_date),
        ));
        add(difference(
            member::DUE_DATE,
            json!(before.due_date),
            json!(after.due_date),
        ));
        add(difference(
            member::ESTIMATED_TIME,
            json!(before.estimated_time),
            json!(after.estimated_time),
        ));
        add(difference(
            member::TIME_LEFT,
            json!(before.time_left),
            json!(after.time_left),
        ));
        add(difference(
            member::ASSIGNED_TO,
            json!(
                self.assigned_now
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            ),
            json!(
                self.assigned_next
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            ),
        ));
        changes.extend(self.own.changes());
        changes.extend(reset_with_first(
            self.factor.as_ref(),
            &self.assigned_now,
            &self.assigned_next,
        ));
        changes
    }

    /// Whether the change takes the time factor back to ×1: the first assigned person changes.
    fn resets_factor(&self) -> bool {
        reset_with_first(
            self.factor.as_ref(),
            &self.assigned_now,
            &self.assigned_next,
        )
        .is_some()
    }
}

/// The fields of the catalogue that the request names, changed or not (`Given`). Each has to be
/// one the person may change: judging only the values that differ would answer 200 or 403
/// depending on the value a field has now, which tells a person who may not see the field what it
/// holds. Changing the issue type needs the right to change the issue (*Change issues*, or
/// *Change own issues* on an issue the person reported) and no field of its own.
fn named(input: &ChangeIssue) -> Vec<Field> {
    Given::from(input).fields()
}

/// Gives `next` the dates the request names (`SetTo::Nothing` takes one away) and refuses a start
/// after the due date it has then.
fn set_dates(next: &mut Issue, input: &ChangeIssue) -> Result<(), AppError> {
    next.start_date = input.start_date.map_or(next.start_date, SetTo::into_option);
    next.due_date = input.due_date.map_or(next.due_date, SetTo::into_option);
    ensure_dates_in_order(next)
}

/// A change of the status of an issue, with the issue as it will be.
struct StatusMove<'a> {
    input: &'a ChangeIssue,
    current: &'a Issue,
    next: &'a Issue,
    chain: &'a [ProjectId],
    assigned: &'a [UserId],
    own: &'a PlannedValues,
    stored: &'a ItemValues,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The status group the issue enters when its status changes, once the workflow allows the
    /// change (the one judge, ADR-0031); none while the status stays.
    async fn entering_group(
        &self,
        actor: &Actor,
        held: &Evaluation,
        change: &StatusMove<'_>,
    ) -> Result<Option<StatusGroup>, AppError> {
        let StatusMove {
            input,
            current,
            next,
            ..
        } = change;
        if next.status == current.status {
            return Ok(None);
        }
        let own_filled = change
            .own
            .filled_after(change.stored.filled_fields(current.id.into()));
        let given = Given::from(*input).keys(input.fields.keys());
        let check = StatusCheck {
            held,
            person: actor.person.id,
            chain: change.chain,
            issue_type: next.issue_type,
            reported_by: current.reported_by,
            // Whether the person chose the status or the workflow of a new type put the issue back
            // into one, it is a change of the status and needs the *Status* level of the type the
            // issue ends in.
            may_change_status: StatusCheck::may_change_status(
                held,
                next.issue_type,
                current.reported_by,
            ),
            assigned: change.assigned,
            description_filled: !current.description.is_empty(),
            own_filled: &own_filled,
            given: &given,
        };
        let checked = self
            .ensure_status_change(&check, &current.status, &next.status)
            .await?;
        Ok(Some(checked.group))
    }

    /// The status an issue gets from a request: the one it names, judged afterwards. Naming the
    /// status the issue has already is no status change: with a new type it is the same as naming
    /// none, and the issue keeps its status when the workflow that counts then has it, else starts
    /// again in the starting status (ADR-0031 amendment 3).
    async fn status_to_write(
        &self,
        input: &ChangeIssue,
        current: &Issue,
        issue_type: IssueTypeId,
        chain: &[ProjectId],
    ) -> Result<StatusKey, AppError> {
        let named = input
            .status
            .as_ref()
            .filter(|status| **status != current.status || issue_type == current.issue_type);
        match named {
            Some(status) => Ok(status.clone()),
            None => {
                self.status_after_change(chain, issue_type, &current.status)
                    .await
            }
        }
    }

    /// The issue a person asks to change, with what they hold there: they see it (or it answers
    /// like a missing one) and may change it.
    pub(crate) async fn issue_to_change(
        &self,
        actor: &Actor,
        issue: &str,
    ) -> Result<(Arc<Evaluation>, Issue), AppError> {
        let (current, held) = self.visible_issue(actor, issue).await?;
        if !held.may_change_issue(current.reported_by) {
            return Err(ErrorCode::Forbidden.into());
        }
        Ok((held, current))
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The time factor a change of the people may take back to ×1: it matters only when the first
    /// assigned person changes, and only then is it read.
    async fn factor_to_reset(
        &self,
        issue: IssueId,
        people_now: &[UserId],
        people_next: &[UserId],
    ) -> Result<Option<IssueTimeFactor>, AppError> {
        if crate::plan::person_on_bar(people_now) == crate::plan::person_on_bar(people_next) {
            return Ok(None);
        }
        self.store()
            .time_factor(issue)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// What the person asks for, when they may ask it: they see the issue (or it answers like a
    /// missing one), may change it, may change each field they touch, and looked at the version
    /// that is stored.
    pub(crate) async fn plan_issue_change(
        &self,
        actor: &Actor,
        input: &ChangeIssue,
    ) -> Result<(Arc<Evaluation>, Plan), AppError> {
        let (held, current) = self.issue_to_change(actor, &input.issue).await?;
        // An agent changes only what its hand-overs and rules allow (ADR-0029, decision 3).
        self.ensure_agent_may_change(actor, current.id, input)
            .await?;
        // Nothing asked: no field of the catalogue, no issue type, no own field.
        if named(input).is_empty() && input.issue_type.is_none() && input.fields.is_empty() {
            return Err(ErrorCode::InvalidRequest.into());
        }
        // Before anything else is looked at: a field the person may not change is refused whether
        // or not the value differs, and whatever else is wrong with the request.
        // The levels of both types count when the type changes: one request must not do what the
        // same two changes, one after the other, would be refused (ADR-0027).
        let named = named(input);
        for issue_type in input.issue_type.into_iter().chain([current.issue_type]) {
            ensure_may_change(&held.field_mask_for(Some(issue_type)), &named)?;
        }
        let assigned_now = self.assigned_to(current.id).await?;
        let assigned_next = match &input.assigned_to {
            Some(people) => {
                self.clean_assigned_people(
                    actor.person.id,
                    &held,
                    current.project,
                    current.reported_by,
                    people,
                    &assigned_now,
                )
                .await?
            }
            None => assigned_now.clone(),
        };
        let mut next = current.clone();
        if let Some(title) = &input.title {
            next.title = clean_title(title)?;
        }
        next.priority = input.priority.unwrap_or(current.priority);
        set_dates(&mut next, input)?;
        next.estimated_time = input
            .estimated_time
            .map_or(current.estimated_time, SetTo::into_option);
        next.time_left = input
            .time_left
            .map_or(current.time_left, SetTo::into_option);
        if let Some(issue_type) = input.issue_type {
            next.issue_type = self.writable_type(actor, issue_type, &current).await?;
        }
        let chain = self.chain_of(current.project).await?;
        // The status of the request is judged in the workflow that counts afterwards, from the
        // status the issue is in. An issue whose status that workflow lacks starts again in its
        // starting status when the request names none (ADR-0031 amendment 3).
        next.status = self
            .status_to_write(input, &current, next.issue_type, &chain)
            .await?;
        let item = Item {
            kind: RecordKind::Issue,
            id: Some(current.id.into()),
            chain,
            issue_type: Some(next.issue_type),
            reported_by: Some(current.reported_by),
        };
        let stored = self
            .load_values(RecordKind::Issue, &[current.id.into()])
            .await?;
        let own = self
            .plan_values(actor, &held, &item, &input.fields, &stored, Making::Change)
            .await?;
        let entering = self
            .entering_group(
                actor,
                &held,
                &StatusMove {
                    input,
                    current: &current,
                    next: &next,
                    chain: &item.chain,
                    assigned: &assigned_next,
                    own: &own,
                    stored: &stored,
                },
            )
            .await?;
        let factor = self
            .factor_to_reset(current.id, &assigned_now, &assigned_next)
            .await?;
        let plan = Plan {
            current,
            assigned_now,
            next,
            assigned_next,
            own,
            entering,
            factor,
        };
        if plan.current.version != input.version {
            return Err(AppError::stale(
                entity(RecordKind::Issue, plan.current.id),
                Place::Project(plan.current.project),
                actor.person.id,
            ));
        }
        Ok((held, plan))
    }

    /// The issue type a change moves an issue to: it has to exist, not be retired and be offered in
    /// the project of the issue (an issue keeps its type if it is retired or not offered already, so
    /// the type is only judged when it changes).
    async fn writable_type(
        &self,
        actor: &Actor,
        wanted: IssueTypeId,
        current: &Issue,
    ) -> Result<IssueTypeId, AppError> {
        if wanted == current.issue_type {
            return Ok(wanted);
        }
        let chain = self.chain_of(current.project).await?;
        self.writable_issue_type(actor, wanted, &chain).await?;
        Ok(wanted)
    }
}

/// Inside the change of an issue: a change of the type or of the status looks at the offers and the
/// workflow again, on what is stored now, after the shared lock of the workflows; any other change
/// takes no such lock.
async fn ensure_type_and_status_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    stored: &Issue,
    written: &Issue,
    stopped: &Why,
) -> Result<(), S::Error> {
    if written.issue_type == stored.issue_type && written.status == stored.status {
        return Ok(());
    }
    let chain = lock_chain_in::<S>(transaction, written.project).await?;
    if written.issue_type != stored.issue_type {
        ensure_type_takes_issues_in::<S>(transaction, &chain, written.issue_type, stopped).await?;
    }
    // The workflow the status was judged on is still the one that counts.
    ensure_written_status_in::<S>(transaction, &chain, written, Some(stored), stopped).await
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What changing the issue would record, answered without saving: the plan [`change_issue`]
    /// writes (AI-21). A change that changes nothing answers with no change.
    ///
    /// # Errors
    /// As [`change_issue`].
    pub(crate) async fn preview_change_issue(
        &self,
        actor: &Actor,
        input: &ChangeIssue,
    ) -> Result<Vec<FieldChange>, AppError> {
        Ok(self
            .plan_issue_change(actor, input)
            .await?
            .1
            .field_changes())
    }
}

/// Changes the fields of an issue. What is left out stays as it is; a change that changes nothing
/// writes nothing and answers with the issue as it is.
///
/// # Errors
/// `resource.not-found` for an issue that does not exist, is in the trash or that the person may
/// not see, `access.forbidden` without *Change issues* (or *Change own issues* on an issue they
/// reported) or for a field they may not change, `request.invalid` for nothing asked or a text or
/// person that cannot be one, `issue.type-retired` for a retired issue type,
/// `edit.version-changed` when somebody changed the issue since it was looked at.
pub async fn change_issue<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeIssue,
) -> Result<Authorized<IssueView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (held, plan) = ctx.plan_issue_change(&actor, &input).await?;
    let project = ctx
        .store()
        .project(plan.current.project)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let mut views = ctx.issue_views(&actor, held.clone(), project).await?;
    let field_changes = plan.field_changes();
    if field_changes.is_empty() {
        let view = ctx
            .issue_view(&mut views, &plan.current, &plan.assigned_now)
            .await?;
        return Ok(held.authorize(view));
    }
    let now = ctx.now();
    let resets_factor = plan.resets_factor();
    let Plan {
        current,
        mut next,
        assigned_now,
        assigned_next,
        own,
        entering,
        ..
    } = plan;
    let own_writes = own.writes();
    next.updated_at = now;
    next.closed_at = entering.map_or(current.closed_at, |group| {
        group.closed_after(current.closed_at, now)
    });
    let id = current.id;
    let field_changes = crate::event_members::issue_counted_up(field_changes, current.version);
    let (record, place, editor) = (
        entity(RecordKind::Issue, id),
        Place::Project(current.project),
        actor.person.id,
    );
    let event = ctx.event(by(actor.person.id), record.clone(), field_changes)?;
    let people = assigned_rows(id, &assigned_next, now);
    let version = current.version;
    let why = Why::default();
    let newly_assigned: Vec<UserId> = assigned_next
        .iter()
        .filter(|person| !assigned_now.contains(person))
        .copied()
        .collect();
    let (stopped, written, new_people) = (why.clone(), next.clone(), people);
    let kind = EntityKind::from(RecordKind::Issue);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // Judged again on what is stored now (the issue, its new type, its workflow).
                let Some(stored) = S::issue_in(transaction, id).await? else {
                    return Err(stopped.stop::<S>(ErrorCode::NotFound));
                };
                if stored.deleted_at.is_some() {
                    return Err(stopped.stop::<S>(ErrorCode::NotFound));
                }
                if stored.version != version {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                ensure_type_and_status_in::<S>(transaction, &stored, &written, &stopped).await?;
                if !S::update_issue(transaction, &written, Version::new(version)).await? {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                S::replace_assigned_people(transaction, id, &new_people).await?;
                // Another first person: the time factor of the one before goes (ADR-0030,
                // amendment M8).
                if resets_factor {
                    S::delete_time_factor(transaction, id).await?;
                }
                for (field, cells) in &own_writes {
                    S::put_field_value(transaction, kind.clone(), id.into(), *field, cells).await?;
                }
                // Whoever the issue is assigned to now follows it (ISS-07).
                let followers = started_following(id, None, &newly_assigned, None, now);
                S::add_followers(transaction, &followers).await
            })
        })
        .await
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged)
                .about(&record, place, editor)
        })?;
    next.version = next_version(version);
    let view = ctx.issue_view(&mut views, &next, &assigned_next).await?;
    Ok(held.authorize(view))
}
