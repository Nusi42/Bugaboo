//! Changing an issue (ISS-01): the fields the person may change, on the version they looked at.
//! The closed date follows the status group *Done* (ADR-0027, decision 5).

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::fields::Field;
use bb_core::id::{IssueTypeId, UserId};
use bb_core::issue::{Issue, IssueAssignee};
use bb_core::version::{counted_up, next as next_version};
use bb_policy::{Authorized, Evaluation, Place};
use serde_json::{Value, json};

use super::fields::{Given, clean_title, ensure_may_change, member};
use super::types::{ChangeIssue, IssueView};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal};
use crate::projects::Why;
use std::sync::Arc;

/// What a change of an issue asks for, checked against what the person may do and what is
/// stored.
struct Plan {
    /// The issue as it is.
    current: Issue,
    /// The people it is assigned to now, in order.
    assigned_now: Vec<UserId>,
    /// The issue as it will be (the version is the one it has now).
    next: Issue,
    /// The people it will be assigned to.
    assigned_next: Vec<UserId>,
}

impl Plan {
    /// The fields that change, before and after, for the event.
    fn field_changes(&self) -> Vec<FieldChange> {
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
            json!(before.status.code()),
            json!(after.status.code()),
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

impl<S: AuthStore> Ctx<'_, S> {
    /// What the person asks for, when they may ask it: they see the issue (or it answers like a
    /// missing one), may change it, may change each field they touch, and looked at the version
    /// that is stored.
    async fn plan_issue_change(
        &self,
        actor: &Actor,
        input: &ChangeIssue,
    ) -> Result<(Arc<Evaluation>, Plan), AppError> {
        let found = self.find_issue(&input.issue).await?;
        let place = found.as_ref().map_or(Place::Organization, |found| {
            Place::Project(found.issue.project)
        });
        let held = self.evaluation_of(actor, place).await?;
        let Some(current) = found
            .map(|found| found.issue)
            .filter(|issue| issue.deleted_at.is_none() && held.sees_issue(issue.reported_by))
        else {
            return Err(AppError::Refused(ErrorCode::NotFound));
        };
        if !held.may_change_issue(current.reported_by) {
            return Err(ErrorCode::Forbidden.into());
        }
        if input.issue_type.is_none()
            && input.title.is_none()
            && input.status.is_none()
            && input.priority.is_none()
            && input.assigned_to.is_none()
        {
            return Err(ErrorCode::InvalidRequest.into());
        }
        // Before anything else is looked at: a field the person may not change is refused whether
        // or not the value differs, and whatever else is wrong with the request.
        ensure_may_change(&held.field_mask(), &named(input))?;
        let assigned_now: Vec<UserId> = self
            .store()
            .assignees(current.id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|assignee| assignee.person)
            .collect();
        let assigned_next = match &input.assigned_to {
            Some(people) => {
                self.clean_assignees(current.project, current.reported_by, people, &assigned_now)
                    .await?
            }
            None => assigned_now.clone(),
        };
        let mut next = current.clone();
        if let Some(title) = &input.title {
            next.title = clean_title(title)?;
        }
        next.status = input.status.unwrap_or(current.status);
        next.priority = input.priority.unwrap_or(current.priority);
        if let Some(issue_type) = input.issue_type {
            next.issue_type = self.writable_type(issue_type, &current).await?;
        }
        let plan = Plan {
            current,
            assigned_now,
            next,
            assigned_next,
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
    let mut views = ctx.issue_views(held.clone(), project).await?;
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
        assigned_next,
        ..
    } = plan;
    next.updated_at = now;
    next.closed_at = current
        .status
        .closed_after(next.status, current.closed_at, now);
    let id = current.id;
    let mut field_changes = field_changes;
    field_changes.push(counted_up(current.version));
    let (record, place, editor) = (
        entity(RecordKind::Issue, id),
        Place::Project(current.project),
        actor.person.id,
    );
    let event = ctx.event(by(actor.person.id), record.clone(), field_changes)?;
    let people: Vec<IssueAssignee> = assigned_next
        .iter()
        .map(|person| IssueAssignee {
            issue: id,
            person: *person,
            added_at: now,
        })
        .collect();
    let version = current.version;
    let why = Why::default();
    let (stopped, written, new_people) = (why.clone(), next.clone(), people);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // Judged again on what is stored now: somebody may have changed the issue, or
                // retired its new type, since the request began.
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
                if !S::update_issue(transaction, &written, version).await? {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                S::replace_assignees(transaction, id, &new_people).await
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
