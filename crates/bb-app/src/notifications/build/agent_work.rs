//! The notifications of agent work (ADR-0033, amendment M7): a question, a plan that waits, a
//! result ready for review, an agent that gave up, an agent that was stopped and a standing rule
//! that runs out soon. A part of the reader of the outbox, kept apart so the reader stays small.
//!
//! The responsible person and the person who handed the issue over hear of the first four; the
//! reader skips an issue a person no longer sees like any other (decision 2).

use bb_core::agent_work::AgentWorkState;
use bb_core::event::DomainEvent;
use bb_core::id::{AgentId, AgentWorkId, IssueId, StandingRuleId, UserId};
use bb_core::notification::{NotificationKind, Once};

use super::{Hears, Keeps, Round, Subject};
use crate::agent_work::{event_member, waiting_for};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::event_members::text_set;

/// The person who did it as themselves; the person of an agent still hears of what it did.
pub(super) const fn doer_in_person(event: &DomainEvent) -> Option<UserId> {
    match event.actor {
        bb_core::event::Actor::Person(person) => Some(person),
        _ => None,
    }
}

/// The kind a new state of agent work is told as, if it is told at all.
fn kind_of(state: &str, waiting: Option<&str>) -> Option<NotificationKind> {
    match (AgentWorkState::from_code(state)?, waiting) {
        (AgentWorkState::WaitingForYou, Some(waiting_for::PLAN)) => {
            Some(NotificationKind::PlanWaits)
        }
        (AgentWorkState::WaitingForYou, _) => Some(NotificationKind::AgentAsks),
        (AgentWorkState::ReadyForReview, _) => Some(NotificationKind::ReadyForReview),
        (AgentWorkState::AgentGaveUp, _) => Some(NotificationKind::AgentGaveUp),
        _ => None,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// A change of the state of the work on an issue: the people who answer for it hear of what
    /// waits for them.
    pub(super) async fn agent_work_event(
        &self,
        round: &mut Round,
        event: &DomainEvent,
    ) -> Result<(), AppError> {
        let Some(state) = text_set(&event.changes, event_member::WORK_STATE) else {
            return Ok(());
        };
        let waiting = text_set(&event.changes, event_member::WAITING_FOR);
        let Some(kind) = kind_of(state, waiting) else {
            return Ok(());
        };
        let Ok(work) = AgentWorkId::from_bytes(event.entity.id.to_bytes()) else {
            return Ok(());
        };
        let failed = |error| internal::<S>(&error);
        let Some(issue_id) = text_set(&event.changes, event_member::ISSUE)
            .and_then(|text| text.parse::<IssueId>().ok())
        else {
            return Ok(());
        };
        let Some(issue) = self.store().issue(issue_id).await.map_err(failed)? else {
            return Ok(());
        };
        let people = self.store().people_of_work(work).await.map_err(failed)?;
        let subject = Subject {
            event,
            issue: Some(&issue),
            record: Some(event.entity.id),
            keeps: Keeps::Nothing,
        };
        let hears = people
            .into_iter()
            .map(|person| Hears {
                person,
                kinds: vec![Some(kind)],
                follows: false,
                once: Once::Event(event.id),
            })
            .collect();
        self.deliver(round, &subject, hears).await
    }

    /// An agent was stopped, by its person, by an administrator, or because its person lost access:
    /// the person responsible for it hears of it.
    pub(super) async fn agent_event(
        &self,
        round: &mut Round,
        event: &DomainEvent,
    ) -> Result<(), AppError> {
        let stopped = text_set(&event.changes, "state")
            == Some(bb_core::agent::AgentState::Stopped.code())
            || text_set(&event.changes, event_member::WORK_STATE)
                == Some(AgentWorkState::Stopped.code());
        if !stopped {
            return Ok(());
        }
        let Ok(agent) = AgentId::from_bytes(event.entity.id.to_bytes()) else {
            return Ok(());
        };
        let Some(found) = self
            .store()
            .agent(agent)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(());
        };
        let subject = Subject {
            event,
            issue: None,
            record: Some(event.entity.id),
            keeps: Keeps::Nothing,
        };
        let hears = vec![Hears {
            person: found.person,
            kinds: vec![Some(NotificationKind::AgentStopped)],
            follows: false,
            once: Once::Event(event.id),
        }];
        self.deliver(round, &subject, hears).await
    }

    /// Three days before a standing rule runs out its person is told, once.
    pub(super) async fn standing_rule_event(
        &self,
        round: &mut Round,
        event: &DomainEvent,
    ) -> Result<(), AppError> {
        if text_set(&event.changes, event_member::STATE) != Some(event_member::ABOUT_TO_RUN_OUT) {
            return Ok(());
        }
        let Ok(rule) = StandingRuleId::from_bytes(event.entity.id.to_bytes()) else {
            return Ok(());
        };
        let Some(found) = self
            .store()
            .standing_rule(rule)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(());
        };
        let subject = Subject {
            event,
            issue: None,
            record: Some(event.entity.id),
            keeps: Keeps::Nothing,
        };
        let hears = vec![Hears {
            person: found.person,
            kinds: vec![Some(NotificationKind::RuleRunsOutSoon)],
            follows: false,
            once: Once::Event(event.id),
        }];
        self.deliver(round, &subject, hears).await
    }

    /// An issue changed: remembers the projects, of the project it is in and above, where a
    /// standing rule works, so the job of the matches is queued for them after the round.
    pub(super) async fn note_projects_with_rules(
        &self,
        round: &mut Round,
        issue: &bb_core::issue::Issue,
    ) -> Result<(), AppError> {
        if round.rule_scope.is_none() {
            let rules = self
                .store()
                .open_rules()
                .await
                .map_err(|error| internal::<S>(&error))?;
            round.rule_scope = Some(rules.into_iter().map(|rule| rule.project).collect());
        }
        let scope = round.rule_scope.clone().unwrap_or_default();
        if scope.is_empty() {
            return Ok(());
        }
        for project in self.chain_of(issue.project).await? {
            if scope.contains(&project) {
                round.rule_projects.insert(project);
            }
        }
        Ok(())
    }

    /// Queues the job that writes down what newly comes under the rules of each project.
    pub(super) async fn queue_rule_matches(
        &self,
        projects: &std::collections::BTreeSet<bb_core::id::ProjectId>,
    ) -> Result<(), AppError> {
        for project in projects {
            let job = self.queued(
                crate::jobs::JobKind::RuleMatches,
                serde_json::json!({ crate::agent_work::job_payload::PROJECT: project.to_string() }),
                self.now(),
            )?;
            self.store()
                .queue_job(job.id, &job.kind, &job.payload, job.run_at, self.now())
                .await
                .map_err(|error| internal::<S>(&error))?;
        }
        Ok(())
    }
}
