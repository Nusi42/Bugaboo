//! Ending what gives an agent work, and stopping the work (AI-31, AI-33): a hand-over that runs
//! out, an agent that is stopped, a person who loses access.

use bb_core::ErrorCode;
use bb_core::agent_records::{GrantState, HandOver, IssueWork};
use bb_core::agent_work::AgentWorkState;
use bb_core::event::{Actor as EventActor, RecordKind};
use bb_core::id::{AgentId, HandOverId};
use serde_json::{Value, json};

use super::members::{member, payload};
use crate::auth::{AppError, AuthStore, Ctx, entity, internal, set_to};

/// Ends the hand-over and stops the work of it that is open, inside a change. A hand-over that
/// changed since it was read stops the change.
pub(crate) async fn end_and_stop<S: AuthStore>(
    transaction: &mut S::Transaction,
    hand_over: &HandOver,
    works: &[IssueWork],
) -> Result<(), S::Error> {
    if !S::end_hand_over(transaction, hand_over.id, hand_over.version).await? {
        return Err(S::aborted());
    }
    for work in works.iter().filter(|work| work.state.is_open()) {
        if !S::set_work_state(
            transaction,
            work.id,
            AgentWorkState::Stopped,
            None,
            false,
            work.version,
        )
        .await?
        {
            return Err(S::aborted());
        }
    }
    Ok(())
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The work of the job of a hand-over that ran out: ends the hand-over and stops its open
    /// work. A hand-over that ended before, or is not there, leaves nothing to do.
    ///
    /// # Errors
    /// `request.invalid` for a payload that names no hand-over, an internal error when the store
    /// fails.
    pub(crate) async fn run_hand_over_runs_out(&self, job: &Value) -> Result<(), AppError> {
        let id: HandOverId = job
            .get(payload::HAND_OVER)
            .and_then(Value::as_str)
            .and_then(|text| text.parse().ok())
            .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
        let Some(hand_over) = self
            .store()
            .hand_over(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|found| found.state == GrantState::Open && found.ends_at <= self.now())
        else {
            return Ok(());
        };
        self.end_hand_over_of(&hand_over).await
    }

    /// Ends one hand-over and stops its open work, as the system.
    pub(crate) async fn end_hand_over_of(&self, hand_over: &HandOver) -> Result<(), AppError> {
        let works = self
            .store()
            .works_of_hand_over(hand_over.id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let event = self.event(
            EventActor::System,
            entity(RecordKind::HandOver, hand_over.id),
            vec![
                set_to(member::STATE, json!(GrantState::Ended.code())),
                set_to(
                    member::ISSUES,
                    json!(
                        works
                            .iter()
                            .map(|w| w.issue.to_string())
                            .collect::<Vec<_>>()
                    ),
                ),
                bb_core::version::counted_up(hand_over.version),
            ],
        )?;
        let kept = hand_over.clone();
        let result = self
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move { end_and_stop::<S>(transaction, &kept, &works).await })
            })
            .await;
        match result {
            // Somebody changed the work at the same moment: the next look finds it ended or not.
            Err(error) if S::is_aborted(&error) => Ok(()),
            other => other.map_err(|error| internal::<S>(&error)),
        }
    }

    /// Ends every hand-over and rule of the agent and stops its open work, because the agent was
    /// stopped or its person lost access (ADR-0029, decision 2).
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn stop_work_of_agent(&self, agent: AgentId) -> Result<(), AppError> {
        let event = self.event(
            EventActor::System,
            entity(RecordKind::Agent, agent),
            vec![set_to(
                member::WORK_STATE,
                json!(AgentWorkState::Stopped.code()),
            )],
        )?;
        self.store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    S::stop_work_of_agent(transaction, agent).await?;
                    S::end_grants_of_agent(transaction, agent).await
                })
            })
            .await
            .map_err(|error| internal::<S>(&error))
    }
}
