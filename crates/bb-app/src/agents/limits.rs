//! The limits of an agent (AI-37, ADR-0029, decision 9): changes a minute, issues at once and
//! attempts on one issue. They are counted here and nowhere else.
//!
//! A request that would pass a limit does not start. It pauses the agent and answers with
//! `agent.limit-reached`; the sentence of the limit (*"Fixer paused: 60 changes in a minute"*) is on
//! the agent, and its person is told. A change that has started goes on to its end: the count is
//! made before the change begins, so no change stops halfway. The person lets the agent work again
//! with *Resume*; an agent that is paused is refused at its next request like one that is stopped.

use std::sync::atomic::Ordering;
use std::time::Instant;

use bb_core::ErrorCode;
use bb_core::agent::{Agent, AgentState, LimitKind};
use bb_core::event::{Actor as EventActor, FieldChange, RecordKind};
use bb_core::id::AgentId;
use bb_core::version::counted_up;
use serde_json::json;

use crate::agent_work::event_member;
use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, entity, internal, locked};
use crate::rate_window::{Overflow, Window};

/// The changes of every agent in the last minute, by agent. An agent key counts toward its agent,
/// so many keys cannot add up to more than the agent may do. A full table refuses an agent it does
/// not know yet, so a flood of invented agents cannot use up the memory.
pub(crate) struct AgentRate {
    changes: Window<AgentId>,
}

impl Default for AgentRate {
    fn default() -> Self {
        Self {
            changes: Window::new(u32::MAX, Overflow::Refuse),
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Says that the request makes a change, so that when it is known to come from an agent (when
    /// its key is read) the agent is checked against its limit of changes a minute, and when the
    /// change has been made (`settle_agent_change`) it is counted. The use cases that change
    /// something are told so in one place (`use_cases!`); a tool that changes something says it for
    /// itself.
    pub(crate) fn expect_change(&self) {
        self.change_due.store(true, Ordering::Relaxed);
    }

    /// Lets the agent start again from nothing: its person resumed it after a pause.
    pub(crate) fn forget_agent_changes(&self, agent: AgentId) {
        self.services.agent_rate.changes.forget(&agent);
    }

    /// Checks the agent against its limit of changes a minute, once for a request that makes a
    /// change. Over the limit the agent is paused and the request is refused before anything is
    /// written. A request that makes no change is not checked.
    ///
    /// # Errors
    /// `agent.limit-reached` when one more change would pass the limit.
    pub(crate) async fn count_agent_change(&self, agent: &Agent) -> Result<(), AppError> {
        if !self.change_due.swap(false, Ordering::Relaxed) {
            return Ok(());
        }
        let limit = u32::try_from(agent.changes_a_minute.max(0)).unwrap_or(u32::MAX);
        if self
            .services
            .agent_rate
            .changes
            .room_within(&agent.id, Instant::now(), limit)
            .is_err()
        {
            return Err(self.pause_for_limit(agent, LimitKind::ChangesAMinute).await);
        }
        *locked(&self.change_pending) = Some(agent.id);
        Ok(())
    }

    /// Counts the change the request made, when it was made: a request that was refused counted
    /// nothing, so an agent that tries what it may not is not paused for it. The count is made once
    /// for a request.
    pub(crate) fn settle_agent_change(&self) {
        if let Some(agent) = locked(&self.change_pending).take() {
            let _ = self
                .services
                .agent_rate
                .changes
                .hit_within(agent, Instant::now(), u32::MAX);
        }
    }

    /// Refuses the start of work that would pass a limit of a number of things (issues at once,
    /// attempts on one issue): `held` is how many the agent holds now. A request that only asks
    /// what would happen (`writing` is false) pauses nobody.
    ///
    /// # Errors
    /// `agent.limit-reached` when `held` has reached the limit.
    pub(crate) async fn ensure_agent_room(
        &self,
        agent: &Agent,
        kind: LimitKind,
        held: usize,
        writing: bool,
    ) -> Result<(), AppError> {
        let limit = agent.limits().of(kind);
        if i64::try_from(held).unwrap_or(i64::MAX) < limit {
            return Ok(());
        }
        if writing {
            Err(self.pause_for_limit(agent, kind).await)
        } else {
            // A preview says the limit is reached and pauses nobody.
            Err(AppError::Refused(ErrorCode::AgentLimitReached))
        }
    }

    /// Pauses the agent because it reached `kind` and gives the answer for the request that
    /// reached it. The pause is its own change with its own event, so it stays when the request
    /// that caused it is refused. An agent that is not active any more (its person paused or
    /// stopped it meanwhile) is left as it is.
    pub(crate) async fn pause_for_limit(&self, agent: &Agent, kind: LimitKind) -> AppError {
        if let Err(error) = self.write_pause(agent.id, kind).await {
            tracing::warn!(cause = %error, "an agent could not be paused");
        }
        AppError::Refused(ErrorCode::AgentLimitReached)
    }

    async fn write_pause(&self, id: AgentId, kind: LimitKind) -> Result<(), AppError> {
        // The record is read again: the agent may have changed since the request began, and the
        // version decides which of two changes at the same moment wins.
        let Some(found) = self
            .store()
            .agent(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|found| found.state == AgentState::Active)
        else {
            return Ok(());
        };
        let event = self.event(
            EventActor::Agent {
                agent: id,
                person: found.person,
            },
            entity(RecordKind::Agent, id),
            vec![
                FieldChange {
                    field: "state".to_owned(),
                    before: Some(json!(found.state.code())),
                    after: Some(json!(AgentState::Paused.code())),
                },
                FieldChange {
                    field: event_member::PAUSED_FOR.to_owned(),
                    before: None,
                    after: Some(json!(kind.code())),
                },
                counted_up(found.version),
            ],
        )?;
        let version = found.version;
        self.store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    if S::set_agent_state(transaction, id, AgentState::Paused, Some(kind), version)
                        .await?
                    {
                        Ok(())
                    } else {
                        Err(S::aborted())
                    }
                })
            })
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))
    }
}
