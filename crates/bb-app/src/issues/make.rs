//! *Make an issue* (ISS-01): in a project, with the code of its readable ID given out inside the
//! change (ISS-01a).

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::fields::Field;
use bb_core::id::IssueId;
use bb_core::issue::{Issue, IssueAssignee, Priority, Status};
use bb_core::issue_key::IssueCode;
use bb_policy::{Authorized, Permission, Place};
use serde_json::json;

use super::fields::{clean_description, clean_title, ensure_may_change};
use super::ids::new_code_in;
use super::types::{IssueView, MakeIssue};
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::projects::Why;

/// How often a change is tried again when two writers drew the same random code at the same
/// moment (the second one hits the unique key of the project's codes and draws another).
const ATTEMPTS: usize = 3;

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
    let project = ctx
        .find_project(&input.project)
        .await?
        .map(|found| found.project)
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let access = ctx
        .require(&actor, Permission::MakeIssues, Place::Project(project.id))
        .await?;
    // A project the person does not see is not there for them, whatever they may make in it.
    if !access.sees_project() {
        return Err(AppError::Refused(ErrorCode::NotFound));
    }
    let title = clean_title(&input.title)?;
    let description = input
        .description
        .as_deref()
        .map(clean_description)
        .transpose()?;
    let assignees = match &input.assigned_to {
        Some(people) => ctx.clean_assignees(people).await?,
        None => Vec::new(),
    };
    let mut touched = vec![Field::Title];
    for (given, field) in [
        (description.is_some(), Field::Description),
        (input.status.is_some(), Field::Status),
        (input.priority.is_some(), Field::Priority),
        (!assignees.is_empty(), Field::AssignedTo),
    ] {
        if given {
            touched.push(field);
        }
    }
    ensure_may_change(&access.field_mask(), &touched)?;
    let issue_type = ctx
        .store()
        .issue_type(input.issue_type)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    if issue_type.retired_at.is_some() {
        return Err(ErrorCode::IssueTypeRetired.into());
    }
    let plan = ctx.code_plan(project.id).await?;
    let now = ctx.now();
    let status = input.status.unwrap_or(Status::NEW);
    let id =
        IssueId::generate(&mut ctx.ids()).map_err(|error| AppError::Internal(error.to_string()))?;
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
        closed_at: Status::NEW.closed_after(status, None, now),
        deleted_at: None,
        version: 1,
    };
    let people: Vec<IssueAssignee> = assignees
        .iter()
        .map(|person| IssueAssignee {
            issue: id,
            person: *person,
            added_at: now,
        })
        .collect();
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Issue, id),
        vec![
            set_to("project", json!(project.id.to_string())),
            set_to("issue_type", json!(issue.issue_type.to_string())),
            set_to("title", json!(issue.title)),
            set_to("description", json!(issue.description)),
            set_to("status", json!(issue.status.code())),
            set_to("priority", json!(issue.priority.code())),
            set_to("reported_by", json!(actor.person.id.to_string())),
            set_to(
                "assigned_to",
                json!(
                    assignees
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
        let (issue, people) = (issue.clone(), people.clone());
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
    let mut views = ctx.issue_views(access.clone(), project).await?;
    let view = ctx.issue_view(&mut views, &stored, &assignees).await?;
    Ok(access.authorize(view))
}
