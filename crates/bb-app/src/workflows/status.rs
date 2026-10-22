//! Judging a status change: the facts of the issue and the person go to `bb_core::workflow::judge`,
//! and its answer becomes an error (a change that is made) or a list of buttons (an issue that is
//! shown).

use std::collections::BTreeSet;
use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{FieldId, GroupId, IssueTypeId, ProjectId, UserId};
use bb_core::issue::StatusGroup;
use bb_core::workflow::judge::{Because, Changer, Verdict, judge};
use bb_core::workflow::{StatusKey, WorkflowContent};
use bb_policy::Evaluation;

use super::facts::IssueFacts;
use super::types::{StatusChangeView, StatusView};
use crate::auth::{AppError, AuthStore, Ctx, internal, locked};

/// What a status change is judged on: the issue as it will be, and the person who changes it.
pub(crate) struct StatusCheck<'a> {
    /// What the person who changes it may do where the issue is.
    pub(crate) held: &'a Evaluation,
    /// Who changes it.
    pub(crate) person: UserId,
    /// The projects from the top down to the project of the issue.
    pub(crate) chain: &'a [ProjectId],
    /// The issue type: it picks the workflow and the levels of the fields.
    pub(crate) issue_type: IssueTypeId,
    /// Who reported the issue.
    pub(crate) reported_by: UserId,
    /// Whether the person may change the status of the issue at all.
    pub(crate) may_change_status: bool,
    /// The people the issue is assigned to after the change.
    pub(crate) assigned: &'a [UserId],
    /// Whether the description has a text.
    pub(crate) description_filled: bool,
    /// The own fields that have a value after the change.
    pub(crate) own_filled: &'a BTreeSet<FieldId>,
    /// The fields the request carries.
    pub(crate) given: &'a BTreeSet<FieldKey>,
}

impl StatusCheck<'_> {
    /// Whether the person may change the status of an issue at all: they may change the issue
    /// (*Change issues*, or *Change own issues* on an issue they reported) and their level for
    /// *Status* is *Can change*. The one answer for the issue page and for *Try it*.
    pub(crate) fn may_change_status(
        held: &Evaluation,
        issue_type: IssueTypeId,
        reported_by: UserId,
    ) -> bool {
        held.may_change_issue(reported_by)
            && held.field_level_for(Some(issue_type), Field::Status) >= FieldLevel::CanChange
    }
}

/// A status change that the judge allows.
pub(crate) struct StatusChecked {
    /// The group of the status the issue enters: it decides the closed date.
    pub(crate) group: StatusGroup,
}

/// The error a verdict that is no *Allowed* is answered with. A field is named only when the judge
/// says the person may know it.
fn refusal(verdict: Verdict) -> Option<AppError> {
    match verdict {
        Verdict::Allowed => None,
        Verdict::Needs(fields) => Some(match fields.first() {
            Some(field) => AppError::RefusedAtField {
                code: ErrorCode::FieldRequired,
                field: field.code().into_owned(),
            },
            None => AppError::Refused(ErrorCode::FieldRequired),
        }),
        Verdict::NotAllowed(Because { why, field }) => Some(match field {
            Some(field) => AppError::RefusedAtField {
                code: why.error(),
                field: field.code().into_owned(),
            },
            None => AppError::Refused(why.error()),
        }),
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The groups a person is in, read once for the request.
    async fn groups_of_person(&self, person: UserId) -> Result<Arc<Vec<GroupId>>, AppError> {
        let known = locked(&self.known.person_groups).get(&person).cloned();
        if let Some(groups) = known {
            return Ok(groups);
        }
        let groups: Arc<Vec<GroupId>> = Arc::new(
            self.store()
                .groups_of(person)
                .await
                .map_err(|error| internal::<S>(&error))?
                .into_iter()
                .map(|group| group.id)
                .collect(),
        );
        locked(&self.known.person_groups).insert(person, Arc::clone(&groups));
        Ok(groups)
    }

    /// The judge's answer about the status change `from` → `to`.
    async fn status_verdict(
        &self,
        check: &StatusCheck<'_>,
        workflow: &WorkflowContent,
        from: &StatusKey,
        to: &StatusKey,
    ) -> Result<Verdict, AppError> {
        let groups = self.groups_of_person(check.person).await?;
        let roles: Vec<_> = check.held.roles().iter().map(|role| role.id).collect();
        // Who is assigned counts for *Who can do it* only for a person who is shown the field:
        // the buttons must not tell what the field would not.
        let sees_assigned = check
            .held
            .field_level_for(Some(check.issue_type), Field::AssignedTo)
            >= FieldLevel::CanSee;
        let changer = Changer {
            may_change_status: check.may_change_status,
            roles: &roles,
            groups: &groups,
            assigned: sees_assigned && check.assigned.contains(&check.person),
            reporter: check.reported_by == check.person,
        };
        let facts = IssueFacts {
            viewer: check.held,
            issue_type: check.issue_type,
            description_filled: check.description_filled,
            assigned_filled: !check.assigned.is_empty(),
            own_filled: check.own_filled,
            given: check.given,
        };
        Ok(judge(workflow, from, to, &changer, &facts))
    }

    /// Lets the status change `from` → `to` pass, or refuses it with the answer of the judge.
    ///
    /// # Errors
    /// `workflow.no_such_change`, `workflow.not_for_you`, `workflow.not_named`, `workflow.only_if` and
    /// `workflow.cannot_fill` for a change the judge turns down, and `field.required` at the field
    /// for a change that still needs one.
    pub(crate) async fn ensure_status_change(
        &self,
        check: &StatusCheck<'_>,
        from: &StatusKey,
        to: &StatusKey,
    ) -> Result<StatusChecked, AppError> {
        let workflow = self
            .workflow_in_force(check.chain, Some(check.issue_type))
            .await?;
        let verdict = self.status_verdict(check, &workflow, from, to).await?;
        if let Some(error) = refusal(verdict) {
            return Err(error);
        }
        let group = workflow
            .status(to)
            .map(|status| status.group)
            .ok_or(AppError::Refused(ErrorCode::WorkflowNoSuchChange))?;
        Ok(StatusChecked { group })
    }

    /// The status an issue is in after its type or project changed and the request names none: the
    /// one it has when the workflow that counts has it, else the starting status of that workflow.
    pub(crate) async fn status_after_change(
        &self,
        chain: &[ProjectId],
        issue_type: IssueTypeId,
        status: &StatusKey,
    ) -> Result<StatusKey, AppError> {
        let workflow = self.workflow_in_force(chain, Some(issue_type)).await?;
        if workflow.status(status).is_some() {
            return Ok(status.clone());
        }
        workflow
            .initial()
            .cloned()
            .ok_or(AppError::Refused(ErrorCode::WorkflowNoSuchChange))
    }

    /// What the judge says about every status change from `status`, each with the status it ends in
    /// and the text of its button. An issue in a status that its workflow does not have may go only
    /// to the starting status of it, and the button is the name of that status.
    pub(crate) async fn status_verdicts(
        &self,
        check: &StatusCheck<'_>,
        workflow: &WorkflowContent,
        status: &StatusKey,
    ) -> Result<Vec<(StatusView, String, Verdict)>, AppError> {
        let targets: Vec<(&StatusKey, String)> = if workflow.status(status).is_some() {
            workflow
                .changes_from(status)
                .map(|change| (&change.to, change.button.clone()))
                .collect()
        } else {
            workflow
                .initial()
                .and_then(|start| workflow.status(start))
                .map(|each| (&each.key, each.name.clone()))
                .into_iter()
                .collect()
        };
        let mut verdicts = Vec::with_capacity(targets.len());
        for (to, button) in targets {
            let Some(entered) = workflow.status(to) else {
                continue;
            };
            let verdict = self.status_verdict(check, workflow, status, to).await?;
            verdicts.push((StatusView::from(entered), button, verdict));
        }
        Ok(verdicts)
    }

    /// The status changes from `status` that the person may make now, each with what it still
    /// needs.
    pub(crate) async fn status_changes_of(
        &self,
        check: &StatusCheck<'_>,
        workflow: &WorkflowContent,
        status: &StatusKey,
    ) -> Result<Vec<StatusChangeView>, AppError> {
        Ok(self
            .status_verdicts(check, workflow, status)
            .await?
            .into_iter()
            .filter_map(|(to, button, verdict)| match verdict {
                Verdict::Allowed => Some((to, button, Vec::new())),
                Verdict::Needs(fields) => Some((to, button, fields)),
                Verdict::NotAllowed(_) => None,
            })
            .map(|(to, button, needs)| StatusChangeView { to, button, needs })
            .collect())
    }
}
