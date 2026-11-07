//! Ending what gives an agent work, and stopping the work (AI-31, AI-33): a hand-over that runs
//! out, an agent that is stopped, a person who loses access.

use bb_core::ErrorCode;
use bb_core::agent_records::{GivingState, HandOver, IssueWork};
use bb_core::agent_work::AgentWorkState;
use bb_core::event::{Actor as EventActor, RecordKind};
use bb_core::id::{AgentId, HandOverId};
use serde_json::{Value, json};

use super::members::{member, payload};
use crate::auth::{AppError, AuthStore, Ctx, entity, internal, set_to};

/// Which work a review or an accept by itself closes, and how.
pub(crate) struct Closing {
    /// The work.
    pub(crate) work: bb_core::id::AgentWorkId,
    /// The version it has when it is closed.
    pub(crate) version: i64,
    /// Whether it was accepted by itself and still waits for a person to look at it.
    pub(crate) accepted_by_itself: bool,
    /// Whether the status stays and a person chooses the next one.
    pub(crate) status_pending: bool,
}

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
    for work in works
        .iter()
        .filter(|work| !work.closed && work.state.is_open())
    {
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

/// Closes the work, which is at `version`, with a review. The hand-over ends with its last piece of
/// open work, which is read inside the change, after the work is closed, so two reviews of the two
/// last pieces at the same moment cannot both leave the hand-over open. Work that changed since it
/// was read stops the change.
pub(crate) async fn close_work_of<S: AuthStore>(
    transaction: &mut S::Transaction,
    hand_over: &HandOver,
    closing: Closing,
) -> Result<(), S::Error> {
    let Closing {
        work,
        version,
        accepted_by_itself,
        status_pending,
    } = closing;
    if !S::close_work(
        transaction,
        work,
        accepted_by_itself,
        status_pending,
        version,
    )
    .await?
    {
        return Err(S::aborted());
    }
    let others_open = S::works_of_hand_over_in(transaction, hand_over.id)
        .await?
        .iter()
        .any(|other| other.id != work && !other.closed && other.state.is_open());
    if !others_open && !S::end_hand_over(transaction, hand_over.id, hand_over.version).await? {
        return Err(S::aborted());
    }
    Ok(())
}

/// How often the end of a hand-over is tried when it changes meanwhile.
const TRIES: usize = 3;

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
            .filter(|found| found.state == GivingState::Open && found.ends_at <= self.now())
        else {
            return Ok(());
        };
        self.end_hand_over_of(&hand_over).await
    }

    /// Ends one hand-over and stops its open work, as the system. When the hand-over or its work
    /// changed since they were read, both are read again and the end tried again, a few times; a
    /// hand-over that kept changing is an error, so the job runs again later.
    pub(crate) async fn end_hand_over_of(&self, hand_over: &HandOver) -> Result<(), AppError> {
        let failed = |error| internal::<S>(&error);
        let mut current = hand_over.clone();
        for _ in 0..TRIES {
            let works = self
                .store()
                .works_of_hand_over(current.id)
                .await
                .map_err(failed)?;
            let event = self.event(
                EventActor::System,
                entity(RecordKind::HandOver, current.id),
                vec![
                    set_to(member::STATE, json!(GivingState::Ended.code())),
                    set_to(
                        member::ISSUES,
                        json!(
                            works
                                .iter()
                                .map(|w| w.issue.to_string())
                                .collect::<Vec<_>>()
                        ),
                    ),
                    bb_core::version::counted_up(current.version),
                ],
            )?;
            let kept = current.clone();
            let result = self
                .store()
                .record_change(&event, move |transaction| {
                    Box::pin(async move { end_and_stop::<S>(transaction, &kept, &works).await })
                })
                .await;
            match result {
                Err(error) if S::is_aborted(&error) => {
                    // Somebody changed it at the same moment: look again at what is stored now.
                    match self
                        .store()
                        .hand_over(current.id)
                        .await
                        .map_err(failed)?
                        .filter(|found| found.state == GivingState::Open)
                    {
                        Some(found) => current = found,
                        None => return Ok(()),
                    }
                }
                other => return other.map_err(failed),
            }
        }
        Err(AppError::Internal(
            "a hand-over kept changing while it was ended".to_owned(),
        ))
    }

    /// Ends every hand-over and rule of the agent and stops its open work, because the agent was
    /// stopped or its person lost access (ADR-0029, decision 2).
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn stop_work_of_agent(&self, agent: AgentId) -> Result<(), AppError> {
        // Nothing left to end: no event and no notification (a refused request asks every time).
        if !self
            .store()
            .agent_has_open_giving(agent)
            .await
            .map_err(|error| internal::<S>(&error))?
        {
            return Ok(());
        }
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
