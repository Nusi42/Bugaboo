//! Deadline drivers and the two warnings of impossible dependencies (PLAN-05, ADR-0043, amendment
//! of 2026-11-24).
//!
//! Every rule here stands on the one schedule along dependencies ([`super::order`], which *Move it
//! after* uses too), with every bar free to move: the days each bar would have if every issue
//! started no earlier than the day after the issues it waits for end, each keeping its number of
//! days. It is worked out, never shown and never stored: the bars keep the dates people set, and
//! nothing moves by itself (ADR-0043, decision 5). The issues on a loop keep their days, and the
//! dependencies of the loop count for nothing (*Loop*).
//!
//! - The **drivers** of an end are the issues whose delay by a day moves that end by a day: the
//!   members that end last, and, walking back, every issue they wait for with no slack.
//! - ***Cannot fit***: an issue is due before the issues it waits for can end.
//! - ***Loop***: an issue waits for itself.
//!
//! Pure, and given only the issues the reader sees: a chain through a hidden issue is not followed,
//! and nothing tells that one exists.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::hash::BuildHasher;

use jiff::civil::Date;
use serde::Serialize;

use super::Days;
use super::bar::Bar;
use super::order::Schedule;
use super::warning::Warning;
use crate::id::{IssueId, ReleaseId};
use crate::relation::{Dependency, DependencyGraph, dependency_loops};

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

/// The deadline drivers of a page: each issue with the ends it sets, and the dependencies of the
/// chains, along which a delay moves an end.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Drivers {
    /// Each driver with the ends it sets, in the order of the ends.
    pub ends: BTreeMap<IssueId, Vec<EndOf>>,
    /// The dependencies with no slack between two drivers of one end.
    pub links: BTreeSet<Dependency>,
}

/// The chain of a plan: the one schedule along its dependencies ([`DependencyGraph::schedule`]),
/// where every bar may move, worked out and never shown.
pub struct Chain {
    schedule: Schedule,
}

impl Chain {
    /// The schedule of these bars along these dependencies (the loop rule of [`super::order`]).
    #[must_use]
    pub fn new<H: BuildHasher>(
        dependencies: &[Dependency],
        bars: &HashMap<IssueId, Bar, H>,
    ) -> Self {
        let days: HashMap<IssueId, Days> = bars.iter().map(|(id, bar)| (*id, bar.days)).collect();
        Self {
            schedule: DependencyGraph::new(dependencies).schedule(&days, None, |_| true),
        }
    }

    /// The earliest days of the bar of `issue`; none without a bar.
    #[must_use]
    pub fn earliest(&self, issue: IssueId) -> Option<Days> {
        self.schedule.days(issue)
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
    /// over any number of steps, that ends on the day before the one after it starts; with the
    /// dependencies walked.
    #[must_use]
    pub fn drivers(&self, members: &[IssueId]) -> (BTreeSet<IssueId>, BTreeSet<Dependency>) {
        let mut links = BTreeSet::new();
        let Some(end) = self.end(members) else {
            return (BTreeSet::new(), links);
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
            for first in self.schedule.before(at) {
                let binds = self
                    .earliest(*first)
                    .and_then(|days| days.last.tomorrow().ok())
                    == Some(start);
                if binds {
                    links.insert((*first, at));
                    if found.insert(*first) {
                        queue.push_back(*first);
                    }
                }
            }
        }
        (found, links)
    }

    /// The drivers of every end ([`Drivers`]).
    #[must_use]
    pub fn drivers_of(&self, ends: &[End]) -> Drivers {
        let mut drivers = Drivers::default();
        for end in ends {
            let (found, links) = self.drivers(&end.members);
            for issue in found {
                let list = drivers.ends.entry(issue).or_default();
                if !list.contains(&end.of) {
                    list.push(end.of);
                }
            }
            drivers.links.extend(links);
        }
        drivers
    }

    /// *Cannot fit*: each issue with a due date people set (`dues`) that is on or before the
    /// earliest day the issues it waits for can end, with the one of them that ends last and the
    /// due date it needs to keep the number of days of its bar after them. An issue without a due
    /// date has no such warning, whatever its bar.
    #[must_use]
    pub fn cannot_fit<H: BuildHasher, G: BuildHasher>(
        &self,
        bars: &HashMap<IssueId, Bar, H>,
        dues: &HashMap<IssueId, Date, G>,
    ) -> Vec<(IssueId, Warning)> {
        let mut found: Vec<(IssueId, Warning)> = dues
            .iter()
            .filter_map(|(id, due)| {
                let bar = bars.get(id)?;
                let (first, ends) = self
                    .schedule
                    .before(*id)
                    .iter()
                    .filter_map(|first| self.earliest(*first).map(|days| (*first, days.last)))
                    .max_by_key(|(first, last)| (*last, std::cmp::Reverse(*first)))?;
                if *due > ends {
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
