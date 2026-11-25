//! Deadline drivers on a page of the timeline (PLAN-05, ADR-0043, amendment of 2026-11-24): for
//! each end of the page (each goal with a *Due date*, each release with a *Release date*, and the
//! last day of the view) the issues whose delay by a day moves that end by a day. The rule is
//! `bb_core::plan::drivers`; this module gives it the issues the reader sees.
//!
//! A chain may reach back past the issues around the page, so the plan of a page with the drivers
//! on also holds the issues its issues wait for, over any number of steps, as far as the reader
//! sees them: the walk stops at an issue they may not see, so a chain through it is not outlined
//! and nothing tells that one exists. Two readers may see different drivers, as they may see
//! different bars.

use std::collections::HashSet;

use bb_core::id::IssueId;
use bb_core::plan::drivers::{End, EndOf};
use bb_core::project_release::Release;

use super::build::{Plan, PlanAsk};
use super::read::goals_of;
use super::types::{DriverView, GoalView, ReleaseMark, WarningView};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::relations::effect_kinds;
use crate::releases::in_order;
use crate::search::SearchStore;

/// How many issues the walk back along dependencies adds to the plan of one page at most: the
/// chains of a page are worked out within this many issues the reader sees.
const CHAIN_ISSUES_MOST: usize = 1_000;

impl<S: AuthStore> Ctx<'_, S> {
    /// The plan of the issues of `ask` with the issues they wait for, over any number of steps,
    /// that the reader sees, and the issues around them all: what the drivers of a page are worked
    /// out over. Each step back reads the dependencies of the issues reached last and judges the
    /// new ones at once.
    pub(crate) async fn plan_with_chains(&self, mut ask: PlanAsk<'_>) -> Result<Plan, AppError> {
        let failed = |error: S::Error| internal::<S>(&error);
        let kinds = effect_kinds(&self.store().relation_kinds().await.map_err(failed)?);
        let mut known: HashSet<IssueId> = ask.issues.iter().map(|issue| issue.id).collect();
        let mut reached: Vec<IssueId> = known.iter().copied().collect();
        let mut added = 0;
        while !reached.is_empty() && added < CHAIN_ISSUES_MOST {
            let last: HashSet<IssueId> = reached.iter().copied().collect();
            let mut before: Vec<IssueId> = self
                .store()
                .dependencies_touching(kinds.clone(), reached)
                .await
                .map_err(failed)?
                .into_iter()
                .filter(|(first, then)| last.contains(then) && !known.contains(first))
                .map(|(first, _)| first)
                .collect();
            before.sort_unstable();
            before.dedup();
            before.truncate(CHAIN_ISSUES_MOST - added);
            known.extend(before.iter().copied());
            let seen = self.seen_issues(ask.actor, &before, &[]).await?;
            added += seen.len();
            reached = seen.iter().map(|(issue, _)| issue.id).collect();
            ask.issues.extend(seen.into_iter().map(|(issue, _)| issue));
        }
        self.plan_with_surroundings(ask).await
    }
}

/// What a page of rows shows besides its rows and arrows: the warnings of its issues, its goals, its
/// releases, and its deadline drivers when the switch is on.
#[derive(Default)]
pub(crate) struct PageMarks {
    pub(crate) warnings: Vec<WarningView>,
    pub(crate) goals: Vec<GoalView>,
    pub(crate) releases: Vec<ReleaseMark>,
    pub(crate) drivers: Vec<DriverView>,
}

impl<S: AuthStore + SearchStore> Ctx<'_, S> {
    /// The marks of a page of rows ([`PageMarks`]), the drivers only with `drivers_on`.
    pub(crate) async fn page_marks(
        &self,
        plan: &Plan,
        page: &HashSet<IssueId>,
        seen: &crate::projects::Leveled,
        drivers_on: bool,
    ) -> Result<PageMarks, AppError> {
        let goals = goals_of(plan, page);
        let releases = release_marks(plan, page);
        let drivers = if drivers_on {
            drivers_on_page(plan, page, &goals, &releases)
        } else {
            Vec::new()
        };
        Ok(PageMarks {
            warnings: self.warning_views(plan, page, seen).await?,
            goals,
            releases,
            drivers,
        })
    }
}

/// The releases of the page: those its issues are planned for, as the reader may know them, in the
/// order of their *Release date* (none last), then by name.
fn release_marks(plan: &Plan, page: &HashSet<IssueId>) -> Vec<ReleaseMark> {
    let mut releases: Vec<Release> = plan
        .issues
        .values()
        .filter(|issue| page.contains(&issue.id))
        .filter_map(|issue| plan.planned_for(issue))
        .cloned()
        .collect();
    in_order(&mut releases);
    releases.dedup_by_key(|release| release.id);
    releases
        .into_iter()
        .map(|release| ReleaseMark {
            id: release.id,
            project: release.project,
            name: release.name,
            release_date: release.release_date,
            version: release.version,
        })
        .collect()
}

/// The ends of a page: each goal with a due date the reader may read over its sub-issues, each
/// release with a *Release date* over the issues planned for it, and the view over the issues of
/// the page.
fn ends_of(
    plan: &Plan,
    page: &HashSet<IssueId>,
    goals: &[GoalView],
    releases: &[ReleaseMark],
) -> Vec<End> {
    let goal_ends = goals
        .iter()
        .filter(|goal| goal.due.is_some())
        .map(|goal| End {
            of: EndOf::Goal(goal.issue),
            members: plan
                .parents
                .iter()
                .filter(|(_, parent)| *parent == goal.issue)
                .map(|(child, _)| *child)
                .collect(),
        });
    let release_ends = releases
        .iter()
        .filter(|release| release.release_date.is_some())
        .map(|release| End {
            of: EndOf::Release(release.id),
            members: plan
                .issues
                .values()
                .filter(|issue| {
                    plan.planned_for(issue)
                        .is_some_and(|planned| planned.id == release.id)
                })
                .map(|issue| issue.id)
                .collect(),
        });
    let mut view: Vec<IssueId> = page.iter().copied().collect();
    view.sort_unstable();
    goal_ends
        .chain(release_ends)
        .chain(std::iter::once(End {
            of: EndOf::View,
            members: view,
        }))
        .collect()
}

/// The deadline drivers among the issues of the page, each with the ends it sets.
fn drivers_on_page(
    plan: &Plan,
    page: &HashSet<IssueId>,
    goals: &[GoalView],
    releases: &[ReleaseMark],
) -> Vec<DriverView> {
    plan.chain
        .drivers_of(&ends_of(plan, page, goals, releases))
        .into_iter()
        .filter(|(issue, _)| page.contains(issue))
        .map(|(issue, ends)| DriverView { issue, ends })
        .collect()
}
