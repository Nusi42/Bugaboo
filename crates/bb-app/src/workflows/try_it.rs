//! *Try it* (WF-02, ADR-0031 decision 3): pick an issue and a person and see which status changes
//! the person would get, and why not the others. It asks the same judge as the issue page, with the
//! facts of the person tried, so the answer of the editor is the answer the person would get.

use std::collections::BTreeSet;

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::event::RecordKind;
use bb_core::fields::{Field, FieldKey};
use bb_core::workflow::judge::{Verdict, Why};
use bb_policy::{Authorized, Permission, Place};

use super::status::StatusCheck;
use super::types::{StatusView, Tried, TriedChange, TriedVerdict, TryWorkflowQuery};
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The answer of the judge as *Try it* tells it. A change whose rules name a field the person who
/// tries may not see is answered `cannot fill`, with no field and no detail, whatever the judge
/// said about it: the answer must tell nothing about the field.
fn told(verdict: Verdict, rules_hide_a_field: bool) -> TriedVerdict {
    if rules_hide_a_field {
        return TriedVerdict::NotAllowed {
            why: Why::CannotFill,
            field: None,
        };
    }
    match verdict {
        Verdict::Allowed => TriedVerdict::Allowed,
        Verdict::Needs(fields) => TriedVerdict::Needs { fields },
        Verdict::NotAllowed(because) => TriedVerdict::NotAllowed {
            why: because.why,
            field: because.field,
        },
    }
}

/// The status changes a person would get on an issue, with the answer of the judge for each.
/// Needs *Manage workflows* in the project of the issue, and the issue to be one the person who
/// tries sees.
///
/// # Errors
/// `resource.not_found` for an issue the person may not see or a person they may not know about,
/// `access.forbidden` without *Manage workflows* in the project of the issue.
pub async fn try_workflow<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: TryWorkflowQuery,
) -> Result<Authorized<Tried>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, _) = ctx.visible_issue(&actor, &input.issue).await?;
    let place = Place::Project(issue.project);
    let held = ctx
        .require(&actor, Permission::ManageWorkflows, place)
        .await?;
    ctx.store()
        .person(input.person)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let theirs = ctx.evaluate(input.person, place).await?;
    // Like *Check access*: whoever does not manage people reads the people who have a role there.
    if !held.manages_people() && theirs.roles().is_empty() {
        return Err(AppError::Refused(ErrorCode::NotFound));
    }
    let chain = held.chain().to_vec();
    let workflow = ctx
        .workflow_in_force(&chain, Some(issue.issue_type))
        .await?;
    let assigned: Vec<_> = ctx
        .store()
        .assigned_people(issue.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .map(|each| each.person)
        .collect();
    let values = ctx
        .load_values(RecordKind::Issue, &[issue.id.into()])
        .await?;
    let own_filled = values.filled_fields(issue.id.into());
    let may_change_status = theirs.may_change_issue(issue.reported_by)
        && theirs.field_level_for(Some(issue.issue_type), Field::Status) >= FieldLevel::CanChange;
    let check = StatusCheck {
        held: &theirs,
        person: input.person,
        chain: &chain,
        issue_type: issue.issue_type,
        reported_by: issue.reported_by,
        may_change_status,
        assigned: &assigned,
        description_filled: !issue.description.is_empty(),
        own_filled: &own_filled,
        given: &BTreeSet::new(),
    };
    let verdicts = ctx
        .status_verdicts(&check, &workflow, &issue.status)
        .await?;
    let hidden_from_trier =
        |field: FieldKey| held.field_level_for(Some(issue.issue_type), field) < FieldLevel::CanSee;
    Ok(held.authorize(Tried {
        status: StatusView::of_issue(&workflow, &issue),
        changes: verdicts
            .into_iter()
            .map(|(to, button, verdict)| {
                let hides = workflow
                    .change(&issue.status, &to.key)
                    .is_some_and(|each| each.rules.fields().into_iter().any(hidden_from_trier));
                TriedChange {
                    verdict: told(verdict, hides),
                    to,
                    button,
                }
            })
            .collect(),
    }))
}
