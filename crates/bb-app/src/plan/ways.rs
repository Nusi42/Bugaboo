//! The ways out of a plan warning that the server works out (PLAN-13, ADR-0030, decision 5,
//! ADR-0043, decision 5): the preview of *Move it after*, and *Keep and accept*. *Make it longer*
//! is the due date the warning answers, set by the usual change of the issue when the person
//! confirms it; *Give it to someone else…* is the usual change of *Assigned to*. Nothing moves until
//! a person chooses.

use std::collections::{HashMap, HashSet, VecDeque};

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::id::IssueId;
use bb_core::plan::Days;
use bb_core::plan::order::move_after;
use bb_core::plan::record::AcceptedWarning;
use bb_core::relation::DependencyGraph;
use bb_policy::{Authorized, Place};
use jiff::ToSpan as _;
use jiff::civil::Date;
use serde_json::json;

use super::build::{PlanAsk, window_of};
use super::factor::person_on_bar;
use super::read::{date_targets, date_words};
use super::types::{
    AcceptWarning, AcceptedWarningView, MoveAfterQuery, MovePreview, MovePreviewPage,
};
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::issues::member;
use crate::projects::Why;
use crate::relations::effect_kinds;
use crate::search::{SearchScope, SearchStore};

/// A date moved by `by` days; `None` past the ends of the calendar.
fn moved(date: Option<Date>, by: i64) -> Result<Option<Date>, AppError> {
    date.map(|date| date.checked_add(by.days()))
        .transpose()
        .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))
}

/// The preview of *Move it after*: `issue` starts on the day after `after`, and every issue that
/// waits for a moved issue, over any number of steps, starts on the day after the last of the
/// issues it waits for ends. Only issues the reader may see are walked. Each keeps its number of
/// days; the answer names every issue that moves with its new *Start date* and *Due date* and the
/// version to move it from, and nothing is saved.
///
/// # Errors
/// `resource.not-found` for an issue the person may not see; `request.invalid` for a view whose
/// bars come from own date fields (a move changes *Start date* and *Due date* only), or for an
/// issue without a bar.
pub async fn move_after_preview<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    query: MoveAfterQuery,
) -> Result<Authorized<MovePreviewPage>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    if query.options.dates.is_some() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let (issue, held) = ctx.visible_issue(&actor, &query.issue).await?;
    let failed = |error: S::Error| internal::<S>(&error);
    // One graph of every dependency, built once for the walk (M8-F26: one change reads it once).
    let kinds = effect_kinds(&ctx.store().relation_kinds().await.map_err(failed)?);
    let all = ctx
        .store()
        .dependencies_of_kinds(kinds)
        .await
        .map_err(failed)?;
    let graph = DependencyGraph::new(&all);
    let mut reached: Vec<IssueId> = Vec::new();
    let mut seen: HashSet<IssueId> = HashSet::new();
    let mut queue = VecDeque::from([issue.id]);
    while let Some(at) = queue.pop_front() {
        if seen.insert(at) {
            reached.push(at);
            queue.extend(graph.waiting(at).iter().copied());
        }
    }
    // The issues each reached issue waits for: they keep their days, and the walk starts after them.
    let before: Vec<IssueId> = all
        .iter()
        .filter(|(_, then)| seen.contains(then))
        .map(|(first, _)| *first)
        .filter(|first| !seen.contains(first))
        .collect();
    let mut asked: Vec<IssueId> = reached.into_iter().chain(before).collect();
    asked.sort();
    asked.dedup();
    let visible = ctx.seen_issues(&actor, &asked).await?;
    let looked = ctx
        .looked_at(&actor, SearchScope::Within(issue.project))
        .await?;
    let words = date_words(&query.options);
    let targets = date_targets(&looked, &words);
    let window = window_of(Days::one(query.after))?;
    let plan = ctx
        .work_out_plan(
            PlanAsk {
                issues: visible.into_iter().map(|(found, _)| found).collect(),
                dates: targets,
                by_effort: query.options.plan_by_effort,
                window,
                actor: &actor,
            },
            &all,
            &[],
        )
        .await?;
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
        issues.push(MovePreview {
            issue: found.id,
            key,
            version: found.version,
            from: step.from,
            to: step.to,
            start_date: moved(found.start_date, step.by())?,
            due_date: moved(found.due_date, step.by())?,
        });
    }
    Ok(held.authorize(MovePreviewPage { issues }))
}

/// *Keep and accept*: keeps the warning of this kind on the issue, for the dates the issue has
/// now, so everybody who sees the issue sees it faded until its dates change. The person on the bar
/// and whoever may change the issue may accept it.
///
/// # Errors
/// `resource.not-found` for an issue the person may not see; `access.forbidden` for somebody else;
/// `edit.version-changed` when the issue changed since it was read.
pub async fn accept_warning<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AcceptWarning,
) -> Result<Authorized<AcceptedWarningView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let on_bar = person_on_bar(&ctx.assigned_to(issue.id).await?) == Some(actor.person.id);
    if actor.agent().is_some() || !(on_bar || held.may_change_issue(issue.reported_by)) {
        return Err(ErrorCode::Forbidden.into());
    }
    let record = entity(RecordKind::Issue, issue.id);
    let place = Place::Project(issue.project);
    if issue.version != input.version {
        return Err(AppError::stale(record, place, actor.person.id));
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
        start_date: accepted.start_date,
        due_date: accepted.due_date,
        by: accepted.accepted_by,
    }))
}
