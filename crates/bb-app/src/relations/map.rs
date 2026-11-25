//! The relation map (ISS-16, UX design §4.15, ADR-0009 amendment 2026-11-24): every issue connected
//! to one issue through the kinds of relation chosen, step by step, and the map of a search's
//! results, each with the relations between the issues on it.
//!
//! Three rules, each in one place:
//! - **Only through what the reader sees.** Each step is one statement of the store that joins only
//!   issues of the reader's sight, the one the search uses (`SearchStore::map_step`). An issue
//!   reached only through a hidden one is not on the map, and nothing counts or names the hidden
//!   one: `more` is worked out on the walk the reader sees.
//! - **One limit of steps and one of issues.** *Everything* is [`MAX_STEPS`], the limit of the
//!   search language (ADR-0005, amendment 2026-11-24); a map holds at most [`MAP_ISSUES_MOST`]
//!   issues, the start among them, and says when there are more.
//! - **One order.** The issues come by steps away, then by the kind of the relation that reached
//!   them, then by Issue ID, so the screen draws the answer as it comes and never sorts again.

use std::collections::HashMap;

use bb_core::ErrorCode;
use bb_core::id::{IssueId, RelationKindId};
use bb_core::message::{self, Message};
use bb_core::readable_id::ReadableId;
use bb_core::relation::{Dependency, RelationKind, dependency_loops};
use bb_policy::{Authorized, Masked, Place as Where};
use bb_searchlang::MAX_STEPS;
use serde::Serialize;

use super::dependencies::effect_kinds;
use super::types::IssueLine;
use crate::access::Actor;
use crate::ai::Withheld;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::search::{MapStep, Prepared, SearchScope, SearchStore, blocking_kinds, stopped};

/// The most issues one relation map holds, its start among them (UX design §4.15, M10-Q11).
pub const MAP_ISSUES_MOST: usize = 200;

/// The kinds of line a map walks and draws (*Kinds shown*): kinds of relation, and *part of*.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindsShown {
    /// The kinds of relation; `None` for every kind the organization has.
    pub kinds: Option<Vec<RelationKindId>>,
    /// Whether sub-issues and their parents are shown.
    pub part_of: bool,
}

impl Default for KindsShown {
    /// Every kind, and *part of*.
    fn default() -> Self {
        Self {
            kinds: None,
            part_of: true,
        }
    }
}

/// The map of one issue.
#[derive(Clone, Debug)]
pub struct IssueMapQuery {
    /// The session or access key of the person.
    pub session_token: String,
    /// The issue the map starts from, by its readable or internal ID.
    pub issue: String,
    /// How many steps away the map walks: 1 to [`MAX_STEPS`]; `None` for *Everything*.
    pub steps: Option<u32>,
    /// What the map walks through.
    pub shown: KindsShown,
}

/// The map of a search: its results and the relations between them, no walk beyond them.
#[derive(Clone, Debug)]
pub struct SearchMapQuery {
    /// The session or access key of the person.
    pub session_token: String,
    /// The text of the search.
    pub text: String,
    /// Which relations between the results are drawn.
    pub shown: KindsShown,
}

/// An issue on the map.
#[derive(Clone, Debug, Serialize)]
pub struct MapIssue {
    /// How many steps away from the start it is; none on the map of a search.
    pub steps: Option<u32>,
    /// The issue as a line, with the fields the reader may see in its project.
    pub issue: IssueLine,
}

/// A line between two issues on the map: a relation, or a sub-issue under its parent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MapLine {
    /// The issue that reads `forward`: the one that blocks, or the sub-issue.
    pub from: IssueId,
    /// The issue that reads `backward`: the one that waits, or the parent.
    pub to: IssueId,
    /// The kind of relation; none for *part of*.
    pub kind: Option<RelationKindId>,
    /// What `from` reads (*blocks*, *is part of*).
    pub forward: String,
    /// What `to` reads (*is blocked by*, *has sub-issue*).
    pub backward: String,
}

/// The relation map as the reader may see it.
#[derive(Clone, Debug, Serialize)]
pub struct RelationMap {
    /// The issue the map starts from; none for the map of a search.
    pub start: Option<IssueId>,
    /// The issues, in the order of the map.
    pub issues: Vec<MapIssue>,
    /// The lines between them, of the kinds shown.
    pub lines: Vec<MapLine>,
    /// The loops of dependencies among them (*blocks* and the kinds with *Must finish first*),
    /// each as the issues on it in order.
    pub loops: Vec<Vec<IssueId>>,
    /// Whether more issues are connected (or found) than the map holds.
    pub more: bool,
}

// Each issue's fields were left out by its own project (`LineMaker`); the rest is IDs and the
// names of kinds of relation, which every signed-in person reads.
impl Masked for RelationMap {}

/// The issues reached and how many steps away each is, and whether there were more.
struct Walked {
    reached: Vec<(IssueId, Option<u32>)>,
    more: bool,
}

/// The kinds of relation of the organization a map draws, in their order.
fn kinds_drawn(all: Vec<RelationKind>, shown: &KindsShown) -> Vec<RelationKind> {
    match &shown.kinds {
        None => all,
        Some(chosen) => all
            .into_iter()
            .filter(|kind| chosen.contains(&kind.id))
            .collect(),
    }
}

/// The steps a map walks: those asked for, or *Everything*. More than the search language walks
/// is no request.
fn steps_of(asked: Option<u32>) -> Result<u32, AppError> {
    match asked {
        None => Ok(MAX_STEPS),
        Some(steps) if (1..=MAX_STEPS).contains(&steps) => Ok(steps),
        Some(_) => Err(AppError::Refused(ErrorCode::InvalidRequest)),
    }
}

impl<S: AuthStore + SearchStore> Ctx<'_, S> {
    /// The walk from `start`, one statement for each step away, each starting from the issues of
    /// the step before and stopping at the room left on the map and one more, which says `more`.
    /// The whole walk has the time of one search: each step gets what is left of it.
    async fn walk(
        &self,
        prepared: &Prepared,
        start: IssueId,
        steps: u32,
        (kinds, part_of): (&[RelationKind], bool),
    ) -> Result<Walked, AppError> {
        let mut reached = vec![(start, Some(0))];
        let mut near = vec![start];
        let mut more = false;
        let began = std::time::Instant::now();
        for away in 1..=steps {
            let room = MAP_ISSUES_MOST.saturating_sub(reached.len());
            let mut found = self
                .store()
                .map_step(
                    &prepared.request,
                    &MapStep {
                        near: near.clone(),
                        known: reached.iter().map(|(issue, _)| *issue).collect(),
                        kinds: kinds.iter().map(|kind| kind.id).collect(),
                        part_of,
                        most: u64::try_from(room + 1).unwrap_or(u64::MAX),
                        began,
                    },
                )
                .await
                .map_err(stopped)?;
            if found.len() > room {
                more = true;
                found.truncate(room);
            }
            reached.extend(found.iter().map(|issue| (*issue, Some(away))));
            near = found;
            if more || near.is_empty() {
                break;
            }
        }
        Ok(Walked { reached, more })
    }

    /// The map of `walked` for the reader: each issue as a line, the lines between them, their
    /// loops, and the order of the map. A fixed number of statements, whatever the size (PERF-N1).
    async fn drawn(
        &self,
        actor: &Actor,
        walked: Walked,
        start: Option<IssueId>,
        (kinds, part_of): (&[RelationKind], bool),
    ) -> Result<RelationMap, AppError> {
        let store = self.store();
        let failed = |error| internal::<S>(&error);
        let ids: Vec<IssueId> = walked.reached.iter().map(|(issue, _)| *issue).collect();
        let found = self.issues_named(&ids).await?;
        let mut maker = self.line_maker().await?;
        let mut issues = Vec::with_capacity(ids.len());
        for (id, steps) in &walked.reached {
            let Some(issue) = found.iter().find(|issue| issue.id == *id) else {
                continue;
            };
            if let Some(line) = self.line_of(actor, &mut maker, issue).await? {
                issues.push(MapIssue {
                    steps: *steps,
                    issue: line,
                });
            }
        }
        let shown: Vec<IssueId> = issues.iter().map(|each| each.issue.id).collect();
        let mut lines: Vec<MapLine> = store
            .relations_among(kinds.iter().map(|kind| kind.id).collect(), shown.clone())
            .await
            .map_err(failed)?
            .into_iter()
            .filter_map(|relation| {
                let kind = kinds.iter().find(|kind| kind.id == relation.kind)?;
                Some(MapLine {
                    from: relation.from,
                    to: relation.to,
                    kind: Some(kind.id),
                    forward: kind.forward.clone(),
                    backward: kind.backward.clone(),
                })
            })
            .collect();
        if part_of {
            let mut parents = store
                .parents_touching(shown.clone())
                .await
                .map_err(failed)?;
            parents.sort_by_key(|row| (row.parent, row.child));
            lines.extend(
                parents
                    .into_iter()
                    .filter(|row| shown.contains(&row.child) && shown.contains(&row.parent))
                    .map(|row| MapLine {
                        from: row.child,
                        to: row.parent,
                        kind: None,
                        forward: Message::from(message::RELATION_MAP_PART_OF).render(),
                        backward: Message::from(message::RELATION_MAP_HAS_SUB_ISSUE).render(),
                    }),
            );
        }
        // The kinds with *Must finish first*, and *blocks*, whose loops stored before the effect
        // refused them stay (ADR-0043).
        let mut effects = effect_kinds(kinds);
        effects.extend(blocking_kinds(kinds));
        let dependencies: Vec<Dependency> = lines
            .iter()
            .filter(|line| line.kind.is_some_and(|kind| effects.contains(&kind)))
            .map(|line| (line.from, line.to))
            .collect();
        let loops = dependency_loops(&dependencies);
        if start.is_some() {
            order_by_reach(&mut issues, &lines, kinds);
        }
        Ok(RelationMap {
            start,
            issues,
            lines,
            loops,
            more: walked.more,
        })
    }

    /// The kinds of relation a map draws.
    async fn map_kinds(&self, shown: &KindsShown) -> Result<Vec<RelationKind>, AppError> {
        let all = self
            .store()
            .relation_kinds()
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(kinds_drawn(all, shown))
    }

    /// The map of one issue for the actor, with what is kept from AI left out of the walk for an
    /// AI app or an agent (`withheld`).
    pub(crate) async fn issue_map_for(
        &self,
        actor: &Actor,
        query: &IssueMapQuery,
        withheld: Option<&Withheld>,
    ) -> Result<Authorized<RelationMap>, AppError> {
        let steps = steps_of(query.steps)?;
        let (issue, _) = self.visible_issue(actor, &query.issue).await?;
        if withheld.is_some_and(|kept| kept.hides_project(issue.project)) {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        // The sight of a search that finds everything the reader reads, as the AI reads it when
        // `withheld` is given.
        let prepared = self
            .prepare_search_as(actor, "", SearchScope::Everywhere, withheld)
            .await?;
        let kinds = self.map_kinds(&query.shown).await?;
        let shown = (kinds.as_slice(), query.shown.part_of);
        let walked = self.walk(&prepared, issue.id, steps, shown).await?;
        let map = self.drawn(actor, walked, Some(issue.id), shown).await?;
        let organization = self.evaluation_of(actor, Where::Organization).await?;
        Ok(organization.authorize(map))
    }

    /// The map of a search for the actor: its results, by its order, and the lines between them.
    pub(crate) async fn search_map_for(
        &self,
        actor: &Actor,
        query: &SearchMapQuery,
        withheld: Option<&Withheld>,
    ) -> Result<Authorized<RelationMap>, AppError> {
        let mut prepared = self
            .prepare_search_as(actor, &query.text, SearchScope::Everywhere, withheld)
            .await?;
        prepared.request.limit = u64::try_from(MAP_ISSUES_MOST + 1).unwrap_or(u64::MAX);
        let mut hits = self
            .store()
            .search_page(&prepared.request)
            .await
            .map_err(stopped)?;
        let more = hits.len() > MAP_ISSUES_MOST;
        hits.truncate(MAP_ISSUES_MOST);
        let walked = Walked {
            reached: hits.iter().map(|hit| (hit.issue.id, None)).collect(),
            more,
        };
        let kinds = self.map_kinds(&query.shown).await?;
        let map = self
            .drawn(actor, walked, None, (kinds.as_slice(), query.shown.part_of))
            .await?;
        let organization = self.evaluation_of(actor, Where::Organization).await?;
        Ok(organization.authorize(map))
    }
}

/// The order of a map: by steps away, then by the kind of the relation that reached the issue from
/// the step before (in the order of the kinds, *part of* last), then by Issue ID (UX design §4.15).
fn order_by_reach(issues: &mut [MapIssue], lines: &[MapLine], kinds: &[RelationKind]) {
    let steps: HashMap<IssueId, Option<u32>> = issues
        .iter()
        .map(|each| (each.issue.id, each.steps))
        .collect();
    let rank = |line: &MapLine| {
        line.kind.map_or(kinds.len(), |kind| {
            kinds
                .iter()
                .position(|each| each.id == kind)
                .unwrap_or(kinds.len())
        })
    };
    let reach = |issue: &MapIssue| -> usize {
        let before = issue.steps.and_then(|steps| steps.checked_sub(1));
        lines
            .iter()
            .filter(|line| {
                let other = if line.from == issue.issue.id {
                    Some(line.to)
                } else if line.to == issue.issue.id {
                    Some(line.from)
                } else {
                    None
                };
                other.is_some_and(|other| steps.get(&other).copied().flatten() == before)
            })
            .map(rank)
            .min()
            .unwrap_or(0)
    };
    issues.sort_by_cached_key(|issue| {
        (
            issue.steps,
            reach(issue),
            issue.issue.key.parse::<ReadableId>().ok(),
        )
    });
}

/// The map of an issue: every issue connected to it through the kinds shown, up to the steps chosen
/// and at most [`MAP_ISSUES_MOST`] issues, walked only through issues the person sees.
///
/// # Errors
/// `resource.not-found` for an issue that does not exist, is in the trash or that the person may
/// not see; `request.invalid` for more steps than the search language walks; `search.too-slow`
/// for a walk that takes longer than a search may.
pub async fn issue_map<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    query: IssueMapQuery,
) -> Result<Authorized<RelationMap>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    ctx.issue_map_for(&actor, &query, None).await
}

/// The map of a search: its results (at most [`MAP_ISSUES_MOST`], by its order) and the relations
/// between them.
///
/// # Errors
/// The mistakes of the text; `search.too-slow` for a search that takes longer than the limit.
pub async fn search_map<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    query: SearchMapQuery,
) -> Result<Authorized<RelationMap>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    ctx.search_map_for(&actor, &query, None).await
}
