//! What an agent may change on an issue it works on (ADR-0029, decision 3): the ticks of the
//! hand-over and the fields of the standing rule. Never a member that holds people (COMP-10), and
//! the status only when it was ticked.

use bb_core::ErrorCode;
use bb_core::id::IssueId;

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::issues::ChangeIssue;

/// The member of an issue that a change of its status writes.
const STATUS: &str = "status";

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
    ///
    /// # Errors
    /// `access.forbidden` for a member the agent may not change on this issue.
    pub(crate) async fn ensure_agent_may_change(
        &self,
        actor: &Actor,
        issue: IssueId,
        input: &ChangeIssue,
    ) -> Result<(), AppError> {
        let Some(agent) = actor.agent() else {
            return Ok(());
        };
        let work = self
            .store()
            .agent_work(agent)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let allowance = work.allowances.get(&issue).cloned().unwrap_or_default();
        if members_of(input)
            .iter()
            .all(|member| allowance.may_change(member, STATUS))
        {
            Ok(())
        } else {
            Err(ErrorCode::Forbidden.into())
        }
    }
}
