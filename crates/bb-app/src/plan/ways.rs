//! The ways out of a plan warning that the server works out (PLAN-13, ADR-0030, decision 5,
//! ADR-0043, decision 5): the preview of *Move it after*, and *Keep and accept*. *Make it longer*
//! is the due date the warning answers, set by the usual change of the issue when the person
//! confirms it; *Give it to someone else…* is the usual change of *Assigned to*. Nothing moves until
//! a person chooses.
//!
//! Both work on views of *Start date* and *Due date* only: a move changes those two, and an
//! accepted warning comes back when they change, so a view of own date fields can do neither.

use std::collections::{HashMap, HashSet};

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::fields::Field;
use bb_core::id::IssueId;
use bb_core::issue::Issue;
use bb_core::many_changes::MANY_CHANGES_JOB_ABOVE;
use bb_core::plan::Days;
use bb_core::plan::order::move_after;
use bb_core::plan::record::AcceptedWarning;
use bb_core::plan::warning::WarningKind;
use bb_core::relation::DependencyGraph;
use bb_core::saved_search::TimelineOptions;
use bb_policy::{Authorized, Evaluation, Place};
use jiff::ToSpan as _;
use jiff::civil::Date;
use serde_json::json;

use super::build::{Plan, PlanAsk, window_of};
use super::factor::person_on_bar;
use super::read::{date_targets, date_words};
use super::types::{
    AcceptWarning, AcceptedWarningView, MoveAfterQuery, MovePreview, MovePreviewPage,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::issues::member;
use crate::projects::Why;
use crate::relations::effect_kinds;
use crate::search::{SearchScope, SearchStore};

/// The most issues one *Move it after* moves: as many as one run of changes takes before it goes
/// to the background. A walk that reaches more is refused before anything is worked out.
const MOVED_MOST: usize = MANY_CHANGES_JOB_ABOVE;

/// A date moved by `by` days, when the reader may read it; `None` past the ends of the calendar.
fn moved(date: Option<Date>, by: i64) -> Result<Option<Date>, AppError> {
    date.map(|date| date.checked_add(by.days()))
        .transpose()
        .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))
}

/// Refuses a view of own date fields: the ways out change and keep *Start date* and *Due date*.
fn ensure_built_in_dates(options: &TimelineOptions) -> Result<(), AppError> {
    if options.dates.is_some() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    Ok(())
}

impl<S: AuthStore + SearchStore> Ctx<'_, S> {
    /// What a way out is worked out from: the date fields of a view of *Start date* and *Due date*
    /// as the actor may read them where `issue` is, and the days to read work times for.
    async fn ask_for<'a>(
        &self,
        actor: &'a Actor,
        issue: &Issue,
        issues: Vec<Issue>,
        options: &TimelineOptions,
        around: Date,
    ) -> Result<PlanAsk<'a>, AppError> {
        let looked = self
            .looked_at(actor, SearchScope::Within(issue.project))
            .await?;
        Ok(PlanAsk {
            issues,
            dates: date_targets(&looked, &date_words(options)),
            by_effort: options.plan_by_effort,
            window: window_of(Days::one(around))?,
            actor,
        })
    }
}

/// One issue of the preview, with only the dates the reader may read.
fn preview_of(
    plan: &Plan,
    found: &Issue,
    key: String,
    step: &bb_core::plan::order::Moved,
) -> Result<MovePreview, AppError> {
    let by = step.by();
    Ok(MovePreview {
        issue: found.id,
        key,
        version: found.version,
        from: step.from,
        to: step.to,
        start_date: moved(found.start_date, by)?.filter(|_| plan.reads(found, Field::StartDate)),
        due_date: moved(found.due_date, by)?.filter(|_| plan.reads(found, Field::DueDate)),
    })
}

/// The preview of *Move it after*: `issue` starts on the day after `after`, and every issue that
/// waits for a moved issue, over any number of steps, starts on the day after the last of the
/// issues it waits for ends. Only issues the reader may see are walked, and at most as many as one
/// run of changes takes. Each keeps its number of days; the answer names every issue that moves
/// with the new *Start date* and *Due date* the reader may read and the version to move it from,
/// and nothing is saved.
///
/// # Errors
/// `resource.not-found` for an issue the person may not see; `request.invalid` for a view whose
/// bars come from own date fields, for an issue without a bar, or for a walk that reaches more
/// issues than one run of changes takes.
pub async fn move_after_preview<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    query: MoveAfterQuery,
) -> Result<Authorized<MovePreviewPage>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    ensure_built_in_dates(&query.options)?;
    let (issue, held) = ctx.visible_issue(&actor, &query.issue).await?;
    let failed = |error: S::Error| internal::<S>(&error);
    // One graph of every dependency, built once for the walk (M8-F26: one statement for one way
    // out).
    let kinds = effect_kinds(&ctx.store().relation_kinds().await.map_err(failed)?);
    let all = ctx
        .store()
        .dependencies_of_kinds(kinds)
        .await
        .map_err(failed)?;
    let graph = DependencyGraph::new(&all);
    let reached: Vec<IssueId> = graph
        .waiting_from(issue.id, |_| true)
        .take(MOVED_MOST + 1)
        .collect();
    if reached.len() > MOVED_MOST {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let seen: HashSet<IssueId> = reached.iter().copied().collect();
    // The issues each reached issue waits for: they keep their days, and the walk starts after them.
    let mut asked: Vec<IssueId> = all
        .iter()
        .filter(|(first, then)| seen.contains(then) && !seen.contains(first))
        .map(|(first, _)| *first)
        .chain(reached)
        .collect();
    asked.sort();
    asked.dedup();
    let visible = ctx.seen_issues(&actor, &asked, &[]).await?;
    let ask = ctx
        .ask_for(
            &actor,
            &issue,
            visible.into_iter().map(|(found, _)| found).collect(),
            &query.options,
            query.after,
        )
        .await?;
    let plan = ctx.work_out_plan(ask, &all, &[]).await?;
    if !plan.bars.contains_key(&issue.id) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let bars: HashMap<IssueId, Days> = plan.bars.iter().map(|(id, bar)| (*id, bar.days)).collect();
    let mut issues = Vec::new();
    let mut line = ctx.line_maker().await?;
    for step in move_after(&graph, &bars, issue.id, query.after) {
        let Some(found) = plan.issues.get(&step.issue) else {
            continue;
        };
        let key = ctx
            .line_of(&actor, &mut line, found)
            .await?
            .map(|line| line.key)
            .unwrap_or_default();
        issues.push(preview_of(&plan, found, key, &step)?);
    }
    Ok(held.authorize(MovePreviewPage { issues }))
}

/// Whether `actor` may accept a warning of `issue`: the person on the bar, with a credential that
/// may change things at all, or whoever may change the issue; never an agent.
fn ensure_may_accept(
    actor: &Actor,
    held: &Evaluation,
    issue: &Issue,
    on_bar: bool,
) -> Result<(), AppError> {
    let allowed = (on_bar && actor.key_may_change()) || held.may_change_issue(issue.reported_by);
    if actor.agent().is_some() || !allowed {
        return Err(ErrorCode::Forbidden.into());
    }
    Ok(())
}

/// *Keep and accept*: keeps the warning of this kind on the issue, for the dates the issue has
/// now, so everybody who sees the issue sees it faded until its dates change. The person on the bar
/// and whoever may change the issue may accept it, and only a warning the issue has now in the view
/// it was shown in. *Too much at once* depends on the person's other work on the screen, so it is
/// taken as it is said.
///
/// # Errors
/// `resource.not-found` for an issue the person may not see; `access.forbidden` for somebody else;
/// `request.invalid` for a view of own date fields or a warning the issue does not have;
/// `edit.version-changed` when the issue changed since it was read.
pub async fn accept_warning<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    input: AcceptWarning,
) -> Result<Authorized<AcceptedWarningView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ensure_built_in_dates(&input.options)?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let on_bar = person_on_bar(&ctx.assigned_to(issue.id).await?) == Some(actor.person.id);
    ensure_may_accept(&actor, &held, &issue, on_bar)?;
    let record = entity(RecordKind::Issue, issue.id);
    let place = Place::Project(issue.project);
    if issue.version != input.version {
        return Err(AppError::stale(record, place, actor.person.id));
    }
    if input.kind != WarningKind::TooMuchAtOnce {
        let around = issue
            .start_date
            .or(issue.due_date)
            .unwrap_or_else(|| ctx.now().to_zoned(jiff::tz::TimeZone::UTC).date());
        let ask = ctx
            .ask_for(&actor, &issue, vec![issue.clone()], &input.options, around)
            .await?;
        let plan = ctx.plan_with_surroundings(ask).await?;
        let has_it = plan
            .warnings
            .iter()
            .any(|(id, warning)| *id == issue.id && warning.kind() == input.kind);
        if !has_it {
            return Err(ErrorCode::InvalidRequest.into());
        }
    }
    let accepted = AcceptedWarning {
        issue: issue.id,
        kind: input.kind,
        start_date: issue.start_date,
        due_date: issue.due_date,
        accepted_by: actor.person.id,
        accepted_at: ctx.now(),
    };
    let event = ctx.event(
        by(actor.person.id),
        record.clone(),
        vec![set_to(member::ACCEPTED_WARNING, json!(input.kind.code()))],
    )?;
    let (why, version, kept) = (Why::default(), issue.version, accepted.clone());
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // Accepted for the dates the issue has: one changed in between is caught.
                match S::issue_in(transaction, kept.issue).await? {
                    Some(stored) if stored.version == version => {}
                    _ => return Err(stopped.stop::<S>(ErrorCode::VersionChanged)),
                }
                S::put_accepted_warning(transaction, &kept).await
            })
        })
        .await
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                &record,
                place,
                actor.person.id,
            )
        })?;
    Ok(held.authorize(AcceptedWarningView {
        issue: accepted.issue,
        kind: accepted.kind,
    }))
}
