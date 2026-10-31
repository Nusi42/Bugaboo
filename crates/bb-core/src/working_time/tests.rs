//! The working-time rules of the preset *Germany*, one test for each rule, and the property tests of
//! the person's day: across midnight, across the changes of daylight-saving time, in a zone east of
//! UTC+12 and for a person who changed zone between two days (TEST-WORKING-TIME).

use jiff::civil::date;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use proptest::prelude::*;

use super::*;
use crate::settings::working_hours::{WORKING_HOURS_KEEP_YEARS, WORKING_HOURS_PRESET};
use crate::settings::{Layers, OrganizationValue, SettingValue};
use crate::time::Minutes;
use crate::time::day::{DaySpan, date_of};
use crate::time::zone;

/// The zones of the property tests: two with summer time on different days, one east of UTC+12
/// (+14, where a day starts before it starts anywhere else), and UTC.
const ZONES: [&str; 4] = [
    "Europe/Berlin",
    "America/New_York",
    "Pacific/Kiritimati",
    "UTC",
];

fn at(text: &str) -> Timestamp {
    text.parse().unwrap_or(Timestamp::UNIX_EPOCH)
}

fn zone_named(name: &str) -> TimeZone {
    zone::named(name).unwrap_or(TimeZone::UTC)
}

const fn minutes(count: u32) -> Minutes {
    Minutes::new(count)
}

/// The rules of the preset *Germany*: the defaults of the registry.
fn germany() -> Rules {
    Rules::of(&Layers::default()).unwrap_or(Rules {
        day_most: minutes(0),
        first_break: BreakRule {
            after: minutes(0),
            needed: minutes(0),
        },
        second_break: BreakRule {
            after: minutes(0),
            needed: minutes(0),
        },
        shortest_break: minutes(0),
        in_a_row_most: minutes(0),
        rest_least: minutes(0),
    })
}

/// A day of one period from `start` lasting `length`, with breaks of these minutes after these
/// minutes of the day.
fn day(start: Timestamp, length: i64, breaks: &[(i64, i64)]) -> Times {
    let after = |offset: i64| start + SignedDuration::from_mins(offset);
    Times {
        periods: vec![Span::closed(start, after(length))],
        breaks: breaks
            .iter()
            .map(|(from, minutes)| Span::closed(after(*from), after(from + minutes)))
            .collect(),
    }
}

fn layers_with(values: &[(&str, SettingValue)]) -> Layers {
    let mut layers = Layers::default();
    for (key, value) in values {
        layers.organization.insert(
            (*key).to_owned(),
            OrganizationValue {
                value: value.clone(),
                locked: false,
            },
        );
    }
    layers
}

#[test]
fn time_09_the_preset_germany_holds_the_numbers_of_the_law() {
    let rules = germany();
    assert_eq!(rules.day_most, minutes(600));
    assert_eq!(
        rules.first_break,
        BreakRule {
            after: minutes(360),
            needed: minutes(30)
        }
    );
    assert_eq!(
        rules.second_break,
        BreakRule {
            after: minutes(540),
            needed: minutes(45)
        }
    );
    assert_eq!(rules.shortest_break, minutes(15));
    assert_eq!(rules.in_a_row_most, minutes(360));
    assert_eq!(rules.rest_least, minutes(660));
}

#[test]
fn time_09_ten_hours_and_a_minute_warns_and_is_still_recorded() {
    let rules = germany();
    let start = at("2026-10-05T06:00:00Z");
    // 10 hours of work and a minute, with 45 minutes of break in two parts.
    let long = day(start, 601 + 45, &[(180, 30), (420, 15)]);
    let found = warnings(&long, None, Some(&rules));
    assert_eq!(
        found,
        vec![Warning::LongDay {
            worked: minutes(601),
            most: minutes(600)
        }]
    );
    // The day is counted all the same: the totals show what really happened.
    assert_eq!(Totals::of(&long, start).worked, minutes(601));
    // Ten hours exactly do not warn.
    let ten = day(start, 600 + 45, &[(180, 30), (420, 15)]);
    assert_eq!(warnings(&ten, None, Some(&rules)), Vec::new());
    // The sentence names both lengths in words.
    assert_eq!(
        found[0].said().render(),
        "10 hours 1 minute worked. The rules allow 10 hours a day at most."
    );
}

#[test]
fn time_09_breaks_under_fifteen_minutes_do_not_count() {
    let rules = germany();
    let start = at("2026-10-05T07:00:00Z");
    // Seven hours of work with three breaks of 10 minutes: 30 minutes of break, none of which
    // counts, so the break is short and the work runs on in a row.
    let short = day(start, 420 + 30, &[(120, 10), (240, 10), (330, 10)]);
    let found = warnings(&short, None, Some(&rules));
    assert!(found.contains(&Warning::ShortBreak {
        worked: minutes(420),
        breaks: minutes(0),
        needed: minutes(30),
        after: minutes(360),
    }));
    assert!(found.contains(&Warning::LongStretch {
        stretch: minutes(450),
        most: minutes(360),
        shortest: minutes(15),
    }));
    // The totals count every minute of break.
    assert_eq!(Totals::of(&short, start).breaks, minutes(30));
    // Two breaks of 15 minutes count.
    let counted = day(start, 420 + 30, &[(150, 15), (300, 15)]);
    assert_eq!(warnings(&counted, None, Some(&rules)), Vec::new());
}

#[test]
fn time_09_nine_hours_and_a_minute_ask_for_forty_five_minutes_of_break() {
    let rules = germany();
    let start = at("2026-10-05T07:00:00Z");
    let found = warnings(&day(start, 541 + 30, &[(240, 30)]), None, Some(&rules));
    assert_eq!(
        found,
        vec![Warning::ShortBreak {
            worked: minutes(541),
            breaks: minutes(30),
            needed: minutes(45),
            after: minutes(540),
        }]
    );
    // Six hours exactly need no break.
    assert_eq!(
        warnings(&day(start, 360, &[]), None, Some(&rules)),
        Vec::new()
    );
}

#[test]
fn time_09_six_hours_in_a_row_without_a_break_warns() {
    let rules = germany();
    let start = at("2026-10-05T07:00:00Z");
    // Six hours and a minute, then a break of 30 minutes, then an hour: the break is enough, the
    // stretch before it is not.
    let found = warnings(&day(start, 361 + 30 + 60, &[(361, 30)]), None, Some(&rules));
    assert_eq!(
        found,
        vec![Warning::LongStretch {
            stretch: minutes(361),
            most: minutes(360),
            shortest: minutes(15),
        }]
    );
    // The gap between two periods is a break as well.
    let two_periods = Times {
        periods: vec![
            Span::closed(start, at("2026-10-05T12:00:00Z")),
            Span::closed(at("2026-10-05T12:30:00Z"), at("2026-10-05T15:30:00Z")),
        ],
        breaks: Vec::new(),
    };
    assert_eq!(warnings(&two_periods, None, Some(&rules)), Vec::new());
    assert_eq!(
        Totals::of(&two_periods, start),
        Totals {
            worked: minutes(480),
            breaks: minutes(30)
        }
    );
}

#[test]
fn time_09_eleven_hours_of_rest_are_checked_against_the_day_before() {
    let rules = germany();
    let before = day(at("2026-10-05T12:00:00Z"), 480 + 30, &[(240, 30)]);
    // The day before ended at 20:30; the next starts at 07:00: 10 hours 30 minutes of rest.
    let next = day(at("2026-10-06T07:00:00Z"), 240, &[]);
    assert_eq!(
        warnings(&next, Some(&before), Some(&rules)),
        vec![Warning::ShortRest {
            rest: minutes(630),
            needed: minutes(660)
        }]
    );
    // Eleven hours are enough.
    let later = day(at("2026-10-06T07:30:00Z"), 240, &[]);
    assert_eq!(warnings(&later, Some(&before), Some(&rules)), Vec::new());
    // A day before that was never ended gives no rest to check.
    let open = Times {
        periods: vec![Span::open(at("2026-10-05T12:00:00Z"))],
        breaks: Vec::new(),
    };
    assert_eq!(warnings(&next, Some(&open), Some(&rules)), Vec::new());
}

#[test]
fn time_09_no_rules_gives_no_warning() {
    let layers = layers_with(&[(
        WORKING_HOURS_PRESET.definition().key(),
        SettingValue::Text(Preset::NoRules.code().to_owned()),
    )]);
    assert_eq!(Rules::of(&layers), None);
    let start = at("2026-10-05T03:00:00Z");
    let endless = day(start, 900, &[]);
    let before = day(at("2026-10-04T12:00:00Z"), 600, &[]);
    assert_eq!(
        warnings(&endless, Some(&before), Rules::of(&layers).as_ref()),
        Vec::new()
    );
    // The long-timer warning still has a longest day: 10 hours.
    assert_eq!(longest_day(&layers), NO_RULES_LONGEST_DAY);
    assert_eq!(longest_day(&Layers::default()), minutes(600));
}

#[test]
fn time_09_what_runs_is_not_checked_until_it_ends() {
    let rules = germany();
    let running = Times {
        periods: vec![Span::open(at("2026-10-05T03:00:00Z"))],
        breaks: Vec::new(),
    };
    assert_eq!(warnings(&running, None, Some(&rules)), Vec::new());
    // The totals count it until now.
    assert_eq!(
        Totals::of(&running, at("2026-10-05T04:30:00Z")).worked,
        minutes(90)
    );
    // A break that runs is a break until now.
    let on_break = Times {
        periods: vec![Span::open(at("2026-10-05T03:00:00Z"))],
        breaks: vec![Span::open(at("2026-10-05T04:00:00Z"))],
    };
    assert_eq!(
        Totals::of(&on_break, at("2026-10-05T04:20:00Z")),
        Totals {
            worked: minutes(60),
            breaks: minutes(20)
        }
    );
}

#[test]
fn time_10_the_keep_period_is_never_below_two_years_with_the_preset_germany() {
    assert!(!keep_years_allowed(Preset::Germany, 1));
    assert!(keep_years_allowed(Preset::Germany, 2));
    assert!(keep_years_allowed(Preset::NoRules, 1));
    let one = (
        WORKING_HOURS_KEEP_YEARS.definition().key(),
        SettingValue::Integer(1),
    );
    assert_eq!(keep_years(&layers_with(std::slice::from_ref(&one))), 2);
    let no_rules = (
        WORKING_HOURS_PRESET.definition().key(),
        SettingValue::Text(Preset::NoRules.code().to_owned()),
    );
    assert_eq!(keep_years(&layers_with(&[one, no_rules])), 1);
    assert_eq!(keep_years(&Layers::default()), GERMANY_KEEP_YEARS_LEAST);
}

#[test]
fn time_10_a_day_is_past_from_the_next_day_on_in_its_zone() {
    let berlin = zone_named("Europe/Berlin");
    let monday = date(2026, 10, 5);
    // 23:30 in Berlin on Monday is still Monday; 00:30 on Tuesday is not.
    assert!(!is_past(monday, &berlin, at("2026-10-05T21:30:00Z")));
    assert!(is_past(monday, &berlin, at("2026-10-05T22:30:00Z")));
    assert!(!is_past(monday, &TimeZone::UTC, at("2026-10-05T22:30:00Z")));
}

#[test]
fn time_08_the_totals_of_a_week_are_the_sum_of_its_days() {
    let start = at("2026-10-05T07:00:00Z");
    let days = [
        day(start, 480 + 30, &[(240, 30)]),
        day(start + SignedDuration::from_hours(24), 300, &[]),
    ];
    let week: Totals = days.iter().map(|times| Totals::of(times, start)).sum();
    assert_eq!(
        week,
        Totals {
            worked: minutes(780),
            breaks: minutes(30)
        }
    );
}

/// A start somewhere in 2026, to the minute.
fn any_start() -> impl Strategy<Value = Timestamp> {
    (0_i64..365 * 24 * 60)
        .prop_map(|offset| at("2026-01-01T00:00:00Z") + SignedDuration::from_mins(offset))
}

proptest! {
    #[test]
    fn time_08_a_day_across_midnight_belongs_to_the_date_it_starts_on_and_counts_every_minute(
        zone in prop::sample::select(ZONES.as_slice()),
        start in any_start(),
        length in 1_i64..24 * 60,
    ) {
        let zone = zone_named(zone);
        let times = day(start, length, &[]);
        let total = Totals::of(&times, start);
        prop_assert_eq!(i64::from(total.worked.get()), length);
        let date = date_of(start, &zone);
        let span = DaySpan::of(date, &zone);
        prop_assert!(span.is_some_and(|span| span.holds(start)));
    }

    #[test]
    fn time_08_a_day_across_the_daylight_saving_changes_counts_the_time_that_passed(
        zone in prop::sample::select(["Europe/Berlin", "America/New_York"].as_slice()),
        start_hour in 0_i64..6,
        length in 60_i64..12 * 60,
    ) {
        // The nights the clocks go forward and back in 2026.
        for night in ["2026-03-29T00:00:00Z", "2026-10-25T00:00:00Z", "2026-03-08T04:00:00Z",
            "2026-11-01T04:00:00Z"] {
            let start = at(night) + SignedDuration::from_hours(start_hour);
            let times = day(start, length, &[]);
            prop_assert_eq!(i64::from(Totals::of(&times, start).worked.get()), length);
            // The zone moves the date, never the length.
            let local = date_of(start, &zone_named(zone));
            prop_assert!(DaySpan::of(local, &zone_named(zone)).is_some_and(|span| span.holds(start)));
        }
    }

    #[test]
    fn time_08_a_zone_east_of_utc_plus_twelve_starts_its_day_first(
        start in any_start(),
    ) {
        let kiritimati = zone_named("Pacific/Kiritimati");
        // +14: the local date is never before the date in UTC, and at most one day after it.
        let local = date_of(start, &kiritimati);
        let utc = date_of(start, &TimeZone::UTC);
        prop_assert!(local >= utc);
        prop_assert!(local.since(utc).is_ok_and(|span| span.get_days() <= 1));
        // And the rules count the same moments wherever the person is.
        let times = day(start, 601, &[]);
        let rules = germany();
        prop_assert_eq!(
            warnings(&times, None, Some(&rules)).first().copied(),
            Some(Warning::LongDay { worked: minutes(601), most: minutes(600) })
        );
    }

    #[test]
    fn time_09_a_person_who_changed_zone_between_two_days_has_the_rest_of_the_moments(
        from in prop::sample::select(ZONES.as_slice()),
        to in prop::sample::select(ZONES.as_slice()),
        start in any_start(),
        rest in 0_i64..24 * 60,
    ) {
        let before = day(start, 480, &[(240, 30)]);
        let next_start = start + SignedDuration::from_mins(480 + rest);
        let next = day(next_start, 240, &[]);
        let rules = germany();
        let found = warnings(&next, Some(&before), Some(&rules));
        let short = found.iter().any(|warning| matches!(warning, Warning::ShortRest { .. }));
        prop_assert_eq!(short, rest < 660);
        // The dates of the two days are each in their own zone, and never the other way round.
        let first = date_of(start, &zone_named(from));
        let second = date_of(next_start, &zone_named(to));
        prop_assert!(second >= first.yesterday().unwrap_or(first));
    }
}
