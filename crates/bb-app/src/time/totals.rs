//! The time of an issue and of a project (TIME-03, PRIV-13, ADR-0035, decision 4): an issue's own
//! *Estimated time*, *Time left* and *Time spent*, and the totals with its sub-issues at every level
//! below it; a project's totals of its issues and its sub-projects.
//!
//! A total counts only what the reader may see, and only the fields they may read: it is the
//! compiled search of the list (`under WEB-142` for an issue, the list of the project for a
//! project), with the reader's sight and readable fields in it, so a hidden issue and a hidden
//! field change nothing in it (ADR-0004). The sums of the estimate and the time left are one
//! statement of that search; the time spent is read through the port of time spent, which reuses
//! the same search. No total walks the tree in Rust.

use bb_core::ErrorCode;
use bb_core::fields::Field;
use bb_core::readable_id::ReadableId;
use bb_core::time::{Minutes, TimeLeft, time_left};
use bb_policy::{Authorized, FieldMask, Masked};
use serde::Serialize;

use crate::access::{Actor, place_of_address};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::search::{SearchRequest, SearchScope, stopped, target_of};

/// Ask for the time of one issue.
pub struct IssueTimeQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
}

/// Ask for the totals of the time of a project.
pub struct ProjectTimeQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, by its ID or its short name (an old one too).
    pub project: String,
}

/// The time of one issue, as the reader may read it: a member is left out for a field the reader
/// may not read, and is empty while there is nothing to show.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct IssueTime {
    /// *Estimated time*.
    #[masked(field = "estimated-time")]
    pub estimated_time: Option<Minutes>,
    /// *Time left*: typed, or worked out from the estimate and the time spent.
    #[masked(field = "time-left")]
    pub time_left: Option<TimeLeft>,
    /// *Time spent*: the total of the issue's time entries.
    #[masked(field = "time-spent")]
    pub time_spent: Option<Minutes>,
    /// The totals of the issue with its sub-issues at every level below it; none for an issue
    /// without sub-issues the reader sees.
    #[masked(nested)]
    pub with_sub_issues: Option<TimeTotals>,
}

/// The totals of the time of many issues, over the issues the reader sees and of the fields they
/// may read in all of them: a member is left out for a field they may not read, and is empty when
/// no issue has a value.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Masked)]
pub struct TimeTotals {
    /// The sum of *Estimated time*.
    #[masked(field = "estimated-time")]
    pub estimated_time: Option<Minutes>,
    /// The sum of *Time left*, each issue's typed or worked out.
    #[masked(field = "time-left")]
    pub time_left: Option<Minutes>,
    /// The sum of *Time spent*.
    #[masked(field = "time-spent")]
    pub time_spent: Option<Minutes>,
}

/// Two lengths added, either missing: none only when both are.
fn plus(one: Option<Minutes>, other: Option<Minutes>) -> Option<Minutes> {
    match (one, other) {
        (Some(one), Some(other)) => Some(one.checked_add(other).unwrap_or(one)),
        (one, other) => one.or(other),
    }
}

impl TimeTotals {
    /// These totals with the numbers of one more issue.
    fn with(self, other: &Self) -> Self {
        Self {
            estimated_time: plus(self.estimated_time, other.estimated_time),
            time_left: plus(self.time_left, other.time_left),
            time_spent: plus(self.time_spent, other.time_spent),
        }
    }
}

/// Whether the field is among the readable fields of the request: a total is made only of those.
fn readable(request: &SearchRequest, field: Field) -> bool {
    target_of(&request.fields, &field.search_word()).is_some()
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The totals of the issues a search finds, for the reader the search was prepared for; none
    /// when it finds none.
    async fn totals_of(&self, request: &SearchRequest) -> Result<Option<TimeTotals>, AppError> {
        let lengths = self
            .store()
            .search_lengths(request)
            .await
            .map_err(stopped)?;
        if !lengths.found_any {
            return Ok(None);
        }
        let time_spent = if readable(request, Field::TimeSpent) {
            Some(
                self.store()
                    .spent_on_search(request)
                    .await
                    .map_err(stopped)?,
            )
        } else {
            None
        };
        Ok(Some(TimeTotals {
            estimated_time: lengths.estimated,
            time_left: lengths.time_left,
            time_spent,
        }))
    }

    /// The totals of a search the actor makes, where it looks.
    async fn totals_found(
        &self,
        actor: &Actor,
        text: &str,
        scope: SearchScope,
    ) -> Result<Option<TimeTotals>, AppError> {
        let prepared = self.prepare_search(actor, text, scope).await?;
        self.totals_of(&prepared.request).await
    }
}

/// The issue's own numbers, as the mask of its type lets the reader read them.
fn own_numbers(
    mask: &FieldMask,
    estimated: Option<Minutes>,
    left: Option<TimeLeft>,
    spent: Minutes,
) -> TimeTotals {
    TimeTotals {
        estimated_time: estimated.filter(|_| mask.can_see(Field::EstimatedTime)),
        time_left: left
            .map(TimeLeft::minutes)
            .filter(|_| mask.can_see(Field::TimeLeft)),
        time_spent: mask.can_see(Field::TimeSpent).then_some(spent),
    }
}

/// The time of one issue: its own numbers and, when it has sub-issues the reader sees, the totals
/// with them.
///
/// # Errors
/// `resource.not-found` for an issue that does not exist, is in the trash or that the person may
/// not see; `search.too-slow` when the totals take longer than the limit of a search.
pub async fn issue_time<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    query: IssueTimeQuery,
) -> Result<Authorized<IssueTime>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &query.issue).await?;
    let mask = held.field_mask_for(Some(issue.issue_type));
    let spent = ctx
        .store()
        .spent_on_issues(&[issue.id])
        .await
        .map_err(|error| internal::<S>(&error))?
        .first()
        .map(|(_, minutes)| *minutes)
        .unwrap_or_default();
    let left = time_left(issue.estimated_time, spent, issue.time_left);
    let own = own_numbers(&mask, issue.estimated_time, left, spent);
    let project = ctx
        .store()
        .project(issue.project)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let key = ReadableId::new(project.short_name, issue.code.clone());
    // The sub-issues the reader sees, through issues they read, in the project of the issue and
    // below it: the search of `under`, so the walk is the compiler's.
    let below = ctx
        .totals_found(
            &actor,
            &format!("under {key}"),
            SearchScope::Within(issue.project),
        )
        .await?;
    Ok(held.authorize(IssueTime {
        estimated_time: own.estimated_time,
        time_left: left.filter(|_| mask.can_see(Field::TimeLeft)),
        time_spent: own.time_spent,
        with_sub_issues: below.map(|below| own.with(&below)),
    }))
}

/// The totals of the time of a project: its issues and the issues of its sub-projects that the
/// reader sees, of the fields they may read in all of them.
///
/// # Errors
/// `resource.not-found` for a project that does not exist or that the person may not see;
/// `search.too-slow` when the totals take longer than the limit of a search.
pub async fn project_time<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    query: ProjectTimeQuery,
) -> Result<Authorized<TimeTotals>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    let found = ctx.find_project(&query.project).await?;
    let place = place_of_address(found.as_ref().map(|found| found.project.id))?;
    let held = ctx.evaluation_of(&actor, place).await?;
    let Some(project) = found
        .map(|found| found.project)
        .filter(|_| held.sees_project())
    else {
        return Err(AppError::Refused(ErrorCode::NotFound));
    };
    let totals = ctx
        .totals_found(&actor, "", SearchScope::Within(project.id))
        .await?
        .unwrap_or_default();
    Ok(held.authorize(totals))
}
