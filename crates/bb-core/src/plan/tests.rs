//! The rules of the plan as tables of cases.

use std::collections::HashMap;

use jiff::civil::{Date, date};

use super::bar::{Bar, BarInput, Shortfall, WorkedOut, bar, load, shortfall};
use super::order::move_after;
use super::warning::{Delay, Warning, away, delays, sub_issue_delays, too_much_at_once};
use super::{Days, TimeFactor};
use crate::id::{IssueId, SequentialIds};
use crate::relation::DependencyGraph;
use crate::time::Minutes;
use crate::work_times::{DayMinutes, Halves, Share, UsualDays, minutes_on};

/// The days from `first` on, `count` of them, each with the minutes of a person with `hours` on
/// Monday to Friday and `share`; away on the days of `away` (whole days).
fn person_days(first: Date, count: i64, hours: u16, share: u8, away: &[Date]) -> Vec<DayMinutes> {
    let usual = UsualDays::new(core::array::from_fn(|at| crate::work_times::UsualDay {
        works: at < 5,
        minutes: hours * 60,
    }))
    .unwrap();
    let share = Share::new(share).unwrap();
    (0..count)
        .map(|offset| {
            let date = first.checked_add(jiff::Span::new().days(offset)).unwrap();
            let is_away = away.contains(&date);
            let minutes = if is_away {
                0
            } else {
                minutes_on(&usual, share, date, &[], false)
            };
            DayMinutes {
                date,
                minutes,
                away: if is_away { Halves::BOTH } else { Halves::NONE },
                holiday: None,
            }
        })
        .collect()
}

fn hours(hours: u32) -> Minutes {
    Minutes::new(hours * 60)
}

fn ids(count: usize) -> Vec<IssueId> {
    let mut sequence = SequentialIds::default();
    (0..count)
        .map(|_| IssueId::generate(&mut sequence).unwrap())
        .collect()
}

// Monday, 2 November 2026.
const MONDAY: Date = date(2026, 11, 2);

#[test]
fn plan_12_plan_by_effort_counts_work_hours_share_away_days_and_holidays() {
    // 6 hours, 50 %, 9 hours of effort: three days, Monday to Wednesday.
    let days = person_days(MONDAY, 21, 6, 50, &[]);
    let cases: [(&str, BarInput<'_>, Option<Days>, WorkedOut); 6] = [
        (
            "forward from the start date",
            BarInput {
                start: Some(MONDAY),
                due: None,
                effort: Some(hours(9)),
                days: &days,
            },
            Days::new(MONDAY, date(2026, 11, 4)),
            WorkedOut::Due,
        ),
        (
            "back from the due date",
            BarInput {
                start: None,
                due: Some(date(2026, 11, 4)),
                effort: Some(hours(9)),
                days: &days,
            },
            Days::new(MONDAY, date(2026, 11, 4)),
            WorkedOut::Start,
        ),
        (
            "the weekend counts nothing",
            BarInput {
                start: Some(date(2026, 11, 5)),
                due: None,
                effort: Some(hours(9)),
                days: &days,
            },
            Days::new(date(2026, 11, 5), date(2026, 11, 9)),
            WorkedOut::Due,
        ),
        (
            "without plan by effort one date is one day",
            BarInput {
                start: Some(MONDAY),
                due: None,
                effort: None,
                days: &days,
            },
            Some(Days::one(MONDAY)),
            WorkedOut::Neither,
        ),
        (
            "both dates are kept",
            BarInput {
                start: Some(MONDAY),
                due: Some(date(2026, 11, 3)),
                effort: Some(hours(9)),
                days: &days,
            },
            Days::new(MONDAY, date(2026, 11, 3)),
            WorkedOut::Neither,
        ),
        (
            "no date is no bar",
            BarInput {
                start: None,
                due: None,
                effort: Some(hours(9)),
                days: &days,
            },
            None,
            WorkedOut::Neither,
        ),
    ];
    for (name, input, expected, worked_out) in cases {
        let found = bar(&input);
        assert_eq!(found.map(|bar| bar.days), expected, "{name}");
        if let Some(found) = found {
            assert_eq!(found.worked_out, worked_out, "{name}");
        }
    }
    // An away day and a public holiday count nothing: the bar gets longer by them.
    let away_tuesday = person_days(MONDAY, 21, 6, 50, &[date(2026, 11, 3)]);
    let mut holiday_wednesday = person_days(MONDAY, 21, 6, 50, &[]);
    holiday_wednesday[2].minutes = 0;
    for days in [&away_tuesday, &holiday_wednesday] {
        let found = bar(&BarInput {
            start: Some(MONDAY),
            due: None,
            effort: Some(hours(9)),
            days,
        })
        .unwrap();
        assert_eq!(found.days, Days::new(MONDAY, date(2026, 11, 5)).unwrap());
    }
}

#[test]
fn plan_12_a_fixed_due_date_never_moves() {
    let days = person_days(MONDAY, 21, 6, 50, &[]);
    let fixed = bar(&BarInput {
        start: Some(MONDAY),
        due: Some(date(2026, 11, 3)),
        effort: Some(hours(9)),
        days: &days,
    })
    .unwrap();
    assert_eq!(fixed.days.last, date(2026, 11, 3));
    assert_eq!(
        shortfall(&fixed, hours(9), &days),
        Some(Shortfall {
            needs: Some(3),
            has: 2,
            due_needed: Some(date(2026, 11, 4)),
        })
    );
    // Enough days: no warning; a worked-out bar holds its effort by being as long as it.
    let roomy = Bar {
        days: Days::new(MONDAY, date(2026, 11, 6)).unwrap(),
        worked_out: WorkedOut::Neither,
    };
    assert_eq!(shortfall(&roomy, hours(9), &days), None);
    let worked = Bar {
        worked_out: WorkedOut::Due,
        ..fixed
    };
    assert_eq!(shortfall(&worked, hours(9), &days), None);
}

#[test]
fn plan_14_the_time_factor_is_a_share_of_the_estimate() {
    let estimate = hours(64);
    let cases: [(u16, Option<u32>); 4] = [
        (100, Some(64 * 60)),
        (125, Some(80 * 60)),
        (5, None),
        (1001, None),
    ];
    for (percent, effort) in cases {
        assert_eq!(
            TimeFactor::new(percent).map(|factor| factor.effort(estimate).get()),
            effort,
            "{percent}"
        );
    }
    // *Takes me 10 days* against an estimate of 8 days: ×1.25.
    assert_eq!(
        TimeFactor::of_length(hours(64), hours(80)).map(TimeFactor::percent),
        Some(125)
    );
    assert_eq!(TimeFactor::of_length(Minutes::new(0), hours(8)), None);
    assert_eq!(TimeFactor::of_length(hours(1), hours(100)), None);
    assert_eq!(TimeFactor::default(), TimeFactor::ONE);
    // Rounded up to the minute: a factor never makes work disappear.
    assert_eq!(
        TimeFactor::new(150).unwrap().effort(Minutes::new(1)).get(),
        2
    );
}

#[test]
fn plan_13_a_delay_is_shown_along_dependencies_and_moves_nothing() {
    let [a, b, c] = ids(3).try_into().unwrap();
    let fixed = |first, last| Bar {
        days: Days::new(first, last).unwrap(),
        worked_out: WorkedOut::Neither,
    };
    let bars = HashMap::from([
        (a, fixed(MONDAY, date(2026, 11, 5))),
        (b, fixed(date(2026, 11, 4), date(2026, 11, 6))),
        (c, fixed(date(2026, 11, 9), date(2026, 11, 10))),
    ]);
    // `b` waits for `a` and starts two days before `a` ends; `c` waits for `b` and starts after.
    let found = delays(&[(a, b), (b, c)], &bars);
    assert_eq!(
        found,
        [(
            b,
            Warning::Delay(Delay::Dependency {
                first: a,
                days: 1,
                after: date(2026, 11, 5),
            })
        )]
    );
    // Starting on the day an issue ends is a delay of one day; a dependency on an issue with no
    // bar is not checked.
    let touching = HashMap::from([
        (a, fixed(MONDAY, date(2026, 11, 5))),
        (b, fixed(date(2026, 11, 5), date(2026, 11, 6))),
    ]);
    assert_eq!(
        delays(&[(a, b), (c, a)], &touching),
        [(
            b,
            Warning::Delay(Delay::Dependency {
                first: a,
                days: 0,
                after: date(2026, 11, 5),
            })
        )]
    );
    // Nothing moved: the bars are what they were.
    assert_eq!(bars[&b].days.first, date(2026, 11, 4));
}

#[test]
fn plan_13_a_sub_issue_after_its_goals_due_date_gives_the_goal_a_delay() {
    let [goal, early, late, later] = ids(4).try_into().unwrap();
    let fixed = |first, last| Bar {
        days: Days::new(first, last).unwrap(),
        worked_out: WorkedOut::Neither,
    };
    let bars = HashMap::from([
        (early, fixed(MONDAY, date(2026, 11, 3))),
        (late, fixed(MONDAY, date(2026, 11, 9))),
        (later, fixed(MONDAY, date(2026, 11, 11))),
    ]);
    let dues = HashMap::from([(goal, date(2026, 11, 6))]);
    let parents = [(early, goal), (late, goal), (later, goal)];
    assert_eq!(
        sub_issue_delays(&parents, &bars, &dues),
        [(
            goal,
            Warning::Delay(Delay::SubIssue {
                sub_issue: later,
                days: 5,
                due_needed: date(2026, 11, 11),
            })
        )]
    );
    // A goal without a due date the reader may read has no delay.
    assert!(sub_issue_delays(&parents, &bars, &HashMap::new()).is_empty());
}

#[test]
fn plan_13_move_it_after_moves_along_the_dependencies_in_order() {
    let [a, b, c, d, loop_one, loop_two] = ids(6).try_into().unwrap();
    let days = |first, last| Days::new(first, last).unwrap();
    // a: Mon–Thu. b waits for a (Wed–Thu), c waits for b (Fri, already after), d waits for b and c.
    let bars = HashMap::from([
        (a, days(MONDAY, date(2026, 11, 5))),
        (b, days(date(2026, 11, 4), date(2026, 11, 5))),
        (c, days(date(2026, 11, 6), date(2026, 11, 6))),
        (d, days(date(2026, 11, 7), date(2026, 11, 9))),
    ]);
    let graph = DependencyGraph::new(&[(a, b), (b, c), (b, d), (c, d)]);
    let moved = move_after(&graph, &bars, b, date(2026, 11, 5));
    let to: Vec<(IssueId, Days)> = moved.iter().map(|each| (each.issue, each.to)).collect();
    assert_eq!(
        to,
        [
            (b, days(date(2026, 11, 6), date(2026, 11, 7))),
            (c, days(date(2026, 11, 8), date(2026, 11, 8))),
            (d, days(date(2026, 11, 9), date(2026, 11, 11))),
        ]
    );
    assert_eq!(moved[0].by(), 2);
    // Issues on a loop stay where they are; an issue without a bar is not walked.
    let looped = DependencyGraph::new(&[(a, loop_one), (loop_one, loop_two), (loop_two, loop_one)]);
    let mut with_loop = bars.clone();
    with_loop.insert(loop_one, days(MONDAY, MONDAY));
    with_loop.insert(loop_two, days(MONDAY, MONDAY));
    assert!(move_after(&looped, &with_loop, loop_one, MONDAY).is_empty());
    assert!(move_after(&graph, &HashMap::new(), b, MONDAY).is_empty());
}

#[test]
fn plan_13_away_and_too_much_at_once() {
    let days = person_days(MONDAY, 14, 6, 50, &[date(2026, 11, 3), date(2026, 11, 4)]);
    let fixed = Bar {
        days: Days::new(MONDAY, date(2026, 11, 6)).unwrap(),
        worked_out: WorkedOut::Neither,
    };
    assert_eq!(
        away(&fixed, &days),
        Some(Warning::Away {
            days: Days::new(date(2026, 11, 3), date(2026, 11, 4)).unwrap(),
            // Two away days given back after Friday: Monday and Tuesday.
            due_needed: Some(date(2026, 11, 10)),
        })
    );
    // A worked-out bar has its away days as a gap and no warning.
    let worked = Bar {
        worked_out: WorkedOut::Due,
        ..fixed
    };
    assert_eq!(away(&worked, &days), None);
    // Two issues of the person on the same Monday: 3 hours each where 3 hours fit.
    let [first, second] = ids(2).try_into().unwrap();
    let plain = person_days(MONDAY, 14, 6, 50, &[]);
    let one_day = Bar {
        days: Days::one(MONDAY),
        worked_out: WorkedOut::Neither,
    };
    let loads = [
        (first, load(&one_day, hours(3), &plain)),
        (second, load(&one_day, hours(9), &plain)),
    ];
    assert_eq!(loads[1].1, [(MONDAY, 180)]);
    let found = too_much_at_once(&loads, &plain);
    assert_eq!(found.len(), 2);
    assert_eq!(
        found[0].1,
        Warning::TooMuchAtOnce {
            days: Days::one(MONDAY),
            worst: MONDAY,
            planned: 360,
            can: 180,
            after: MONDAY,
        }
    );
    assert!(too_much_at_once(&loads[..1], &plain).is_empty());
}

#[test]
fn plan_13_each_warning_offers_the_ways_out_that_fit_it() {
    use super::warning::{WarningKind, WayOut};
    let [first] = ids(1).try_into().unwrap();
    let days = Days::one(MONDAY);
    let cases: [(Warning, Vec<WayOut>); 5] = [
        (
            Warning::Away {
                days,
                due_needed: None,
            },
            vec![
                WayOut::MoveItAfter,
                WayOut::GiveToSomeoneElse,
                WayOut::KeepAndAccept,
            ],
        ),
        (
            Warning::NotEnoughDays(Shortfall {
                needs: Some(3),
                has: 2,
                due_needed: Some(MONDAY),
            }),
            vec![
                WayOut::MakeItLonger,
                WayOut::GiveToSomeoneElse,
                WayOut::KeepAndAccept,
            ],
        ),
        (
            Warning::Delay(Delay::Dependency {
                first,
                days: 0,
                after: MONDAY,
            }),
            vec![WayOut::MoveItAfter, WayOut::KeepAndAccept],
        ),
        (
            Warning::Delay(Delay::SubIssue {
                sub_issue: first,
                days: 1,
                due_needed: MONDAY,
            }),
            vec![WayOut::MakeItLonger, WayOut::KeepAndAccept],
        ),
        (
            Warning::TooMuchAtOnce {
                days,
                worst: MONDAY,
                planned: 2,
                can: 1,
                after: MONDAY,
            },
            vec![
                WayOut::MoveItAfter,
                WayOut::GiveToSomeoneElse,
                WayOut::KeepAndAccept,
            ],
        ),
    ];
    for (warning, ways) in cases {
        assert_eq!(warning.ways_out(), ways, "{warning:?}");
        // *Move it after* is offered exactly where it knows the day to start after.
        assert_eq!(
            warning.move_after().is_some(),
            ways.contains(&WayOut::MoveItAfter),
            "{warning:?}"
        );
    }
    // A person the reader may not see by name is *the assigned person*.
    let away = Warning::Away {
        days,
        due_needed: None,
    };
    assert_eq!(away.sentence(true), WarningKind::Away.sentence());
    assert_eq!(away.sentence(false).as_str(), "plan-warning.away-hidden");
}
