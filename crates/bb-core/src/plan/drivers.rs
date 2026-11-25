//! Deadline drivers and the two warnings of impossible dependencies (PLAN-05, ADR-0043, amendment
//! of 2026-11-24).
//!
//! Every rule here stands on one schedule, [`Chain`]: the days each bar would have if every issue
//! started no earlier than the day after the issues it waits for end, each keeping its number of
//! days. It is worked out, never shown and never stored: the bars keep the dates people set, and
//! nothing moves by itself (ADR-0043, decision 5). Loops of dependencies stored before loops were
//! refused are left out of it: nothing on them can start (*Loop*).
//!
//! - The **drivers** of an end are the issues whose delay by a day moves that end by a day: the
//!   members that end last, and, walking back, every issue they wait for with no slack.
//! - ***Cannot fit***: an issue is due before the issues it waits for can end.
//! - ***Loop***: an issue waits for itself.
//!
//! Pure, and given only the issues the reader sees: a chain through a hidden issue is not followed,
//! and nothing tells that one exists.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::hash::BuildHasher;

use jiff::civil::Date;
use serde::Serialize;

use super::Days;
use super::bar::{Bar, WorkedOut};
use super::warning::Warning;
use crate::id::{IssueId, ReleaseId};
use crate::relation::{Dependency, dependency_loops};

/// An end of the view that deadline drivers set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "kebab-case")]
pub enum EndOf {
    /// The end of a goal with a *Due date*: the last of its sub-issues.
    Goal(IssueId),
    /// The end of a release with a *Release date*: the last of the issues planned for it.
    Release(ReleaseId),
    /// The last day of the view: the last of its bars.
    View,
}

/// An end and the issues whose bars make it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct End {
    /// Which end.
    pub of: EndOf,
    /// The issues whose last day it is the last of.
    pub members: Vec<IssueId>,
}

/// The dependencies that lie on a loop, each named once.
fn on_loops(dependencies: &[Dependency]) -> HashSet<Dependency> {
    dependency_loops(dependencies)
        .iter()
        .flat_map(|on_it| {
            let next = on_it.iter().skip(1).chain(on_it.first());
            on_it.iter().copied().zip(next.copied()).collect::<Vec<_>>()
        })
        .collect()
}

/// *Loop*: every issue on a loop of dependencies, with the issue on it that it waits for directly.
/// An issue on two loops is named once, with the first.
#[must_use]
pub fn loops(dependencies: &[Dependency]) -> Vec<(IssueId, Warning)> {
    let mut named: BTreeMap<IssueId, IssueId> = BTreeMap::new();
    for on_it in dependency_loops(dependencies) {
        // Each issue waits for the one before it on the loop; the first for the last.
        let before = on_it.last().into_iter().chain(on_it.iter());
        for (issue, waits_for) in on_it.iter().zip(before) {
            named.entry(*issue).or_insert(*waits_for);
        }
    }
    named
        .into_iter()
        .map(|(issue, waits_for)| (issue, Warning::Loop { waits_for }))
        .collect()
}

/// The schedule of the chain: the earliest days of every bar along the dependencies that lie on no
/// loop.
pub struct Chain {
    /// The issues each issue waits for, among the bars.
    before: HashMap<IssueId, Vec<IssueId>>,
    /// The earliest days of each bar: as late as the bar itself, or the day after the last of the
    /// issues it waits for ends, whichever is later, with its number of days.
    earliest: HashMap<IssueId, Days>,
}

impl Chain {
    /// The schedule of these bars along these dependencies. A dependency with an end that has no
    /// bar, or that lies on a loop, is left out.
    #[must_use]
    pub fn new<H: BuildHasher>(
        dependencies: &[Dependency],
        bars: &HashMap<IssueId, Bar, H>,
    ) -> Self {
        let looped = on_loops(dependencies);
        let mut before: HashMap<IssueId, Vec<IssueId>> = HashMap::new();
        let mut after: HashMap<IssueId, Vec<IssueId>> = HashMap::new();
        for (first, then) in dependencies.iter().copied() {
            if looped.contains(&(first, then))
                || !bars.contains_key(&first)
                || !bars.contains_key(&then)
            {
                continue;
            }
            before.entry(then).or_default().push(first);
            after.entry(first).or_default().push(then);
        }
        // Kahn's order, the smallest ID first, so the same plan gives the same schedule.
        let mut open: BTreeMap<IssueId, usize> = bars
            .keys()
            .map(|id| (*id, before.get(id).map_or(0, Vec::len)))
            .collect();
        let mut ready: VecDeque<IssueId> = open
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut earliest: HashMap<IssueId, Days> = HashMap::new();
        while let Some(at) = ready.pop_front() {
            let Some(now) = bars.get(&at).map(|bar| bar.days) else {
                continue;
            };
            let start = before
                .get(&at)
                .into_iter()
                .flatten()
                .filter_map(|first| earliest.get(first))
                .map(|days| days.last)
                .max()
                .and_then(|last| last.tomorrow().ok());
            let days = match start {
                Some(start) if start > now.first => {
                    now.shifted(Days::between(now.first, start)).unwrap_or(now)
                }
                _ => now,
            };
            earliest.insert(at, days);
            for next in after.get(&at).into_iter().flatten() {
                if let Some(count) = open.get_mut(next) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        ready.push_back(*next);
                    }
                }
            }
        }
        Self { before, earliest }
    }

    /// The earliest days of the bar of `issue`; none without a bar.
    #[must_use]
    pub fn earliest(&self, issue: IssueId) -> Option<Days> {
        self.earliest.get(&issue).copied()
    }

    /// The end of these members: the last of their earliest last days; none when none has a bar.
    #[must_use]
    pub fn end(&self, members: &[IssueId]) -> Option<Date> {
        members
            .iter()
            .filter_map(|member| self.earliest(*member))
            .map(|days| days.last)
            .max()
    }

    /// The drivers of these members: the ones that end on their end, and every issue they wait for,
    /// over any number of steps, that ends on the day before the one after it starts.
    #[must_use]
    pub fn drivers(&self, members: &[IssueId]) -> BTreeSet<IssueId> {
        let Some(end) = self.end(members) else {
            return BTreeSet::new();
        };
        let mut found: BTreeSet<IssueId> = members
            .iter()
            .copied()
            .filter(|member| self.earliest(*member).is_some_and(|days| days.last == end))
            .collect();
        let mut queue: VecDeque<IssueId> = found.iter().copied().collect();
        while let Some(at) = queue.pop_front() {
            let Some(start) = self.earliest(at).map(|days| days.first) else {
                continue;
            };
            for first in self.before.get(&at).into_iter().flatten() {
                let binds = self
                    .earliest(*first)
                    .and_then(|days| days.last.tomorrow().ok())
                    == Some(start);
                if binds && found.insert(*first) {
                    queue.push_back(*first);
                }
            }
        }
        found
    }

    /// The drivers of every end, each issue with the ends it sets, in the order of the ends.
    #[must_use]
    pub fn drivers_of(&self, ends: &[End]) -> BTreeMap<IssueId, Vec<EndOf>> {
        let mut sets: BTreeMap<IssueId, Vec<EndOf>> = BTreeMap::new();
        for end in ends {
            for issue in self.drivers(&end.members) {
                let list = sets.entry(issue).or_default();
                if !list.contains(&end.of) {
                    list.push(end.of);
                }
            }
        }
        sets
    }

    /// *Cannot fit*: each issue whose due date (a last day people set, not one *Plan by effort*
    /// worked out) is on or before the earliest day the issues it waits for can end, with the one
    /// of them that ends last and the due date it needs to keep its number of days after them.
    #[must_use]
    pub fn cannot_fit<H: BuildHasher>(
        &self,
        bars: &HashMap<IssueId, Bar, H>,
    ) -> Vec<(IssueId, Warning)> {
        let mut found: Vec<(IssueId, Warning)> = bars
            .iter()
            .filter(|(_, bar)| bar.worked_out != WorkedOut::Due)
            .filter_map(|(id, bar)| {
                let (first, ends) = self
                    .before
                    .get(id)?
                    .iter()
                    .filter_map(|first| self.earliest(*first).map(|days| (*first, days.last)))
                    .max_by_key(|(first, last)| (*last, std::cmp::Reverse(*first)))?;
                if bar.days.last > ends {
                    return None;
                }
                let length = Days::between(bar.days.first, bar.days.last) + 1;
                let due_needed = ends.checked_add(jiff::Span::new().days(length)).ok()?;
                Some((
                    *id,
                    Warning::CannotFit {
                        first,
                        ends,
                        due_needed,
                    },
                ))
            })
            .collect();
        found.sort_by_key(|(id, _)| *id);
        found
    }
}

#[cfg(test)]
mod tests;
