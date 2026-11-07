//! What an agent may change on an issue it works on (ADR-0029, decision 3): the ticks of the
//! hand-over and the fields of the standing rule. Never a member that holds people (COMP-10), and
//! the status only when it was ticked. Reading, commenting, reporting, attaching and linking
//! related issues need nothing.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::id::{AgentId, IssueId};

use crate::access::Actor;
use crate::agents::AgentWork;
use crate::auth::{AppError, AuthStore, Ctx, internal, locked};
use crate::issues::ChangeIssue;

/// The members of an issue that a hand-over or a rule can allow an agent to change, by their keys:
/// the ones events and levels name (`issues::member`) and the one thing no hand-over gives.
pub(crate) mod issue_member {
    pub(crate) use crate::issues::member::{DESCRIPTION, LABELS, PARENT, PROJECT, STATUS, TAGS};
    pub(crate) use bb_core::agent_work::issue_member::{ATTACHMENTS, COMMENTS, RELATIONS};

    /// Deleting an issue: no hand-over gives it.
    pub(crate) const DELETE: &str = "delete";
}

use issue_member::STATUS;

/// The members of the issue a change names, by their keys: the ones of the catalogue and the own
/// fields.
fn members_of(input: &ChangeIssue) -> Vec<String> {
    use bb_core::agent_work::issue_member as key;
    let named = [
        (input.issue_type.is_some(), key::ISSUE_TYPE),
        (input.title.is_some(), key::TITLE),
        (input.status.is_some(), STATUS),
        (input.priority.is_some(), key::PRIORITY),
        (input.estimated_time.is_some(), key::ESTIMATED_TIME),
        (input.time_left.is_some(), key::TIME_LEFT),
        (input.assigned_to.is_some(), key::ASSIGNED_TO),
    ];
    named
        .into_iter()
        .filter(|(set, _)| *set)
        .map(|(_, key)| key.to_owned())
        .chain(input.fields.keys().cloned())
        .collect()
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The work of an agent now, read once for the request.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn work_of_agent_now(
        &self,
        agent: AgentId,
    ) -> Result<Arc<AgentWork>, AppError> {
        if let Some(held) = locked(&self.agent_work).get(&agent) {
            return Ok(Arc::clone(held));
        }
        let read = Arc::new(
            self.store()
                .agent_work(agent, self.now())
                .await
                .map_err(|error| internal::<S>(&error))?,
        );
        locked(&self.agent_work).insert(agent, Arc::clone(&read));
        Ok(read)
    }

    /// The own field of the catalogue a member names, when it names one.
    pub(crate) async fn own_field_named(
        &self,
        member: &str,
    ) -> Result<Option<crate::fields::Definition>, AppError> {
        let Ok(id) = member.parse::<bb_core::id::FieldId>() else {
            return Ok(None);
        };
        Ok(self
            .field_catalogue()
            .await?
            .iter()
            .find(|definition| definition.field.id == id)
            .cloned())
    }

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
        let work = self.work_of_agent_now(agent).await?;
        let allowance = work.allowances.get(&issue).cloned().unwrap_or_default();
        let by_the_system = *locked(&self.accepting) == Some(issue);
        let allowed = members.iter().all(|member| {
            allowance.may_change(member, STATUS) || (by_the_system && member == STATUS)
        });
        if !allowed {
            return Err(ErrorCode::Forbidden.into());
        }
        // Whatever it was given: never an own field that holds people.
        for member in members {
            let named = self.own_field_named(member).await?;
            if named.is_some_and(|definition| definition.field.kind.holds_people()) {
                return Err(ErrorCode::Forbidden.into());
            }
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

    /// The work a search of the actor is cut to: for an agent, its own; `None` for a person.
    pub(crate) fn cut_to_work(&self, actor: &Actor) -> Option<crate::search::WorkOf> {
        actor.agent().map(|agent| crate::search::WorkOf {
            agent,
            at: self.now(),
        })
    }

    /// Refuses a new issue by an agent unless a hand-over lets it create sub-issues or a standing
    /// rule lets it create issues in that project.
    ///
    /// # Errors
    /// `access.forbidden` for an agent that may not create issues there.
    pub(crate) async fn ensure_agent_may_create(
        &self,
        actor: &Actor,
        project: bb_core::id::ProjectId,
    ) -> Result<(), AppError> {
        let Some(agent) = actor.agent() else {
            return Ok(());
        };
        let work = self.work_of_agent_now(agent).await?;
        if work.creates_in.contains(&project)
            || work
                .allowances
                .values()
                .any(|allowance| allowance.sub_issues)
        {
            Ok(())
        } else {
            Err(ErrorCode::Forbidden.into())
        }
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
        agent: AgentId,
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
