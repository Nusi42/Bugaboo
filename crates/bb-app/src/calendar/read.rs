//! The calendar read (PLAN-09, PRJ-06, M10-Q5): the issues whose chosen date field lies in a window
//! of days, each on its day, and the due dates of their goals as marks.
//!
//! The window is the search with `<date field> >= first and <date field> <= last` added, run with
//! the reader's rights, so an issue whose date the reader may not read is on no day; the issues are
//! sorted by that date first, then by the sort of the search. A goal is shown only when the reader
//! sees it and may read its *Due date*, as on the timeline.

use std::collections::HashSet;

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::fields::Field;
use bb_core::id::{IssueId, ProjectId};
use bb_core::issue::Issue;
use bb_core::plan::Days;
use bb_core::readable_id::ReadableId;
use bb_policy::{Authorized, Evaluation, Place as Where};
use bb_searchlang::{Comparison, Direction, Expr, FieldRef, SortKey, Span, Test, check};

use super::types::{
    CALENDAR_DAYS_MAX, CALENDAR_ISSUES_MAX, CalendarEntry, CalendarMark, CalendarPage,
    CalendarQuery,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::issues::{ReadOnRequest, WithAgent};
use crate::plan::{DateTargets, date_from, target_key};
use crate::search::{
    FieldTarget, Prepared, SearchStore, day_operand, narrow_by, readable_of, stopped,
};

/// `<word> <comparison> <day>`; none for a day the language cannot write.
fn compared(word: &str, operator: Comparison, day: bb_core::issue::CalendarDay) -> Option<Expr> {
    let span = Span::default();
    Some(Expr::Field {
        field: FieldRef {
            word: word.to_owned(),
            span,
        },
        test: Test::Compare {
            operator,
            value: day_operand(day)?,
            span,
        },
        span,
    })
}

/// The search narrowed to the window and sorted by the date field first.
fn window_search(prepared: &mut Prepared, word: &str, days: Days) -> Result<(), AppError> {
    let span = Span::default();
    let mut search = prepared.request.search.clone().into_search();
    let window = Expr::And {
        items: [
            compared(word, Comparison::GreaterOrEqual, days.first),
            compared(word, Comparison::LessOrEqual, days.last),
        ]
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?,
        span,
    };
    narrow_by(&mut search, window);
    search.sort.insert(
        0,
        SortKey {
            field: FieldRef {
                word: word.to_owned(),
                span,
            },
            direction: Some(Direction::OldestFirst),
            span,
        },
    );
    // The date field is sorted by once, first: a later key on it would sort nothing.
    let date_key = bb_core::fields::word_key(word);
    let mut at = 0;
    search.sort.retain(|key| {
        at += 1;
        at == 1 || bb_core::fields::word_key(&key.field.word) != date_key
    });
    prepared.request.search =
        check(search, &readable_of(&prepared.looked.fields)).map_err(AppError::Mistakes)?;
    Ok(())
}

impl<S: AuthStore + SearchStore> Ctx<'_, S> {
    /// The issues of the window, at most [`CALENDAR_ISSUES_MAX`], and whether there are more: one
    /// statement of the search that reads one issue more than the bound.
    async fn window_issues(&self, prepared: &mut Prepared) -> Result<(Vec<Issue>, bool), AppError> {
        let bound = u64::try_from(CALENDAR_ISSUES_MAX).unwrap_or(u64::MAX);
        prepared.request.limit = bound.saturating_add(1);
        prepared.request.after = None;
        let mut issues: Vec<Issue> = self
            .store()
            .search_page(&prepared.request)
            .await
            .map_err(stopped)?
            .into_iter()
            .map(|hit| hit.issue)
            .collect();
        let more = issues.len() > CALENDAR_ISSUES_MAX;
        issues.truncate(CALENDAR_ISSUES_MAX);
        Ok((issues, more))
    }

    /// The goals of the issues of the window (their goals, and those of them that are goals) that
    /// the reader sees, whose *Due date* they may read and that lies in the window.
    async fn goal_marks(
        &self,
        actor: &Actor,
        prepared: &Prepared,
        shown: &[(Issue, std::sync::Arc<Evaluation>)],
        days: Days,
    ) -> Result<Vec<CalendarMark>, AppError> {
        let ids: Vec<IssueId> = shown.iter().map(|(issue, _)| issue.id).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let (_, parents) = self.around(&ids).await?;
        let page: HashSet<IssueId> = ids.iter().copied().collect();
        let goals: HashSet<IssueId> = parents
            .iter()
            .filter(|(child, parent)| page.contains(child) || page.contains(parent))
            .map(|(_, parent)| *parent)
            .collect();
        let others: Vec<IssueId> = goals
            .iter()
            .copied()
            .filter(|goal| !page.contains(goal))
            .collect();
        let projects: Vec<ProjectId> = shown.iter().map(|(issue, _)| issue.project).collect();
        let mut found: Vec<(Issue, std::sync::Arc<Evaluation>)> = shown
            .iter()
            .filter(|(issue, _)| goals.contains(&issue.id))
            .cloned()
            .collect();
        if !others.is_empty() {
            found.extend(self.seen_issues(actor, &others, &projects).await?);
        }
        let mut marks: Vec<CalendarMark> = found
            .into_iter()
            .filter_map(|(goal, held)| {
                let mask = held.field_mask_for(Some(goal.issue_type));
                let day = goal
                    .due_date
                    .filter(|day| mask.can_see(Field::DueDate) && days.contains(*day))?;
                let project = prepared.looked.seen.stored(goal.project)?;
                Some(CalendarMark {
                    issue: goal.id,
                    readable_id: ReadableId::new(project.short_name.clone(), goal.code.clone())
                        .to_string(),
                    title: mask.can_see(Field::Title).then(|| goal.title.clone()),
                    day,
                })
            })
            .collect();
        marks.sort_by(|one, other| one.day.cmp(&other.day).then(one.issue.cmp(&other.issue)));
        Ok(marks)
    }
}

/// The issues of a window of days by the chosen date field, each on its day, with the due dates of
/// their goals as marks.
///
/// # Errors
/// The mistakes of the text; `resource.not-found` for a saved search or a project the person may
/// not open; `request.invalid` for a window that is backwards or longer than six weeks, or a date
/// field that is none the reader may read; `search.too-slow` for a search that takes longer than
/// the limit.
pub async fn calendar<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    query: CalendarQuery,
) -> Result<Authorized<CalendarPage>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    let invalid = || AppError::Refused(ErrorCode::InvalidRequest);
    let days = Days::new(query.first, query.last).ok_or_else(invalid)?;
    if Days::between(days.first, days.last) >= CALENDAR_DAYS_MAX {
        return Err(invalid());
    }
    let view = ctx
        .prepare_view(
            &actor,
            &query.search,
            (query.project.as_deref(), query.only_this_project),
        )
        .await?;
    let mut prepared = view.prepared;
    let word = query
        .options
        .dates
        .clone()
        .unwrap_or_else(|| Field::DueDate.search_word());
    let target: FieldTarget =
        DateTargets::of_word(&prepared.looked.fields, &word).ok_or_else(invalid)?;
    window_search(&mut prepared, &word, days)?;
    let (issues, more) = ctx.window_issues(&mut prepared).await?;
    let items = ctx
        .page_items(
            &actor,
            &prepared.looked.seen,
            &issues,
            ReadOnRequest {
                people: true,
                labels: false,
                agent: WithAgent::No,
                time: false,
            },
        )
        .await?;
    let ids: Vec<bb_core::id::EntityId> = issues.iter().map(|issue| issue.id.into()).collect();
    let values = match target {
        FieldTarget::Own { .. } => ctx.load_values(RecordKind::Issue, &ids).await?,
        _ => crate::fields::ItemValues::default(),
    };
    let key = target_key(target);
    let mut entries = Vec::with_capacity(issues.len());
    let mut shown = Vec::with_capacity(issues.len());
    for (issue, (held, view)) in issues.into_iter().zip(items) {
        let Some(day) = date_from(&issue, Some(target), &values) else {
            continue;
        };
        let changeable = ctx.changeable_fields(&held, &issue).await?;
        entries.push(CalendarEntry {
            day,
            may_move: key.is_some_and(|key| changeable.contains(&key)),
            issue: held.authorize(view),
        });
        shown.push((issue, held));
    }
    let marks = ctx.goal_marks(&actor, &prepared, &shown, days).await?;
    let organization = ctx.evaluation_of(&actor, Where::Organization).await?;
    Ok(organization.authorize(CalendarPage {
        search_text: view.text,
        entries,
        more,
        marks,
    }))
}
