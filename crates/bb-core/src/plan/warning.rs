//! The four plan warnings (PLAN-13, ADR-0030, decision 5, ADR-0043, decisions 5 and 7), one enum.
//! A warning names issues by their IDs and no person at all: the use case shows them as the reader
//! may know them, and leaves a warning out when the reader may not see an issue in it. Warnings
//! never move anything: every way out is a person's choice.

use std::collections::{BTreeMap, HashMap};
use std::hash::BuildHasher;

use jiff::civil::Date;
use serde::Serialize;

use super::Days;
use super::bar::{Bar, Shortfall, WorkedOut};
use crate::id::IssueId;
use crate::message::MessageKey;
use crate::relation::Dependency;
use crate::work_times::DayMinutes;

words! {
    /// The kind of a plan warning. Its message is the sentence of the warning, filled in with what
    /// the warning holds (UX §4.7, *The list of plan warnings*).
    pub enum WarningKind {
        /// The person on the bar is away on days of the bar.
        Away => "away",
        /// The days from the start date to the due date do not hold the effort.
        NotEnoughDays => "not-enough-days",
        /// The issue starts before an issue it waits for ends, or a sub-issue ends after its due
        /// date.
        Delay => "delay",
        /// The person on the bar has more planned on a day in the project than their share there.
        TooMuchAtOnce => "too-much-at-once",
    }
    messages sentence = "plan-warning.";
}

words! {
    /// A way out of a plan warning: each is a person's choice, and nothing moves until it is made
    /// (ADR-0030, decision 5).
    pub enum WayOut {
        /// Move the issue, and the issues that wait for it, to start after the days that are in the
        /// way; a preview names every issue and its new dates first.
        MoveItAfter => "move-it-after",
        /// Give the bar the days it needs: the due date it needs is answered, and it changes only
        /// when the person confirms it.
        MakeItLonger => "make-it-longer",
        /// The usual change of *Assigned to*, with nobody suggested (COMP-10).
        GiveToSomeoneElse => "give-to-someone-else",
        /// Leave the plan as it is: the warning stays, faded, until the dates of its issue change.
        KeepAndAccept => "keep-and-accept",
    }
    messages label = "plan-way-out.";
}

words_in_api!(
    WayOut,
    "A way out of a plan warning: move the issue after what is in the way, make it longer, give it to someone else, or keep the plan and accept the warning."
);

words_in_api!(
    WarningKind,
    "The kind of a plan warning: the person on the bar is away, not enough days before the due date, a delay along a dependency or after a sub-issue, or too much at once for the person."
);

/// The sentence of *Away* for a person the reader may not see by name: *"the assigned person is
/// away"* (ADR-0030, decision 5).
pub const AWAY_NAMELESS: MessageKey = crate::derived_key!("plan-warning.", "away-hidden");

/// The sentence of *Delay* of a goal after one of its sub-issues (ADR-0043, decision 7).
pub const DELAY_OF_GOAL: MessageKey = crate::derived_key!("plan-warning.", "delay-goal");

/// Why an issue has the warning *Delay*.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "cause", rename_all = "kebab-case")]
pub enum Delay {
    /// It waits for `first`, which ends `days` days after the day it starts (0: on that day;
    /// ADR-0043, decision 5).
    Dependency {
        /// The issue that must finish first.
        first: IssueId,
        /// How many days after its start `first` ends.
        days: u32,
        /// The last day of `first`: *Move it after* starts the issue on the day after it.
        after: Date,
    },
    /// Its sub-issue `sub_issue` ends `days` days after its due date (ADR-0043, decision 7).
    SubIssue {
        /// The sub-issue.
        sub_issue: IssueId,
        /// How many days after the due date it ends.
        days: u32,
        /// The last day of the sub-issue: the due date *Make it longer* answers.
        due_needed: Date,
    },
}

/// A plan warning of one issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Warning {
    /// The person on the bar is away on these days of a bar whose dates people set.
    Away {
        /// From the first to the last away day inside the bar.
        days: Days,
        /// The due date that gives back the work days the away days take (*Make it longer*); none
        /// when the known days do not reach it.
        due_needed: Option<Date>,
    },
    /// The days do not hold the effort.
    NotEnoughDays(Shortfall),
    /// A delay.
    Delay(Delay),
    /// More is planned for the person on the bar than they can work in the project.
    TooMuchAtOnce {
        /// From the first to the last day of the bar that holds too much.
        days: Days,
        /// The day with the most too much.
        worst: Date,
        /// The minutes planned for the person on that day in the project.
        planned: u32,
        /// The minutes the person can work on that day in the project.
        can: u32,
        /// The last day of the person's other work on those days: *Move it after* starts the
        /// issue on the day after it.
        after: Date,
    },
}

impl Warning {
    /// The kind of the warning.
    #[must_use]
    pub const fn kind(&self) -> WarningKind {
        match self {
            Self::Away { .. } => WarningKind::Away,
            Self::NotEnoughDays(_) => WarningKind::NotEnoughDays,
            Self::Delay(_) => WarningKind::Delay,
            Self::TooMuchAtOnce { .. } => WarningKind::TooMuchAtOnce,
        }
    }

    /// The sentence of the warning: the message of its kind, the sentence of a goal for a delay
    /// after a sub-issue, and the sentence that names nobody for the away time of a person the
    /// reader may not see by name (ADR-0030, decision 5: *"the assigned person is away"*).
    #[must_use]
    pub const fn sentence(&self, person_named: bool) -> MessageKey {
        match self {
            Self::Away { .. } if !person_named => AWAY_NAMELESS,
            Self::Delay(Delay::SubIssue { .. }) => DELAY_OF_GOAL,
            _ => self.kind().sentence(),
        }
    }

    /// The ways out that fit the warning, in the order they are offered (UX §4.7).
    #[must_use]
    pub fn ways_out(&self) -> Vec<WayOut> {
        use WayOut::{GiveToSomeoneElse, KeepAndAccept, MakeItLonger, MoveItAfter};
        let longer = self.due_needed().map(|_| MakeItLonger);
        match self {
            Self::Away { .. } => [Some(MoveItAfter), longer, Some(GiveToSomeoneElse)]
                .into_iter()
                .flatten()
                .chain([KeepAndAccept])
                .collect(),
            Self::NotEnoughDays(_) => longer
                .into_iter()
                .chain([GiveToSomeoneElse, KeepAndAccept])
                .collect(),
            Self::Delay(Delay::Dependency { .. }) => vec![MoveItAfter, KeepAndAccept],
            Self::Delay(Delay::SubIssue { .. }) => vec![MakeItLonger, KeepAndAccept],
            Self::TooMuchAtOnce { .. } => vec![MoveItAfter, GiveToSomeoneElse, KeepAndAccept],
        }
    }

    /// The due date *Make it longer* answers, when it fits the warning and is known.
    #[must_use]
    pub const fn due_needed(&self) -> Option<Date> {
        match self {
            Self::Away { due_needed, .. } => *due_needed,
            Self::NotEnoughDays(shortfall) => shortfall.due_needed,
            Self::Delay(Delay::SubIssue { due_needed, .. }) => Some(*due_needed),
            Self::Delay(Delay::Dependency { .. }) | Self::TooMuchAtOnce { .. } => None,
        }
    }

    /// The day *Move it after* starts the issue after, when it fits the warning.
    #[must_use]
    pub const fn move_after(&self) -> Option<Date> {
        match self {
            Self::Away { days, .. } => Some(days.last),
            Self::Delay(Delay::Dependency { after, .. }) | Self::TooMuchAtOnce { after, .. } => {
                Some(*after)
            }
            Self::NotEnoughDays(_) | Self::Delay(Delay::SubIssue { .. }) => None,
        }
    }

    /// The other issue the warning names, if it names one: an issue it waits for, or a sub-issue.
    #[must_use]
    pub const fn other_issue(&self) -> Option<IssueId> {
        match self {
            Self::Delay(Delay::Dependency { first, .. }) => Some(*first),
            Self::Delay(Delay::SubIssue { sub_issue, .. }) => Some(*sub_issue),
            _ => None,
        }
    }
}

/// *Away*: the days of a bar whose dates people set on which its person on the bar is away. A bar
/// that *Plan by effort* worked out has its away days as a gap, and no warning.
#[must_use]
pub fn away(bar: &Bar, days: &[DayMinutes]) -> Option<Warning> {
    if bar.worked_out != WorkedOut::Neither {
        return None;
    }
    let away: Vec<Date> = days
        .iter()
        .filter(|day| bar.days.contains(day.date) && day.away.count() > 0)
        .map(|day| day.date)
        .collect();
    let (first, last) = (*away.first()?, *away.last()?);
    // The away days given back after the bar, on days the person works.
    let due_needed = days
        .iter()
        .filter(|day| day.date > bar.days.last && day.minutes > 0)
        .nth(away.len() - 1)
        .map(|day| day.date);
    Some(Warning::Away {
        days: Days { first, last },
        due_needed,
    })
}

/// *Delay* along dependencies: each issue that starts on or before the last day of an issue it
/// waits for, with that issue (ADR-0043, decision 5). A dependency with an end that has no bar is
/// not checked.
#[must_use]
pub fn delays<H: BuildHasher>(
    dependencies: &[Dependency],
    bars: &HashMap<IssueId, Bar, H>,
) -> Vec<(IssueId, Warning)> {
    dependencies
        .iter()
        .filter_map(|(first, then)| {
            let (before, after) = (bars.get(first)?, bars.get(then)?);
            (after.days.first <= before.days.last).then(|| {
                let days = Days::between(after.days.first, before.days.last);
                (
                    *then,
                    Warning::Delay(Delay::Dependency {
                        first: *first,
                        days: u32::try_from(days).unwrap_or(u32::MAX),
                        after: before.days.last,
                    }),
                )
            })
        })
        .collect()
}

/// *Delay* of a goal: each issue with a due date and a sub-issue whose bar ends after it, with the
/// sub-issue that ends last (ADR-0043, decision 7). `parents` are pairs of a sub-issue and the
/// issue it sits under; `dues` the due dates the reader may read.
#[must_use]
pub fn sub_issue_delays<H: BuildHasher, G: BuildHasher>(
    parents: &[(IssueId, IssueId)],
    bars: &HashMap<IssueId, Bar, H>,
    dues: &HashMap<IssueId, Date, G>,
) -> Vec<(IssueId, Warning)> {
    let mut latest: BTreeMap<IssueId, (IssueId, Date)> = BTreeMap::new();
    for (child, parent) in parents {
        let (Some(bar), Some(due)) = (bars.get(child), dues.get(parent)) else {
            continue;
        };
        if bar.days.last <= *due {
            continue;
        }
        let entry = latest.entry(*parent).or_insert((*child, bar.days.last));
        if bar.days.last > entry.1 {
            *entry = (*child, bar.days.last);
        }
    }
    latest
        .into_iter()
        .filter_map(|(parent, (child, last))| {
            let days = Days::between(*dues.get(&parent)?, last);
            Some((
                parent,
                Warning::Delay(Delay::SubIssue {
                    sub_issue: child,
                    days: u32::try_from(days).unwrap_or(u32::MAX),
                    due_needed: last,
                }),
            ))
        })
        .collect()
}

/// *Too much at once* for one person in one project: each issue planned on a day on which the
/// minutes planned for the person add up to more than they can work there (work hours times the
/// share). `loads` are the minutes of each issue on each day ([`super::bar::load`]), only of the
/// issues the person is the person on the bar of; `days` the person's days.
#[must_use]
pub fn too_much_at_once(
    loads: &[(IssueId, Vec<(Date, u32)>)],
    days: &[DayMinutes],
) -> Vec<(IssueId, Warning)> {
    let mut planned: BTreeMap<Date, u32> = BTreeMap::new();
    for (_, load) in loads {
        for (date, minutes) in load {
            let sum = planned.entry(*date).or_default();
            *sum = sum.saturating_add(*minutes);
        }
    }
    let can: HashMap<Date, u32> = days.iter().map(|day| (day.date, day.minutes)).collect();
    let too_much: BTreeMap<Date, (u32, u32)> = planned
        .into_iter()
        .filter_map(|(date, sum)| {
            let can = can.get(&date).copied().unwrap_or(0);
            (sum > can).then_some((date, (sum, can)))
        })
        .collect();
    loads
        .iter()
        .filter_map(|(issue, load)| {
            let hit: Vec<(Date, u32, u32)> = load
                .iter()
                .filter_map(|(date, _)| too_much.get(date).map(|(sum, can)| (*date, *sum, *can)))
                .collect();
            let (first, last) = (hit.first()?.0, hit.last()?.0);
            let (worst, planned, can) = hit
                .iter()
                .copied()
                .max_by_key(|(date, sum, can)| (sum - can, std::cmp::Reverse(*date)))?;
            // The last day of the other work planned on those days.
            let after = loads
                .iter()
                .filter(|(other, load)| {
                    other != issue
                        && load
                            .iter()
                            .any(|(date, _)| hit.iter().any(|(hit, _, _)| hit == date))
                })
                .filter_map(|(_, load)| load.last().map(|(date, _)| *date))
                .max()
                .unwrap_or(last);
            Some((
                *issue,
                Warning::TooMuchAtOnce {
                    days: Days { first, last },
                    worst,
                    planned,
                    can,
                    after,
                },
            ))
        })
        .collect()
}
