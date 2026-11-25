//! Deadline drivers on a page of the timeline (PLAN-05, ADR-0043, amendment of 2026-11-24): for
//! each end of the page (each goal with a *Due date*, each release with a *Release date*, and the
//! last day of the view) the issues whose delay by a day moves that end by a day. The rule is
//! `bb_core::plan::drivers`; this module gives it the issues the reader sees.
//!
//! A chain may reach past the issues around the page, so the plan of a page with the drivers on also
//! holds the issues connected to its issues by dependencies, as far as the reader sees them: the walk
//! stops at an issue they may not see, so a chain through it is not outlined and nothing tells that
//! one exists. Two readers may see different drivers, as they may see different bars.

use std::collections::HashSet;

use bb_core::id::IssueId;
use bb_core::plan::drivers::{End, EndOf};
use bb_core::project_release::Release;

use bb_searchlang::MAX_STEPS;

use super::build::{Plan, PlanAsk};
use super::read::goals_of;
use super::types::{DriverView, GoalView, ReleaseMark, WarningView};
use crate::auth::{AppError, AuthStore, Ctx};
use crate::releases::in_order;
use crate::search::{MapStep, SearchFailure, SearchScope, SearchStore, stopped};

/// How many issues the walk along dependencies adds to the plan of one page at most: the chains of
/// a page are worked out within this many issues the reader sees.
const CHAIN_ISSUES_MOST: usize = 1_000;

impl<S: AuthStore + SearchStore> Ctx<'_, S> {
    /// The plan of the issues of `ask` with the issues connected to them by dependencies that the
    /// reader sees, and the issues around them all: what the drivers of a page are worked out over.
    /// The walk is the walk of the relation map (`SearchStore::map_step`) along the dependencies:
    /// one statement for each step, through only the issues the reader sees in every project, not
    /// only those of the page's search and project (an issue they may not see is neither walked past
    /// nor counted), stopping in the statement at the room left, at most [`MAX_STEPS`] steps, and all
    /// of it within the time of the page's search, which `began` with it. When that time runs out
    /// the walk keeps what it reached, as at the cap and the limit of steps, and says nothing of it.
    pub(crate) async fn plan_with_chains(
        &self,
        mut ask: PlanAsk<'_>,
        began: std::time::Instant,
    ) -> Result<Plan, AppError> {
        let kinds = self.dependency_kinds().await?;
        let sight = self
            .prepare_search(ask.actor, "", SearchScope::Everywhere)
            .await?;
        let mut known: Vec<IssueId> = ask.issues.iter().map(|issue| issue.id).collect();
        let asked = known.len();
        let mut near = known.clone();
        for _ in 0..MAX_STEPS {
            let room = CHAIN_ISSUES_MOST.saturating_sub(known.len() - asked);
            if near.is_empty() || room == 0 {
                break;
            }
            let step = self
                .store()
                .map_step(
                    &sight.request,
                    &MapStep {
                        near,
                        known: known.clone(),
                        kinds: kinds.clone(),
                        part_of: false,
                        most: u64::try_from(room).unwrap_or(u64::MAX),
                        began,
                    },
                )
                .await;
            near = match step {
                Ok(found) => found,
                Err(SearchFailure::TooSlow) => break,
                Err(other) => return Err(stopped(other)),
            };
            known.extend(near.iter().copied());
        }
        let reached = known.get(asked..).unwrap_or_default();
        ask.issues.extend(
            self.seen_issues(ask.actor, reached, &[])
                .await?
                .into_iter()
                .map(|(issue, _)| issue),
        );
        self.plan_around(ask, kinds).await
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
    /// The dependencies of the drivers' chains between issues of the page.
    pub(crate) driving: Vec<(IssueId, IssueId)>,
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
        let (drivers, driving) = if drivers_on {
            drivers_on_page(plan, page, &goals, &releases)
        } else {
            (Vec::new(), Vec::new())
        };
        Ok(PageMarks {
            warnings: self.warning_views(plan, page, seen).await?,
            goals,
            releases,
            drivers,
            driving,
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

/// The deadline drivers among the issues of the page, each with the ends it sets, and the
/// dependencies of their chains between issues of the page.
fn drivers_on_page(
    plan: &Plan,
    page: &HashSet<IssueId>,
    goals: &[GoalView],
    releases: &[ReleaseMark],
) -> (Vec<DriverView>, Vec<(IssueId, IssueId)>) {
    let drivers = plan.chain.drivers_of(&ends_of(plan, page, goals, releases));
    let links = drivers
        .links
        .into_iter()
        .filter(|(first, then)| page.contains(first) && page.contains(then))
        .collect();
    let views = drivers
        .ends
        .into_iter()
        .filter(|(issue, _)| page.contains(issue))
        .map(|(issue, ends)| DriverView { issue, ends })
        .collect();
    (views, links)
}
