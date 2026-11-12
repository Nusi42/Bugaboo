//! Moving the dates of several issues in one step (PLAN-03, ADR-0043, decision 6): the move that
//! *Move it after* of the plan makes, and a drag of several bars.
//!
//! Every issue is changed by the use case of a single change, so each has its own change, its own
//! event and the same rules: the field levels of the two dates, the version it was read at, the
//! order of start and due date. All of them are judged first, so one refusal stops the whole move
//! before anything is written. The changes then carry one run (`change_run`), so *Undo* puts every
//! date back in one step, as it does for any other run.

use bb_core::ErrorCode;
use bb_core::given::SetTo;
use bb_core::id::{IssueId, RunId};
use bb_core::many_changes::MANY_CHANGES_JOB_ABOVE;
use bb_core::run::{RunActor, RunEnd, RunKind};
use bb_policy::{Authorized, Masked, Place};
use jiff::civil::Date;
use serde::Serialize;

use super::change::change_issue;
use super::types::ChangeIssue;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::relations::LineMaker;
use crate::runs::{RunStore, RunsOf};

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

/// Moves the dates of several issues in one run. Nothing is written when any of the changes is
/// refused; a change that fails after the others were written (somebody changed an issue in the
/// instant between) ends the run as failed and leaves what was written, which *Undo* takes back.
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
    // change would be made on a version the first one has counted up, after part of the move was
    // written.
    let mut seen = std::collections::BTreeSet::new();
    for each in &input.issues {
        let (_, plan) = ctx
            .plan_issue_change(&actor, &change_of(&input.session_token, each))
            .await?;
        if !seen.insert(plan.current.id) {
            return Err(ErrorCode::InvalidRequest.into());
        }
    }
    let run =
        RunId::generate(&mut ctx.ids()).map_err(|error| AppError::Internal(error.to_string()))?;
    let runs = RunsOf::new(ctx, &actor, &input.session_token);
    let by = RunActor {
        person: actor.person.id,
        app: None,
        agent: actor.agent(),
    };
    runs.open_run(run, RunKind::ManyChanges, &by, ctx.now())
        .await?;
    ctx.set_run(Some(run));
    let mut written = Ok(());
    for each in &input.issues {
        let change = change_of(&input.session_token, each);
        if let Err(error) = change_issue(ctx, change).await {
            written = Err(error);
            break;
        }
    }
    ctx.set_run(None);
    let ended = if written.is_ok() {
        RunEnd::Done
    } else {
        RunEnd::Failed
    };
    let closed = runs.close_run(run, ended, ctx.now()).await;
    // The refusal of the change comes first: it is why the run failed.
    written?;
    closed?;
    let issues = ctx.moved_issues(&actor, &input).await?;
    Ok(held.authorize(DatesMoved { run, issues }))
}
