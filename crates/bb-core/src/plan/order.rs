//! The order along dependencies that *Move it after* moves issues in (ADR-0043, decision 5): the
//! issue that waits, and the issues that wait for it in turn, each so that it starts on the first
//! day after the one before it ends. A preview of these moves is answered before anything is saved;
//! the move itself is one run of the dates (`POST /api/v1/issues/move-dates`).
//!
//! The walk is the one step of every walk along dependencies,
//! [`DependencyGraph::waiting`](crate::relation::DependencyGraph::waiting). Issues on a loop that
//! was stored before the date effect existed are left where they are.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::hash::BuildHasher;

use jiff::civil::Date;
use serde::Serialize;

use super::Days;
use crate::id::IssueId;
use crate::relation::DependencyGraph;

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

/// The moves of *Move it after* for `issue`: it starts on the day after `after` (the last day of
/// the issue it waits for, of the away days, or of the person's other work), and every issue that
/// waits for a moved issue, over any number of steps, starts on the day after the last of the
/// issues it waits for ends, when it starts earlier. Each keeps its number of days. Only issues
/// with a bar in `bars` are walked; an issue that need not move is left out, and so are issues on
/// a loop. In the order of the walk, the earliest first.
#[must_use]
pub fn move_after<H: BuildHasher>(
    graph: &DependencyGraph,
    bars: &HashMap<IssueId, Days, H>,
    issue: IssueId,
    after: Date,
) -> Vec<Moved> {
    let waiting = issue;
    // The issues reached from the one that waits.
    let mut reached: HashSet<IssueId> = HashSet::new();
    let mut queue = VecDeque::from([waiting]);
    while let Some(at) = queue.pop_front() {
        if !bars.contains_key(&at) || !reached.insert(at) {
            continue;
        }
        queue.extend(graph.waiting(at).iter().copied());
    }
    // The issues each reached issue waits for, among the reached ones and the ones that stay.
    let mut before: BTreeMap<IssueId, Vec<IssueId>> =
        reached.iter().map(|at| (*at, Vec::new())).collect();
    for from in bars.keys().copied() {
        for to in graph.waiting(from) {
            if let Some(list) = before.get_mut(to) {
                list.push(from);
            }
        }
    }
    // Kahn's order: an issue comes when every reached issue it waits for has come.
    let mut open: BTreeMap<IssueId, usize> = before
        .iter()
        .map(|(at, list)| {
            (
                *at,
                list.iter().filter(|from| reached.contains(from)).count(),
            )
        })
        .collect();
    let mut ready: VecDeque<IssueId> = open
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(at, _)| *at)
        .collect();
    // The days of every bar after the moves so far: those that stay keep theirs.
    let mut placed: HashMap<IssueId, Days> = bars
        .iter()
        .filter(|(at, _)| !reached.contains(at))
        .map(|(at, days)| (*at, *days))
        .collect();
    let mut moved = Vec::new();
    while let Some(at) = ready.pop_front() {
        let Some(now) = bars.get(&at).copied() else {
            continue;
        };
        let earliest = before
            .get(&at)
            .into_iter()
            .flatten()
            .filter_map(|from| placed.get(from))
            .map(|days| days.last)
            .chain((at == waiting).then_some(after))
            .max()
            .and_then(|last| last.tomorrow().ok());
        let to = match earliest {
            Some(earliest) if earliest > now.first => now
                .shifted(Days::between(now.first, earliest))
                .unwrap_or(now),
            _ => now,
        };
        if to != now {
            moved.push(Moved {
                issue: at,
                from: now,
                to,
            });
        }
        placed.insert(at, to);
        for next in graph.waiting(at) {
            if let Some(count) = open.get_mut(next) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    ready.push_back(*next);
                }
            }
        }
    }
    moved
}
