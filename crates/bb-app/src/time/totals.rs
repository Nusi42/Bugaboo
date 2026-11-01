//! The time of an issue and of a project (TIME-03, PRIV-13, ADR-0035, decision 4): an issue's own
//! *Estimated time*, *Time left* and *Time spent*, and the totals with its sub-issues at every level
//! below it; a project's totals of its issues and its sub-projects.
//!
//! A total counts only what the reader may see, and only the fields they may read: it is the
//! compiled search of the list (`under WEB-142` over every project the reader sees for an issue,
//! the list of the project for a project), with the reader's sight and readable fields in it, so a
//! hidden issue and a hidden field change nothing in it (ADR-0004). A field the reader may not read
//! where the issues are is no total at all, never a total of no values. The sums of the estimate
//! and the time left are one statement of that search; the time spent is read through the port of
//! time spent, which reuses the same search. No total walks the tree in Rust.

use bb_core::ErrorCode;
use bb_core::fields::Field;
use bb_core::id::{IssueId, ProjectId};
use bb_core::readable_id::ReadableId;
use bb_core::time::{Minutes, TimeLeft, time_left};
use bb_policy::{Authorized, FieldMask, Masked};
use bb_searchlang::IssueField;
use serde::Serialize;

use crate::access::{Actor, place_of_address};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::search::{
    FieldTarget, GroupKey, SearchRequest, SearchScope, Searchable, lengths_readable_in, stopped,
    target_of,
};

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

/// The total of one length over the issues a search finds: a field the reader may not read where
/// the issues are is no total at all, which is not the same as a total of no values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Summed {
    /// The reader may not read the field in every place the search looks at.
    Unreadable,
    /// The sum; none when no issue found has a value.
    Sum(Option<Minutes>),
}

impl Summed {
    /// A sum of the request, or no total for a field that is not among its readable fields.
    fn of(request: &SearchRequest, field: Field, sum: Option<Minutes>) -> Self {
        if readable(request, field) {
            Self::Sum(sum)
        } else {
            Self::Unreadable
        }
    }

    /// The total as it is shown: none for a field that is not readable.
    const fn shown(self) -> Option<Minutes> {
        match self {
            Self::Unreadable => None,
            Self::Sum(sum) => sum,
        }
    }

    /// The total with the issue's own value: none when the field is not readable below, so the
    /// issue's own value never stands for a total it is not.
    fn with_own(self, own: Option<Minutes>) -> Option<Minutes> {
        match self {
            Self::Unreadable => None,
            Self::Sum(sum) => plus(own, sum),
        }
    }
}

/// The totals of the three lengths over the issues a search finds.
#[derive(Clone, Copy, Debug)]
struct Found {
    estimated: Summed,
    left: Summed,
    spent: Summed,
}

impl Found {
    /// The totals as a project shows them.
    const fn shown(self) -> TimeTotals {
        TimeTotals {
            estimated_time: self.estimated.shown(),
            time_left: self.left.shown(),
            time_spent: self.spent.shown(),
        }
    }

    /// The totals with an issue's own numbers.
    fn with_own(self, own: &TimeTotals) -> TimeTotals {
        TimeTotals {
            estimated_time: self.estimated.with_own(own.estimated_time),
            time_left: self.left.with_own(own.time_left),
            time_spent: self.spent.with_own(own.time_spent),
        }
    }
}

/// Whether the field is among the readable fields of the request: a total is made only of those.
fn readable(request: &SearchRequest, field: Field) -> bool {
    target_of(&request.fields, &field.search_word()).is_some()
}

/// Whether a field of a search is one of the three lengths of an issue, which the totals with the
/// sub-issues judge by the places the sub-issues are in.
const fn is_length(searchable: &Searchable) -> bool {
    matches!(
        searchable.target,
        FieldTarget::EstimatedTime | FieldTarget::TimeLeft | FieldTarget::TimeSpent
    )
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The totals of the issues a search finds, for the reader the search was prepared for; none
    /// when it finds none.
    async fn totals_of(&self, request: &SearchRequest) -> Result<Option<Found>, AppError> {
        let lengths = self
            .store()
            .search_lengths(request)
            .await
            .map_err(stopped)?;
        if !lengths.found_any {
            return Ok(None);
        }
        let spent = if readable(request, Field::TimeSpent) {
            Summed::Sum(Some(
                self.store()
                    .spent_on_search(request)
                    .await
                    .map_err(stopped)?,
            ))
        } else {
            Summed::Unreadable
        };
        Ok(Some(Found {
            estimated: Summed::of(request, Field::EstimatedTime, lengths.estimated),
            left: Summed::of(request, Field::TimeLeft, lengths.time_left),
            spent,
        }))
    }

    /// The totals of the sub-issues of an issue at every level below it that the reader sees, in
    /// whatever project each is: the search of `under` over every project they see, with the lengths
    /// readable in the projects the sub-issues are in (the groups of the search by project).
    async fn totals_below(
        &self,
        actor: &Actor,
        key: &ReadableId,
    ) -> Result<Option<Found>, AppError> {
        let mut prepared = self
            .prepare_search(actor, &format!("under {key}"), SearchScope::Everywhere)
            .await?;
        let projects: Vec<ProjectId> = self
            .store()
            .search_groups(&prepared.request, IssueField::Project.word())
            .await
            .map_err(stopped)?
            .into_iter()
            .filter_map(|group| match group.key {
                GroupKey::Project(project) => Some(project),
                _ => None,
            })
            .collect();
        if projects.is_empty() {
            return Ok(None);
        }
        let lengths = lengths_readable_in(&prepared.looked.seen, &projects);
        prepared.request.fields.retain(|each| !is_length(each));
        prepared.request.fields.extend(lengths);
        self.totals_of(&prepared.request).await
    }

    /// The minutes booked on one issue, through the port of time spent.
    async fn spent_on_one(&self, issue: IssueId) -> Result<Minutes, AppError> {
        Ok(self
            .store()
            .spent_on_issues(&[issue])
            .await
            .map_err(|error| internal::<S>(&error))?
            .first()
            .map(|(_, minutes)| *minutes)
            .unwrap_or_default())
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
/// with them. The issue page reads the minutes spent on the issue here and nowhere else.
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
    let spent = ctx.spent_on_one(issue.id).await?;
    let left = time_left(issue.estimated_time, spent, issue.time_left);
    let own = own_numbers(&mask, issue.estimated_time, left, spent);
    let project = ctx
        .store()
        .project(issue.project)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let key = ReadableId::new(project.short_name, issue.code.clone());
    // The sub-issues the reader sees, through issues they read, wherever they are: the search of
    // `under`, so the walk is the compiler's.
    let below = ctx.totals_below(&actor, &key).await?;
    Ok(held.authorize(IssueTime {
        estimated_time: own.estimated_time,
        time_left: left.filter(|_| mask.can_see(Field::TimeLeft)),
        time_spent: own.time_spent,
        with_sub_issues: below.map(|below| below.with_own(&own)),
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
    let prepared = ctx
        .prepare_search(&actor, "", SearchScope::Within(project.id))
        .await?;
    let totals = ctx
        .totals_of(&prepared.request)
        .await?
        .map(Found::shown)
        .unwrap_or_default();
    Ok(held.authorize(totals))
}
