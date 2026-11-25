//! The one schedule along dependencies (ADR-0043, decision 5 and amendment of 2026-11-24): the days
//! each bar gets when it starts no earlier than the day after the issues it waits for end, keeping
//! its number of days. Two rules stand on it: *Move it after* (the issue that waits, and the issues
//! that wait for it in turn, each on the first day after the one before it ends; a preview of these
//! moves is answered before anything is saved, and the move itself is one run of the dates,
//! `POST /api/v1/issues/move-dates`), and the deadline drivers with *Cannot fit*
//! ([`super::drivers`]).
//!
//! The walk is the one step of every walk along dependencies,
//! [`DependencyGraph::waiting`](crate::relation::DependencyGraph::waiting). **The loop rule:** the
//! issues on a loop of dependencies (stored before loops were refused) stay where they are, and the
//! dependencies of the loop count for nothing; an issue that waits for an issue on a loop waits for
//! it where it stands.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::hash::BuildHasher;

use jiff::civil::Date;
use serde::Serialize;

use super::Days;
use crate::id::IssueId;
use crate::relation::{Dependency, DependencyGraph, dependency_loops};

/// The days of a set of bars along the dependencies between them.
pub struct Schedule {
    /// The issues in the order they were placed: each after every issue it waits for.
    order: Vec<IssueId>,
    /// The days each bar gets.
    days: HashMap<IssueId, Days>,
    /// The issues on a loop.
    looped: HashSet<IssueId>,
    /// The issues each issue waits for among the bars, the dependencies of loops left out.
    before: HashMap<IssueId, Vec<IssueId>>,
}

impl Schedule {
    /// The days the bar of `issue` gets; none without a bar.
    #[must_use]
    pub fn days(&self, issue: IssueId) -> Option<Days> {
        self.days.get(&issue).copied()
    }

    /// The issues that `issue` waits for and that count: those with a bar, off any loop with it.
    #[must_use]
    pub fn before(&self, issue: IssueId) -> &[IssueId] {
        self.before.get(&issue).map_or(&[], Vec::as_slice)
    }

    /// Whether the issue is on a loop of dependencies.
    #[must_use]
    pub fn on_loop(&self, issue: IssueId) -> bool {
        self.looped.contains(&issue)
    }
}

/// Where a schedule starts besides the dependencies: `issue` starts no earlier than the day after
/// `after` (the last day of the issue it waits for, of the away days, or of the person's other
/// work).
#[derive(Clone, Copy, Debug)]
pub struct Seed {
    /// The issue.
    pub issue: IssueId,
    /// The day it starts after.
    pub after: Date,
}

impl DependencyGraph {
    /// The schedule of these bars: each issue that `movable` lets move starts no earlier than the
    /// day after the last of the issues it waits for ends (and than the day after the seed's
    /// `after`), keeping its number of days, and never earlier than it does now. Every other issue
    /// keeps its days, and so does every issue on a loop (the loop rule of this module). Only
    /// dependencies between bars count.
    #[must_use]
    pub fn schedule<H: BuildHasher>(
        &self,
        bars: &HashMap<IssueId, Days, H>,
        seed: Option<Seed>,
        movable: impl Fn(IssueId) -> bool,
    ) -> Schedule {
        let loops = self.loops_among(bars);
        let looped: HashSet<IssueId> = loops
            .iter()
            .flat_map(|(first, then)| [*first, *then])
            .collect();
        let mut before: HashMap<IssueId, Vec<IssueId>> = HashMap::new();
        let mut open: BTreeMap<IssueId, usize> = bars.keys().map(|id| (*id, 0)).collect();
        for first in bars.keys().copied() {
            for then in self.waiting(first) {
                if bars.contains_key(then) && !loops.contains(&(first, *then)) {
                    before.entry(*then).or_default().push(first);
                    if let Some(count) = open.get_mut(then) {
                        *count += 1;
                    }
                }
            }
        }
        // Kahn's order, the smallest ID first, so the same plan gives the same schedule.
        let mut ready: VecDeque<IssueId> = open
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut order = Vec::with_capacity(bars.len());
        let mut days: HashMap<IssueId, Days> = HashMap::with_capacity(bars.len());
        while let Some(at) = ready.pop_front() {
            let Some(now) = bars.get(&at).copied() else {
                continue;
            };
            let earliest = before
                .get(&at)
                .into_iter()
                .flatten()
                .filter_map(|first| days.get(first))
                .map(|first| first.last)
                .chain(seed.filter(|seed| seed.issue == at).map(|seed| seed.after))
                .max()
                .and_then(|last| last.tomorrow().ok());
            let placed = match earliest {
                Some(earliest) if earliest > now.first && movable(at) && !looped.contains(&at) => {
                    now.shifted(Days::between(now.first, earliest))
                        .unwrap_or(now)
                }
                _ => now,
            };
            days.insert(at, placed);
            order.push(at);
            for next in self.waiting(at) {
                if before.get(next).is_some_and(|list| list.contains(&at))
                    && let Some(count) = open.get_mut(next)
                {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        ready.push_back(*next);
                    }
                }
            }
        }
        Schedule {
            order,
            days,
            looped,
            before,
        }
    }

    /// The dependencies between the bars that lie on a loop, each named once
    /// ([`dependency_loops`]).
    fn loops_among<H: BuildHasher>(&self, bars: &HashMap<IssueId, Days, H>) -> HashSet<Dependency> {
        let between: Vec<Dependency> = bars
            .keys()
            .flat_map(|first| {
                self.waiting(*first)
                    .iter()
                    .filter(|then| bars.contains_key(then))
                    .map(|then| (*first, *then))
            })
            .collect();
        dependency_loops(&between)
            .iter()
            .flat_map(|on_it| {
                let next = on_it.iter().skip(1).chain(on_it.first());
                on_it.iter().copied().zip(next.copied()).collect::<Vec<_>>()
            })
            .collect()
    }
}

/// One issue of *Move it after*: the days its bar has, and the days it gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Moved {
    /// The issue.
    pub issue: IssueId,
    /// The days of its bar now.
    pub from: Days,
    /// The days of its bar after the move: as many days, later.
    pub to: Days,
}

impl Moved {
    /// How many days later the bar starts.
    #[must_use]
    pub fn by(&self) -> i64 {
        Days::between(self.from.first, self.to.first)
    }
}

/// The moves of *Move it after* for `issue`: the schedule seeded with it starting after `after`,
/// where only the issues that wait for it, over any number of steps, may move. Only issues with a
/// bar in `bars` are walked; an issue that need not move is left out, and so are issues on a loop.
/// In the order of the schedule, the earliest first.
#[must_use]
pub fn move_after<H: BuildHasher>(
    graph: &DependencyGraph,
    bars: &HashMap<IssueId, Days, H>,
    issue: IssueId,
    after: Date,
) -> Vec<Moved> {
    let reached: HashSet<IssueId> = graph
        .waiting_from(issue, |at| bars.contains_key(&at))
        .collect();
    let schedule = graph.schedule(bars, Some(Seed { issue, after }), |at| {
        reached.contains(&at)
    });
    schedule
        .order
        .iter()
        .filter_map(|at| {
            let (from, to) = (bars.get(at).copied()?, schedule.days(*at)?);
            (from != to).then_some(Moved {
                issue: *at,
                from,
                to,
            })
        })
        .collect()
}
