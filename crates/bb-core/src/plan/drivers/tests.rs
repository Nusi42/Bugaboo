//! Deadline drivers, *Loop*, *Cannot fit* and *Delay* of a release as tables of cases.

use std::collections::HashMap;

use jiff::civil::{Date, date};

use super::{Chain, End, EndOf, loops};
use crate::id::{IssueId, ReleaseId, SequentialIds};
use crate::plan::Days;
use crate::plan::bar::{Bar, WorkedOut};
use crate::plan::order::move_after;
use crate::plan::warning::{Delay, Warning, WayOut, release_delays};
use crate::relation::DependencyGraph;

fn ids(count: usize) -> Vec<IssueId> {
    let mut sequence = SequentialIds::default();
    (0..count)
        .map(|_| IssueId::generate(&mut sequence).unwrap())
        .collect()
}

/// The day `offset` days after Monday, 2 November 2026.
fn day(offset: i64) -> Date {
    date(2026, 11, 2)
        .checked_add(jiff::Span::new().days(offset))
        .unwrap()
}

fn set(first: i64, last: i64) -> Bar {
    Bar {
        days: Days::new(day(first), day(last)).unwrap(),
        worked_out: WorkedOut::Neither,
    }
}

/// Six issues: A (0–2) then B (3–4) then C (5–9), the chain that ends last; D (0–1) then C, with
/// slack; E (0–6) on its own; F without a bar.
fn sample(issues: &[IssueId]) -> (Vec<(IssueId, IssueId)>, HashMap<IssueId, Bar>) {
    let [api, screen, tests, design, alone, _] = issues else {
        panic!("six issues");
    };
    let dependencies = vec![(*api, *screen), (*screen, *tests), (*design, *tests)];
    let bars = HashMap::from([
        (*api, set(0, 2)),
        (*screen, set(3, 4)),
        (*tests, set(5, 9)),
        (*design, set(0, 1)),
        (*alone, set(0, 6)),
    ]);
    (dependencies, bars)
}

#[test]
fn plan_05_the_drivers_are_the_chain_with_no_slack_that_ends_last() {
    let issues = ids(6);
    let (dependencies, bars) = sample(&issues);
    let chain = Chain::new(&dependencies, &bars);
    let members = issues.clone();
    let (drivers, links) = chain.drivers(&members);
    assert_eq!(
        drivers.into_iter().collect::<Vec<_>>(),
        vec![issues[0], issues[1], issues[2]]
    );
    // The chain is the dependencies with no slack; the one with slack is not in it.
    assert_eq!(
        links.into_iter().collect::<Vec<_>>(),
        vec![(issues[0], issues[1]), (issues[1], issues[2])]
    );
    assert_eq!(chain.end(&members), Some(day(9)));
    // A goal over E alone is driven by E alone.
    assert_eq!(
        chain
            .drivers(&[issues[4]])
            .0
            .into_iter()
            .collect::<Vec<_>>(),
        vec![issues[4]]
    );
    // No member with a bar: no end and no drivers.
    assert_eq!(chain.end(&[issues[5]]), None);
    assert!(chain.drivers(&[issues[5]]).0.is_empty());
}

#[test]
fn plan_05_a_delay_of_a_driver_by_a_day_moves_the_end_by_a_day_and_of_any_other_issue_does_not() {
    let issues = ids(6);
    let (dependencies, bars) = sample(&issues);
    let chain = Chain::new(&dependencies, &bars);
    let end = chain.end(&issues).unwrap();
    let (drivers, _) = chain.drivers(&issues);
    for issue in bars.keys() {
        let mut later = bars.clone();
        let bar = later.get_mut(issue).unwrap();
        bar.days = bar.days.shifted(1).unwrap();
        let moved = Chain::new(&dependencies, &later).end(&issues).unwrap();
        let by = Days::between(end, moved);
        assert_eq!(by, i64::from(drivers.contains(issue)), "{issue}");
    }
}

#[test]
fn plan_05_each_end_names_its_drivers_once() {
    let issues = ids(6);
    let (dependencies, bars) = sample(&issues);
    let chain = Chain::new(&dependencies, &bars);
    let goal = issues[5];
    let sets = chain
        .drivers_of(&[
            End {
                of: EndOf::Goal(goal),
                members: vec![issues[1]],
            },
            End {
                of: EndOf::View,
                members: issues.clone(),
            },
        ])
        .ends;
    assert_eq!(
        sets.get(&issues[0]),
        Some(&vec![EndOf::Goal(goal), EndOf::View])
    );
    assert_eq!(sets.get(&issues[2]), Some(&vec![EndOf::View]));
    assert_eq!(sets.get(&issues[3]), None);
}

#[test]
fn plan_05_a_loop_names_each_issue_on_it_with_the_one_it_waits_for() {
    let issues = ids(4);
    let (a, b, c, d) = (issues[0], issues[1], issues[2], issues[3]);
    let found = loops(&[(a, b), (b, c), (c, a), (c, d)]);
    assert_eq!(
        found,
        vec![
            (a, Warning::Loop { waits_for: c }),
            (b, Warning::Loop { waits_for: a }),
            (c, Warning::Loop { waits_for: b }),
        ]
    );
    assert_eq!(
        found[0].1.ways_out(),
        vec![WayOut::OpenTheRelation, WayOut::KeepAndAccept]
    );
    // A loop moves nothing in the chain: its issues keep their days, and what waits for one of
    // them starts after it.
    let bars = HashMap::from([
        (a, set(0, 1)),
        (b, set(0, 1)),
        (c, set(0, 1)),
        (d, set(0, 0)),
    ]);
    let chain = Chain::new(&[(a, b), (b, c), (c, a), (c, d)], &bars);
    assert_eq!(chain.earliest(a), Some(bars[&a].days));
    assert_eq!(chain.earliest(d), Some(Days::new(day(2), day(2)).unwrap()));
}

#[test]
fn plan_05_a_due_date_before_what_it_waits_for_can_end_cannot_fit() {
    let issues = ids(4);
    let (a, b, c, d) = (issues[0], issues[1], issues[2], issues[3]);
    // A (0–4) then B (2–3), which is due before A ends; B then C (6–6), due on the day B can end
    // at the earliest (5–6), which counts as too early too. D (0–8) waits for nothing.
    let bars = HashMap::from([
        (a, set(0, 4)),
        (b, set(2, 3)),
        (c, set(6, 6)),
        (d, set(0, 8)),
    ]);
    let dues = HashMap::from([(b, day(3)), (c, day(6)), (d, day(8))]);
    let chain = Chain::new(&[(a, b), (b, c)], &bars);
    let found = chain.cannot_fit(&bars, &dues);
    assert_eq!(
        found,
        vec![
            (
                b,
                Warning::CannotFit {
                    first: a,
                    ends: day(4),
                    due_needed: day(6),
                }
            ),
            (
                c,
                Warning::CannotFit {
                    first: b,
                    ends: day(6),
                    due_needed: day(7),
                }
            ),
        ]
    );
    assert_eq!(found[0].1.due_needed(), Some(day(6)));
    assert_eq!(
        found[0].1.ways_out(),
        vec![WayOut::MakeItLonger, WayOut::KeepAndAccept]
    );
    // A due date a day after what it waits for ends fits.
    let later = HashMap::from([(c, day(7))]);
    assert!(chain.cannot_fit(&bars, &later).is_empty());
}

#[test]
fn plan_05_an_issue_without_a_due_date_never_cannot_fit() {
    let issues = ids(2);
    let (a, b) = (issues[0], issues[1]);
    // B has only a start date, inside A: a bar of one day people did not give a due date. A due
    // date *Plan by effort* worked out is no due date people set either.
    let bars = HashMap::from([(a, set(0, 4)), (b, set(2, 2))]);
    let chain = Chain::new(&[(a, b)], &bars);
    let found = chain.cannot_fit(&bars, &HashMap::new());
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn plan_13_issues_on_a_loop_stay_and_what_waits_for_them_waits_where_they_stand() {
    let issues = ids(4);
    let (a, b, c, d) = (issues[0], issues[1], issues[2], issues[3]);
    // A then B and B then A: a loop. B then C. D then A, from outside the loop.
    let bars = HashMap::from([
        (a, set(2, 3).days),
        (b, set(0, 5).days),
        (c, set(1, 1).days),
        (d, set(0, 4).days),
    ]);
    let graph = DependencyGraph::new(&[(a, b), (b, a), (b, c), (d, a)]);
    let schedule = graph.schedule(&bars, None, |_| true);
    // The issues on the loop keep their days, also A, which waits for D outside it.
    assert_eq!(schedule.days(a), Some(bars[&a]));
    assert_eq!(schedule.days(b), Some(bars[&b]));
    assert!(schedule.on_loop(a) && schedule.on_loop(b) && !schedule.on_loop(c));
    // C waits for B where B stands: the day after B ends.
    assert_eq!(schedule.days(c), Some(Days::one(day(6))));
    // *Move it after* follows the same rule: the loop stays, C moves.
    let moved = move_after(&graph, &bars, a, day(10));
    assert_eq!(
        moved.iter().map(|each| each.issue).collect::<Vec<_>>(),
        vec![c]
    );
}

#[test]
fn iss_08_an_issue_that_ends_after_its_release_date_has_a_delay() {
    let issues = ids(3);
    let mut sequence = SequentialIds::default();
    let release = ReleaseId::generate(&mut sequence).unwrap();
    let bars = HashMap::from([(issues[0], set(0, 6)), (issues[1], set(0, 4))]);
    let dates = HashMap::from([(release, day(4))]);
    let found = release_delays(
        &[
            (issues[0], release),
            (issues[1], release),
            (issues[2], release),
        ],
        &bars,
        &dates,
    );
    let delay = Warning::Delay(Delay::Release {
        release,
        days: 2,
        due_needed: day(6),
    });
    assert_eq!(found, vec![(issues[0], delay)]);
    assert_eq!(delay.sentence().as_str(), "plan-warning.delay-release");
    assert_eq!(
        delay.ways_out(),
        vec![WayOut::MakeItLonger, WayOut::KeepAndAccept]
    );
}
