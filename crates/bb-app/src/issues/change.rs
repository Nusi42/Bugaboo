//! Changing an issue (ISS-01): the fields the person may change, on the version they looked at.
//! The closed date follows the status group *Done* (ADR-0027, decision 5).

use bb_core::ErrorCode;

use bb_core::event::{EntityKind, FieldChange, RecordKind};
use bb_core::fields::Field;
use bb_core::id::{IssueId, IssueTypeId, ProjectId, UserId};
use bb_core::issue::{AssignedPerson, Issue, StatusGroup};
use bb_core::version::{counted_up, next as next_version};
use bb_core::workflow::StatusKey;
use bb_policy::{Authorized, Evaluation, Place};
use jiff::Timestamp;
use serde_json::{Value, json};

use super::fields::{Given, clean_title, ensure_may_change, member};
use super::types::{ChangeIssue, IssueView};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal};
use crate::fields::{Item, ItemValues, PlannedValues};
use crate::follow::started_following;
use crate::projects::Why;
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
}

impl Plan {
    /// The fields that change, before and after, for the event.
    pub(crate) fn field_changes(&self) -> Vec<FieldChange> {
        let (before, after) = (&self.current, &self.next);
        let mut changes = Vec::new();
        let mut add = |name: &str, was: Value, now: Value| {
            if was != now {
                changes.push(FieldChange {
                    field: name.to_owned(),
                    before: Some(was),
                    after: Some(now),
                });
            }
        };
        add(
            member::ISSUE_TYPE,
            json!(before.issue_type.to_string()),
            json!(after.issue_type.to_string()),
        );
        add(member::TITLE, json!(before.title), json!(after.title));
        add(
            member::STATUS,
            json!(before.status.as_str()),
            json!(after.status.as_str()),
        );
        add(
            member::PRIORITY,
            json!(before.priority.code()),
            json!(after.priority.code()),
        );
        add(
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
        );
        changes.extend(self.own.changes());
        changes
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
            may_change_status: true,
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
    /// What the person asks for, when they may ask it: they see the issue (or it answers like a
    /// missing one), may change it, may change each field they touch, and looked at the version
    /// that is stored.
    pub(crate) async fn plan_issue_change(
        &self,
        actor: &Actor,
        input: &ChangeIssue,
    ) -> Result<(Arc<Evaluation>, Plan), AppError> {
        let (held, current) = self.issue_to_change(actor, &input.issue).await?;
        if input.issue_type.is_none()
            && input.title.is_none()
            && input.status.is_none()
            && input.priority.is_none()
            && input.assigned_to.is_none()
            && input.fields.is_empty()
        {
            return Err(ErrorCode::InvalidRequest.into());
        }
        // Before anything else is looked at: a field the person may not change is refused whether
        // or not the value differs, and whatever else is wrong with the request.
        ensure_may_change(
            &held.field_mask_for(Some(current.issue_type)),
            &named(input),
        )?;
        let assigned_now: Vec<UserId> = self
            .store()
            .assigned_people(current.id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|assigned_person| assigned_person.person)
            .collect();
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
        if let Some(issue_type) = input.issue_type {
            next.issue_type = self.writable_type(issue_type, &current).await?;
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
            .plan_values(actor, &held, &item, &input.fields, &stored, false)
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
        let plan = Plan {
            current,
            assigned_now,
            next,
            assigned_next,
            own,
            entering,
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

    /// The issue type a change moves an issue to: it has to exist and not be retired (an issue
    /// keeps its type if it is retired already, so the type is only judged when it changes).
    async fn writable_type(
        &self,
        wanted: IssueTypeId,
        current: &Issue,
    ) -> Result<IssueTypeId, AppError> {
        if wanted == current.issue_type {
            return Ok(wanted);
        }
        let found = self
            .store()
            .issue_type(wanted)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        if found.retired_at.is_some() {
            return Err(ErrorCode::IssueTypeRetired.into());
        }
        Ok(wanted)
    }
}

/// The rows that say who an issue is assigned to.
fn assigned_rows(issue: IssueId, people: &[UserId], added_at: Timestamp) -> Vec<AssignedPerson> {
    people
        .iter()
        .map(|person| AssignedPerson {
            issue,
            person: *person,
            added_at,
        })
        .collect()
}

/// Changes the fields of an issue. What is left out stays as it is; a change that changes nothing
/// writes nothing and answers with the issue as it is.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist, is in the trash or that the person may
/// not see, `access.forbidden` without *Change issues* (or *Change own issues* on an issue they
/// reported) or for a field they may not change, `request.invalid` for nothing asked or a text or
/// person that cannot be one, `issue.type_retired` for a retired issue type,
/// `edit.version_changed` when somebody changed the issue since it was looked at.
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
    let Plan {
        current,
        mut next,
        assigned_now,
        assigned_next,
        own,
        entering,
    } = plan;
    let own_writes = own.writes();
    next.updated_at = now;
    next.closed_at = entering.map_or(current.closed_at, |group| {
        group.closed_after(current.closed_at, now)
    });
    let id = current.id;
    let mut field_changes = field_changes;
    field_changes.push(counted_up(current.version));
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
                if written.issue_type != stored.issue_type {
                    let retired = S::issue_types_in(transaction)
                        .await?
                        .iter()
                        .find(|found| found.id == written.issue_type)
                        .is_none_or(|found| found.retired_at.is_some());
                    if retired {
                        return Err(stopped.stop::<S>(ErrorCode::IssueTypeRetired));
                    }
                }
                // The workflow the status was judged on is still the one that counts.
                ensure_written_status_in::<S>(transaction, &written, Some(&stored), &stopped)
                    .await?;
                if !S::update_issue(transaction, &written, version).await? {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                S::replace_assigned_people(transaction, id, &new_people).await?;
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
