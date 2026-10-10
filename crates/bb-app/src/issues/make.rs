//! *Make an issue* (ISS-01): in a project, with the code of its readable ID given out inside the
//! change (ISS-01a).

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::id::{IssueId, UserId};
use bb_core::issue::{Issue, IssueAssignee, Priority, Status};
use bb_core::issue_key::IssueCode;
use bb_core::project::Project;
use bb_core::version::FIRST_VERSION;
use bb_policy::{Authorized, Evaluation, Permission, Place};
use serde_json::json;

use super::fields::{Given, clean_description, clean_title, ensure_may_change, member};
use super::ids::{code_plan_in, new_code_in};
use super::types::{IssueView, MakeIssue};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::projects::Why;

/// How often a change is tried again when two writers drew the same random code at the same
/// moment (the second one hits the unique key of the project's codes and draws another).
const ATTEMPTS: usize = 3;

/// An issue that a person may make, ready to be stored: everything but the code of its readable
/// ID, which is given out inside the change.
struct NewIssue {
    project: Project,
    issue: Issue,
    assignees: Vec<UserId>,
}

impl NewIssue {
    /// The rows of the people the issue is assigned to.
    fn people(&self) -> Vec<IssueAssignee> {
        self.assignees
            .iter()
            .map(|person| IssueAssignee {
                issue: self.issue.id,
                person: *person,
                added_at: self.issue.created_at,
            })
            .collect()
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What the person asks for, when they may ask it: they may make issues in the project, the
    /// fields they set are ones they may change (the title always), and the type is one that
    /// takes new issues.
    async fn plan_new_issue(
        &self,
        actor: &Actor,
        input: &MakeIssue,
    ) -> Result<(Arc<Evaluation>, NewIssue), AppError> {
        let project = self
            .find_project(&input.project)
            .await?
            .map(|found| found.project)
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let access = self
            .require(actor, Permission::MakeIssues, Place::Project(project.id))
            .await?;
        // A project the person does not see is not there for them, whatever they may make in it.
        if !access.sees_project() {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        // The mask first: a person who may not set a field is refused alike for any value, so
        // the answer tells nothing about the value (a person ID that exists, a text too long).
        ensure_may_change(&access.field_mask(), &Given::from(input).fields())?;
        let title = clean_title(&input.title)?;
        let description = input
            .description
            .as_deref()
            .map(clean_description)
            .transpose()?;
        let assignees = match &input.assigned_to {
            Some(people) => self.clean_assignees(people).await?,
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
        let status = input.status.unwrap_or(Status::NEW);
        let issue = Issue {
            id: IssueId::generate(&mut self.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?,
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
            closed_at: Status::NEW.closed_after(status, None, now),
            deleted_at: None,
            version: FIRST_VERSION,
        };
        Ok((
            access,
            NewIssue {
                project,
                issue,
                assignees,
            },
        ))
    }
}

/// Makes an issue in a project. The person needs *Make issues* there; the fields they set must be
/// ones they may change (the title always).
///
/// # Errors
/// `resource.not_found` for a project the person may not know about (or that does not exist) or an
/// issue type that does not exist, `access.forbidden` without the permission or for a field the
/// person may not change, `request.invalid` for a title, description or person that cannot be
/// one, `issue.type_retired` for a retired issue type.
pub async fn make_issue<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MakeIssue,
) -> Result<Authorized<IssueView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (access, new) = ctx.plan_new_issue(&actor, &input).await?;
    let (id, people) = (new.issue.id, new.people());
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Issue, id),
        vec![
            set_to(member::PROJECT, json!(new.project.id.to_string())),
            set_to(member::ISSUE_TYPE, json!(new.issue.issue_type.to_string())),
            set_to(member::TITLE, json!(new.issue.title)),
            set_to(member::DESCRIPTION, json!(new.issue.description)),
            set_to(member::STATUS, json!(new.issue.status.code())),
            set_to(member::PRIORITY, json!(new.issue.priority.code())),
            set_to(member::REPORTED_BY, json!(actor.person.id.to_string())),
            set_to(
                member::ASSIGNED_TO,
                json!(
                    new.assignees
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                ),
            ),
        ],
    )?;
    let mut tried = 0;
    loop {
        tried += 1;
        let why = Why::default();
        let (stopped, mut source) = (why.clone(), ctx.owned_ids());
        let (issue, people) = (new.issue.clone(), people.clone());
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
                    S::insert_issue(transaction, &issue, &people).await
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
    let mut views = ctx.issue_views(access.clone(), new.project).await?;
    let view = ctx.issue_view(&mut views, &stored, &new.assignees).await?;
    Ok(access.authorize(view))
}
