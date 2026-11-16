//! Moving the dates of several issues in one step (PLAN-03, ADR-0043, decision 6): the move that
//! *Move it after* of the plan makes, and a drag of several bars.
//!
//! Every issue is planned and written as a single change is ([`super::change::IssueWrite`]), so each
//! has its own change, its own event and the same rules: the field levels of the two dates, the
//! version it was read at, the order of start and due date. All of them are judged first, then
//! written in one transaction: an issue refused there (somebody changed it in the instant between)
//! refuses the whole move, which writes nothing and records no event. The changes carry one run
//! (`change_run`), written in the same transaction as they are, so *Undo* puts every date back in
//! one step, as it does for any other run.

use std::sync::{Arc, Mutex};

use bb_core::ErrorCode;
use bb_core::given::SetTo;
use bb_core::id::{IssueId, RunId};
use bb_core::many_changes::MANY_CHANGES_JOB_ABOVE;
use bb_core::many_changes::RunsToEnd;
use bb_core::run::{RunActor, RunEnd, RunKind};
use bb_policy::{Authorized, Masked, Place};
use jiff::civil::Date;
use serde::Serialize;

use super::change::IssueWrite;
use super::types::ChangeIssue;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity};
use crate::projects::Why;
use crate::relations::LineMaker;
use crate::runs::RunsOf;

/// The new dates of one issue in a move: what is `Some` is changed, and `SetTo::Nothing` takes a
/// date away.
pub struct DatesOf {
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
    /// The version of the issue the person looked at.
    pub version: i64,
    /// The new *Start date*.
    pub start_date: Option<SetTo<Date>>,
    /// The new *Due date*.
    pub due_date: Option<SetTo<Date>>,
}

/// Move the dates of several issues in one run.
pub struct MoveDates {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issues and their new dates.
    pub issues: Vec<DatesOf>,
}

/// One issue after its dates moved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct MovedIssue {
    /// The internal ID of the issue.
    #[masked(open)]
    pub id: IssueId,
    /// The readable ID now.
    #[masked(open)]
    pub key: String,
    /// The version the issue has now: what the next change of it is made on.
    #[masked(open)]
    pub version: i64,
}

/// The run that moved the dates, and the issues in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct DatesMoved {
    /// The run, which *Undo the run* takes back in one step.
    #[masked(open)]
    pub run: RunId,
    /// The issues, in the order they were named.
    #[masked(nested)]
    pub issues: Vec<MovedIssue>,
}

/// The events and the writes of the changes of a move; an issue whose dates the move leaves as they
/// are has neither.
fn written_of<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    actor: &Actor,
    plans: Vec<super::change::Plan>,
) -> Result<(Vec<bb_core::event::DomainEvent>, Vec<IssueWrite>), AppError> {
    let now = ctx.now();
    let mut events = Vec::new();
    let mut writes = Vec::new();
    for plan in plans {
        let changes = plan.field_changes();
        if changes.is_empty() {
            continue;
        }
        let changes = crate::event_members::issue_counted_up(changes, plan.current.version);
        let record = entity(bb_core::event::RecordKind::Issue, plan.current.id);
        events.push(ctx.event(by(actor.person.id), record, changes)?);
        writes.push(plan.into_write(now));
    }
    Ok((events, writes))
}

/// The change of one issue that a move makes.
fn change_of(token: &str, each: &DatesOf) -> ChangeIssue {
    ChangeIssue {
        start_date: each.start_date,
        due_date: each.due_date,
        ..ChangeIssue::of(token, each.issue.clone(), each.version)
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The issues of a move as they are now, once each.
    async fn moved_issues(
        &self,
        actor: &Actor,
        input: &MoveDates,
    ) -> Result<Vec<MovedIssue>, AppError> {
        let mut maker: LineMaker = self.line_maker().await?;
        let mut moved = Vec::with_capacity(input.issues.len());
        for each in &input.issues {
            let (issue, _) = self.visible_issue(actor, &each.issue).await?;
            let line = self
                .line_of(actor, &mut maker, &issue)
                .await?
                .ok_or(AppError::Refused(ErrorCode::NotFound))?;
            moved.push(MovedIssue {
                id: issue.id,
                key: line.key,
                version: issue.version,
            });
        }
        Ok(moved)
    }
}

/// Moves the dates of several issues in one run, written whole or not at all: nothing is written
/// and no event recorded when any of the changes is refused, also when somebody changed an issue in
/// the instant between the judging and the writing.
///
/// # Errors
/// `request.invalid` for no issue, more than a change to many issues takes, an issue twice or a
/// move that names no date; and the errors of [`change_issue`] for the first issue it refuses:
/// `resource.not-found`, `access.forbidden` (also for a date field the person may not change),
/// `issue.start-after-due`, `edit.version-changed`.
pub async fn move_dates<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MoveDates,
) -> Result<Authorized<DatesMoved>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let names_a_date = |each: &DatesOf| each.start_date.is_some() || each.due_date.is_some();
    if input.issues.is_empty()
        || input.issues.len() > MANY_CHANGES_JOB_ABOVE
        || !input.issues.iter().all(names_a_date)
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    // Judged first, so a refusal of any one issue stops the move before a date is written.
    // An issue named twice, by its readable ID and by its internal ID, is the same issue: a second
    // change would be made on a version the first one has counted up.
    let mut seen = std::collections::BTreeSet::new();
    let mut plans = Vec::with_capacity(input.issues.len());
    for each in &input.issues {
        let (_, plan) = ctx
            .plan_issue_change(&actor, &change_of(&input.session_token, each))
            .await?;
        if !seen.insert(plan.current.id) {
            return Err(ErrorCode::InvalidRequest.into());
        }
        plans.push(plan);
    }
    let run =
        RunId::generate(&mut ctx.ids()).map_err(|error| AppError::Internal(error.to_string()))?;
    // The events carry the run, so *Undo the run* finds them.
    ctx.set_run(Some(run));
    let prepared = written_of(ctx, &actor, plans);
    ctx.set_run(None);
    let (changed, mut writes) = prepared?;
    // In the order of their IDs, whatever the order of the request: two moves of the same issues
    // take their rows in one order, and never wait for each other in a circle.
    writes.sort_by_key(IssueWrite::issue);
    // The run, its changes and its end in one transaction: written whole, or refused whole, with no
    // run left behind that has nothing to undo, and no move saved without its run.
    let by_actor = RunActor {
        person: actor.person.id,
        app: None,
        agent: actor.agent(),
    };
    let runs = RunsOf::new(ctx, &actor, &input.session_token);
    let (stored, opened) = runs.opening(run, RunKind::ManyChanges, &by_actor, ctx.now())?;
    let done = RunEnd::Done.status();
    let ended = ctx.run_end_event(run, done, by_actor.event_actor())?;
    let events: Vec<_> = std::iter::once(opened)
        .chain(changed)
        .chain(std::iter::once(ended))
        .collect();
    let (why, failed) = (Why::default(), Arc::new(Mutex::new(None)));
    let (stopped, stopped_at, all, now) =
        (why.clone(), Arc::clone(&failed), writes.clone(), ctx.now());
    ctx.store()
        .record_changes(&events, move |transaction| {
            Box::pin(async move {
                S::insert_many_changes_run(transaction, &stored).await?;
                for (at, write) in all.iter().enumerate() {
                    if let Err(error) = write.write_in::<S>(transaction, &stopped).await {
                        *stopped_at
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(at);
                        return Err(error);
                    }
                }
                S::end_many_changes_runs(transaction, &RunsToEnd::One(run), done, now)
                    .await
                    .map(drop)
            })
        })
        .await
        .map_err(|error| {
            let at = *failed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            // Only an issue that refused itself is named; a failed commit names none.
            at.and_then(|at| writes.get(at)).map_or_else(
                || why.answer::<S>(&error, ErrorCode::VersionChanged),
                |write| write.refused::<S>(&why, &error, actor.person.id),
            )
        })?;
    let issues = ctx.moved_issues(&actor, &input).await?;
    Ok(held.authorize(DatesMoved { run, issues }))
}
