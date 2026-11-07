//! What an agent may change on an issue it works on (ADR-0029, decision 3): the ticks of the
//! hand-over and the fields of the standing rule. Never a member that holds people (COMP-10), and
//! the status only when it was ticked. Reading, commenting, reporting, attaching and linking
//! related issues need nothing.

use bb_core::ErrorCode;
use bb_core::id::IssueId;

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal, locked};
use crate::issues::ChangeIssue;

/// The member of an issue that a change of its status writes.
pub(crate) const STATUS: &str = "status";
/// The description of an issue.
pub(crate) const DESCRIPTION: &str = "description";
/// The project an issue is in.
pub(crate) const PROJECT: &str = "project";
/// The labels of an issue.
pub(crate) const LABELS: &str = "labels";
/// The tags of an issue.
pub(crate) const TAGS: &str = "tags";
/// Putting an issue under another as its sub-issue.
pub(crate) const PARENT: &str = "parent";
/// Deleting an issue: no hand-over gives it.
pub(crate) const DELETE: &str = "delete";

/// The members of the issue a change names, by their keys: the ones of the catalogue and the own
/// fields.
fn members_of(input: &ChangeIssue) -> Vec<String> {
    let named = [
        (input.issue_type.is_some(), "issue_type"),
        (input.title.is_some(), "title"),
        (input.status.is_some(), STATUS),
        (input.priority.is_some(), "priority"),
        (input.estimated_time.is_some(), "estimated_time"),
        (input.time_left.is_some(), "time_left"),
        (input.assigned_to.is_some(), "assigned_to"),
    ];
    named
        .into_iter()
        .filter(|(set, _)| *set)
        .map(|(_, key)| key.to_owned())
        .chain(input.fields.keys().cloned())
        .collect()
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Refuses a change by an agent to a member its work does not allow. A person is not asked.
    /// What the agent then changes belongs to the run of its attempt.
    ///
    /// # Errors
    /// `access.forbidden` for a member the agent may not change on this issue.
    pub(crate) async fn ensure_agent_may(
        &self,
        actor: &Actor,
        issue: IssueId,
        members: &[String],
    ) -> Result<(), AppError> {
        let Some(agent) = actor.agent() else {
            return Ok(());
        };
        let store_failed = |error| internal::<S>(&error);
        let work = self.store().agent_work(agent).await.map_err(store_failed)?;
        let allowance = work.allowances.get(&issue).cloned().unwrap_or_default();
        let by_the_system = *locked(&self.accepting) == Some(issue);
        let allowed = members.iter().all(|member| {
            allowance.may_change(member, STATUS) || (by_the_system && member == STATUS)
        });
        if !allowed {
            return Err(ErrorCode::Forbidden.into());
        }
        if let Some(run) = self.run_of_attempt_on(agent, issue).await? {
            self.set_run(Some(run));
        }
        Ok(())
    }

    /// Refuses a change by an agent to one member of an issue; see [`Self::ensure_agent_may`].
    pub(crate) async fn ensure_agent_may_member(
        &self,
        actor: &Actor,
        issue: IssueId,
        member: &str,
    ) -> Result<(), AppError> {
        self.ensure_agent_may(actor, issue, &[member.to_owned()])
            .await
    }

    /// Refuses a change of an issue by an agent, for what the change names.
    pub(crate) async fn ensure_agent_may_change(
        &self,
        actor: &Actor,
        issue: IssueId,
        input: &ChangeIssue,
    ) -> Result<(), AppError> {
        if actor.agent().is_none() {
            return Ok(());
        }
        self.ensure_agent_may(actor, issue, &members_of(input))
            .await
    }

    /// The run of the attempt that runs now on the issue for the agent, if there is one.
    async fn run_of_attempt_on(
        &self,
        agent: bb_core::id::AgentId,
        issue: IssueId,
    ) -> Result<Option<bb_core::id::RunId>, AppError> {
        let store_failed = |error| internal::<S>(&error);
        let Some(work) = self
            .store()
            .open_work_of(agent, issue)
            .await
            .map_err(store_failed)?
        else {
            return Ok(None);
        };
        let attempts = self
            .store()
            .attempts_of(work.id)
            .await
            .map_err(store_failed)?;
        Ok(attempts
            .last()
            .filter(|attempt| attempt.ended_at.is_none())
            .and_then(|attempt| attempt.run))
    }
}
