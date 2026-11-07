//! What an agent may change on an issue it works on (ADR-0029, decision 3): the ticks of the
//! hand-over and the fields of the standing rule. Never a member that holds people (COMP-10), and
//! the status only when it was ticked. Reading, commenting, reporting, attaching and linking
//! related issues need nothing.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::agent_work::{Allowance, issue_member as key};
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

/// Whether a request sets a member.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Is {
    /// It is left out.
    #[default]
    Left,
    /// It is set.
    Set,
}

impl Is {
    const fn of(set: bool) -> Self {
        if set { Self::Set } else { Self::Left }
    }
}

/// Which members of an issue a request sets: the one list, for a change of an issue and for the
/// input of a new one (which leaves out the type and the title, always its own, and the time left).
#[derive(Default)]
struct Sets {
    issue_type: Is,
    title: Is,
    status: Is,
    priority: Is,
    estimated_time: Is,
    time_left: Is,
    assigned_to: Is,
}

impl Sets {
    /// The keys of the members set, and the own fields.
    fn members<'a>(&self, fields: impl Iterator<Item = &'a String>) -> Vec<String> {
        [
            (self.issue_type, key::ISSUE_TYPE),
            (self.title, key::TITLE),
            (self.status, STATUS),
            (self.priority, key::PRIORITY),
            (self.estimated_time, key::ESTIMATED_TIME),
            (self.time_left, key::TIME_LEFT),
            (self.assigned_to, key::ASSIGNED_TO),
        ]
        .into_iter()
        .filter(|(set, _)| *set == Is::Set)
        .map(|(_, key)| key.to_owned())
        .chain(fields.cloned())
        .collect()
    }
}

/// The members of the issue a change names.
fn members_of(input: &ChangeIssue) -> Vec<String> {
    Sets {
        issue_type: Is::of(input.issue_type.is_some()),
        title: Is::of(input.title.is_some()),
        status: Is::of(input.status.is_some()),
        priority: Is::of(input.priority.is_some()),
        estimated_time: Is::of(input.estimated_time.is_some()),
        time_left: Is::of(input.time_left.is_some()),
        assigned_to: Is::of(input.assigned_to.is_some()),
    }
    .members(input.fields.keys())
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
        let accepted_status = by_the_system && members.iter().any(|member| member == STATUS);
        let others: Vec<String> = members
            .iter()
            .filter(|member| !(accepted_status && *member == STATUS))
            .cloned()
            .collect();
        self.ensure_allowance_covers(&allowance, &others).await?;
        if let Some(run) = self.run_of_attempt_on(agent, issue).await? {
            self.set_run(Some(run));
        }
        Ok(())
    }

    /// Refuses members the allowance does not cover: the one place that judges them, for a change
    /// of an issue and for the input of a new one. A member that holds people is never covered,
    /// whatever was given, and neither is an own field that holds people by its kind.
    async fn ensure_allowance_covers(
        &self,
        allowance: &Allowance,
        members: &[String],
    ) -> Result<(), AppError> {
        if !members
            .iter()
            .all(|member| allowance.may_change(member, STATUS))
        {
            return Err(ErrorCode::Forbidden.into());
        }
        for member in members {
            let named = self.own_field_named(member).await?;
            if named.is_some_and(|definition| definition.field.kind.holds_people()) {
                return Err(ErrorCode::Forbidden.into());
            }
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

    /// The issue a new issue goes under, when the input names one, and what an agent may do there.
    /// A person needs the right to change the parent. An agent creates only under an issue it was
    /// handed with *create sub-issues* ticked, in the project of that issue, or, with none named,
    /// in a project a standing rule of it creates in; and the members it sets must be covered the
    /// way a change covers them (people never, the status only when ticked, fields only when named).
    ///
    /// # Errors
    /// `resource.not-found` for a parent that does not exist or is not seen, `request.invalid` for
    /// a parent in another project, `access.forbidden` for what the actor may not do.
    pub(crate) async fn parent_for_create(
        &self,
        actor: &Actor,
        input: &crate::issues::CreateIssue,
        project: bb_core::id::ProjectId,
    ) -> Result<Option<IssueId>, AppError> {
        let parent = self.named_parent(actor, input.parent.as_deref()).await?;
        if parent
            .as_ref()
            .is_some_and(|found| found.project != project)
        {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let parent = parent.map(|found| found.id);
        let Some(agent) = actor.agent() else {
            return Ok(parent);
        };
        // The work is read before the change that makes the issue: a hand-over that ends between
        // the two still lets this one through, as it does for every change an agent makes.
        let work = self.work_of_agent_now(agent).await?;
        let allowance = match parent {
            Some(parent) => work
                .allowances
                .get(&parent)
                .filter(|allowance| allowance.sub_issues)
                .cloned()
                .ok_or(AppError::Refused(ErrorCode::Forbidden))?,
            None if work.creates_in.contains(&project) => Allowance::default(),
            None => return Err(ErrorCode::Forbidden.into()),
        };
        let members = Sets {
            status: Is::of(input.status.is_some()),
            priority: Is::of(input.priority.is_some()),
            estimated_time: Is::of(input.estimated_time.is_some()),
            assigned_to: Is::of(input.assigned_to.is_some()),
            ..Sets::default()
        }
        .members(input.fields.keys());
        self.ensure_allowance_covers(&allowance, &members).await?;
        Ok(parent)
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
