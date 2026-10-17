//! *Create an issue* (ISS-01): in a project, with the code of its readable ID given out inside the
//! change (ISS-01a).

use std::sync::Arc;

use bb_core::ErrorCode;
use std::collections::BTreeSet;

use bb_core::event::{EntityKind, RecordKind};
use bb_core::id::{IssueId, IssueTypeId, ProjectId, UserId};
use bb_core::issue::{AssignedPerson, Issue, Priority, StatusGroup};
use bb_core::project::Project;
use bb_core::readable_id::IssueCode;
use bb_core::version::FIRST_VERSION;
use bb_core::workflow::StatusKey;
use bb_policy::{Authorized, Evaluation, Permission, Place};
use serde_json::json;

use super::fields::{Given, clean_title, ensure_may_change, member};
use super::ids::{code_plan_in, new_code_in};
use super::types::{CreateIssue, IssueView};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::fields::{Item, PlannedValues};
use crate::follow::started_following;
use crate::input::clean_long_text;
use crate::projects::Why;
use crate::workflows::StatusCheck;

/// How often a change is tried again when two writers drew the same random code at the same
/// moment (the second one hits the unique key of the project's codes and draws another).
const ATTEMPTS: usize = 3;

/// An issue that a person may make, ready to be stored: everything but the code of its readable
/// ID, which is given out inside the change.
struct NewIssue {
    project: Project,
    issue: Issue,
    assigned_people: Vec<UserId>,
    /// The values of own fields it starts with: asked for, or the defaults.
    own: PlannedValues,
}

impl NewIssue {
    /// The rows of the people the issue is assigned to.
    fn people(&self) -> Vec<AssignedPerson> {
        self.assigned_people
            .iter()
            .map(|person| AssignedPerson {
                issue: self.issue.id,
                person: *person,
                added_at: self.issue.created_at,
            })
            .collect()
    }
}

/// The facts of a new issue that the first status is judged on.
struct NewIssueFacts<'a> {
    input: &'a CreateIssue,
    chain: &'a [ProjectId],
    issue_type: IssueTypeId,
    assigned: &'a [UserId],
    description_filled: bool,
    own: &'a PlannedValues,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The status a new issue starts in, with the status group it enters. A new issue starts in the
    /// first status of its workflow; another status is a status change from there, judged like any
    /// other with the issue as it will be.
    async fn starting_status(
        &self,
        actor: &Actor,
        access: &Evaluation,
        facts: &NewIssueFacts<'_>,
    ) -> Result<(StatusKey, Option<StatusGroup>), AppError> {
        let workflow = self
            .workflow_in_force(facts.chain, Some(facts.issue_type))
            .await?;
        let first = workflow
            .initial()
            .ok_or_else(|| AppError::Internal("a workflow has no status".into()))?
            .clone();
        let status = facts.input.status.clone().unwrap_or_else(|| first.clone());
        if status == first {
            let group = workflow.status(&first).map(|each| each.group);
            return Ok((status, group));
        }
        let own_filled = facts.own.filled_after(BTreeSet::new());
        let given = Given::from(facts.input).keys(facts.input.fields.keys());
        let check = StatusCheck {
            held: access,
            person: actor.person.id,
            chain: facts.chain,
            issue_type: facts.issue_type,
            reported_by: actor.person.id,
            may_change_status: true,
            assigned: facts.assigned,
            description_filled: facts.description_filled,
            own_filled: &own_filled,
            given: &given,
        };
        let checked = self.ensure_status_change(&check, &first, &status).await?;
        Ok((status, Some(checked.group)))
    }

    /// What the person asks for, when they may ask it: they may create issues in the project, the
    /// fields they set are ones they may change (the title always), and the type is one that
    /// takes new issues.
    async fn plan_new_issue(
        &self,
        actor: &Actor,
        input: &CreateIssue,
    ) -> Result<(Arc<Evaluation>, NewIssue), AppError> {
        let project = self
            .find_project(&input.project)
            .await?
            .map(|found| found.project)
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let access = self
            .require(actor, Permission::CreateIssues, Place::Project(project.id))
            .await?;
        // A project the person does not see is not there for them, whatever they may make in it.
        if !access.sees_project() {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        // The mask first: a person who may not set a field is refused alike for any value, so
        // the answer tells nothing about the value (a person ID that exists, a text too long).
        ensure_may_change(
            &access.field_mask_for(Some(input.issue_type)),
            &Given::from(input).fields(),
        )?;
        let title = clean_title(&input.title)?;
        let description = input
            .description
            .as_deref()
            .map(clean_long_text)
            .transpose()?;
        let assigned_people = match &input.assigned_to {
            Some(people) => {
                self.clean_assigned_people(
                    actor.person.id,
                    &access,
                    project.id,
                    actor.person.id,
                    people,
                    &[],
                )
                .await?
            }
            None => Vec::new(),
        };
        let issue_type = self
            .store()
            .issue_type(input.issue_type)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        if issue_type.retired_at.is_some() {
            return Err(ErrorCode::IssueTypeRetired.into());
        }
        let now = self.now();
        let id = IssueId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let item = Item {
            kind: RecordKind::Issue,
            id: Some(id.into()),
            chain: self.chain_of(project.id).await?,
            issue_type: Some(issue_type.id),
            reported_by: Some(actor.person.id),
        };
        let nothing_yet = self.load_values(RecordKind::Issue, &[]).await?;
        let own = self
            .plan_values(actor, &access, &item, &input.fields, &nothing_yet, true)
            .await?;
        let (status, entered) = self
            .starting_status(
                actor,
                &access,
                &NewIssueFacts {
                    input,
                    chain: &item.chain,
                    issue_type: issue_type.id,
                    assigned: &assigned_people,
                    description_filled: description.as_deref().is_some_and(|text| !text.is_empty()),
                    own: &own,
                },
            )
            .await?;
        let issue = Issue {
            id,
            project: project.id,
            issue_type: issue_type.id,
            // The code is given out inside the change.
            code: IssueCode::counting(1),
            title,
            description: description.unwrap_or_default(),
            status,
            priority: input.priority.unwrap_or(Priority::NEW),
            reported_by: actor.person.id,
            created_at: now,
            updated_at: now,
            closed_at: entered.and_then(|group| group.closed_after(None, now)),
            deleted_at: None,
            version: FIRST_VERSION,
        };
        Ok((
            access,
            NewIssue {
                project,
                issue,
                assigned_people,
                own,
            },
        ))
    }
}

/// Creates an issue in a project. The person needs *Create issues* there; the fields they set must be
/// ones they may change (the title always).
///
/// # Errors
/// `resource.not_found` for a project the person may not know about (or that does not exist) or an
/// issue type that does not exist, `access.forbidden` without the permission or for a field the
/// person may not change, `request.invalid` for a title, description or person that cannot be
/// one, `issue.type_retired` for a retired issue type.
pub async fn create_issue<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateIssue,
) -> Result<Authorized<IssueView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (access, new) = ctx.plan_new_issue(&actor, &input).await?;
    let (id, people) = (new.issue.id, new.people());
    let own_writes = new.own.writes();
    let own_changes = new.own.changes();
    let mut changes = vec![
        set_to(member::PROJECT, json!(new.project.id.to_string())),
        set_to(member::ISSUE_TYPE, json!(new.issue.issue_type.to_string())),
        set_to(member::TITLE, json!(new.issue.title)),
        set_to(member::DESCRIPTION, json!(new.issue.description)),
        set_to(member::STATUS, json!(new.issue.status.as_str())),
        set_to(member::PRIORITY, json!(new.issue.priority.code())),
        set_to(member::REPORTED_BY, json!(actor.person.id.to_string())),
        set_to(
            member::ASSIGNED_TO,
            json!(
                new.assigned_people
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            ),
        ),
    ];
    changes.extend(own_changes);
    let event = ctx.event(by(actor.person.id), entity(RecordKind::Issue, id), changes)?;
    let mut tried = 0;
    loop {
        tried += 1;
        let why = Why::default();
        let (stopped, mut source) = (why.clone(), ctx.owned_ids());
        let (issue, people) = (new.issue.clone(), people.clone());
        let own_writes = own_writes.clone();
        let result = ctx
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    // The type may have been retired since the request began.
                    let retired = S::issue_types_in(transaction)
                        .await?
                        .iter()
                        .find(|found| found.id == issue.issue_type)
                        .is_none_or(|found| found.retired_at.is_some());
                    if retired {
                        return Err(stopped.stop::<S>(ErrorCode::IssueTypeRetired));
                    }
                    // The mode and the code are judged after the lock on the codes of the
                    // project, so a change of the mode meanwhile is seen whole or not at all.
                    S::lock_issue_codes(transaction, issue.project).await?;
                    let plan = code_plan_in::<S>(transaction, issue.project).await?;
                    let code =
                        new_code_in::<S>(transaction, issue.project, plan, &mut source).await?;
                    let issue = Issue { code, ..issue };
                    S::insert_issue(transaction, &issue, &people).await?;
                    for (field, cells) in &own_writes {
                        S::put_field_value(
                            transaction,
                            EntityKind::from(RecordKind::Issue),
                            issue.id.into(),
                            *field,
                            cells,
                        )
                        .await?;
                    }
                    // Whoever reports an issue, or is assigned to it, follows it (ISS-07).
                    let assigned: Vec<UserId> = people
                        .iter()
                        .map(|assigned_person| assigned_person.person)
                        .collect();
                    let followers = started_following(
                        issue.id,
                        Some(issue.reported_by),
                        &assigned,
                        None,
                        issue.created_at,
                    );
                    S::add_followers(transaction, &followers).await
                })
            })
            .await;
        match result {
            Err(error) if S::is_duplicate(&error) && tried < ATTEMPTS => {}
            Err(error) => return Err(why.answer::<S>(&error, ErrorCode::NameTaken)),
            Ok(()) => break,
        }
    }
    let stored = ctx
        .store()
        .issue(id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or_else(|| AppError::Internal("a new issue is not there".into()))?;
    let mut views = ctx.issue_views(&actor, access.clone(), new.project).await?;
    let view = ctx
        .issue_view(&mut views, &stored, &new.assigned_people)
        .await?;
    Ok(access.authorize(view))
}
