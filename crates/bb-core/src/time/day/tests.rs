use jiff::civil::date;
use jiff::{SignedDuration, Timestamp};
use proptest::prelude::*;

use super::*;
use crate::time::zone;

/// The zones the person's day is tested in: two with summer time on different days, one east of
/// UTC+12 (+14, where a day starts before it starts anywhere else), and UTC.
const ZONES: [&str; 4] = [
    "Europe/Berlin",
    "America/New_York",
    "Pacific/Kiritimati",
    "UTC",
];

fn zone_named(name: &str) -> TimeZone {
    zone::named(name).unwrap_or(TimeZone::UTC)
}

fn at(text: &str) -> Timestamp {
    text.parse().unwrap_or(Timestamp::UNIX_EPOCH)
}

fn hours_of(span: Option<DaySpan>) -> Option<i64> {
    span.map(|span| span.end.duration_since(span.start).as_hours())
}

#[test]
fn time_08_a_day_has_23_or_25_hours_on_the_days_the_clocks_change() {
    let berlin = zone_named("Europe/Berlin");
    let new_york = zone_named("America/New_York");
    assert_eq!(hours_of(DaySpan::of(date(2026, 3, 29), &berlin)), Some(23));
    assert_eq!(hours_of(DaySpan::of(date(2026, 10, 25), &berlin)), Some(25));
    assert_eq!(hours_of(DaySpan::of(date(2026, 10, 26), &berlin)), Some(24));
    assert_eq!(hours_of(DaySpan::of(date(2026, 3, 8), &new_york)), Some(23));
    assert_eq!(
        hours_of(DaySpan::of(date(2026, 11, 1), &new_york)),
        Some(25)
    );
    // The day starts at midnight of the zone: 22:00 UTC in a Berlin summer.
    let summer = DaySpan::of(date(2026, 10, 5), &berlin);
    assert_eq!(
        summer.map(|span| span.start),
        Some(at("2026-10-04T22:00:00Z"))
    );
}

#[test]
fn time_08_a_moment_exactly_at_midnight_belongs_to_the_day_it_starts() {
    let berlin = zone_named("Europe/Berlin");
    assert_eq!(
        date_of(at("2026-10-05T22:00:00Z"), &berlin),
        date(2026, 10, 6)
    );
    let just_before = at("2026-10-05T22:00:00Z") - SignedDuration::from_nanos(1);
    assert_eq!(date_of(just_before, &berlin), date(2026, 10, 5));
    assert_eq!(
        date_of(at("2026-10-05T00:00:00Z"), &TimeZone::UTC),
        date(2026, 10, 5)
    );
}

#[test]
fn time_08_east_of_utc_plus_12_the_date_is_already_the_next() {
    let kiritimati = zone_named("Pacific/Kiritimati");
    assert_eq!(
        date_of(at("2026-10-05T10:30:00Z"), &kiritimati),
        date(2026, 10, 6)
    );
    assert_eq!(
        date_of(at("2026-10-05T09:59:00Z"), &kiritimati),
        date(2026, 10, 5)
    );
}

#[test]
fn time_08_the_29th_of_february_is_a_day_of_its_own() {
    let berlin = zone_named("Europe/Berlin");
    assert_eq!(
        date_of(at("2028-02-29T12:00:00Z"), &berlin),
        date(2028, 2, 29)
    );
    assert_eq!(hours_of(DaySpan::of(date(2028, 2, 29), &berlin)), Some(24));
    let week = Week::of(date(2028, 2, 29), FirstDay::Monday).map(|week| week.days());
    assert_eq!(
        week,
        Some(vec![
            date(2028, 2, 28),
            date(2028, 2, 29),
            date(2028, 3, 1),
            date(2028, 3, 2),
            date(2028, 3, 3),
            date(2028, 3, 4),
            date(2028, 3, 5),
        ])
    );
}

#[test]
fn time_08_the_week_starts_on_the_first_day_the_person_chose() {
    // Wednesday, 2026-10-07.
    let wednesday = date(2026, 10, 7);
    let first = |day| Week::of(wednesday, day).map(|week| week.first());
    assert_eq!(first(FirstDay::Monday), Some(date(2026, 10, 5)));
    assert_eq!(first(FirstDay::Sunday), Some(date(2026, 10, 4)));
    assert_eq!(first(FirstDay::Saturday), Some(date(2026, 10, 3)));
    // A week of the first day itself starts on it.
    assert_eq!(
        Week::of(date(2026, 10, 4), FirstDay::Sunday).map(|week| week.first()),
        Some(date(2026, 10, 4))
    );
    assert_eq!(FirstDay::of_choice(Some("device")), FirstDay::Monday);
    assert_eq!(FirstDay::of_choice(Some("saturday")), FirstDay::Saturday);
    assert_eq!(FirstDay::of_choice(None), FirstDay::Monday);
}

/// Seconds from 2020 to 2035: a stretch with many changes of the clocks in both zones.
const FROM: i64 = 1_577_836_800;
const UNTIL: i64 = 2_051_222_400;

/// The seconds around a change of the clocks that the property looks at: two days either side.
const AROUND: i64 = 2 * crate::time::SECONDS_PER_DAY;

/// A moment `offset` seconds from the `nth` change of the clocks in `zone` after 2020; a zone
/// without changes (UTC) gives a moment from 2020 on.
fn near_a_change(zone: &TimeZone, nth: usize, offset: i64) -> Timestamp {
    let from = Timestamp::from_second(FROM).unwrap_or(Timestamp::UNIX_EPOCH);
    let change = zone
        .following(from)
        .nth(nth)
        .map_or(from, |transition| transition.timestamp());
    Timestamp::from_second(change.as_second() + offset).unwrap_or(from)
}

/// The day of a moment, checked: the date a clock in the zone shows, inside the moments of that
/// date, a date of 23, 24 or 25 hours that ends where the next begins.
fn check_the_day(zone: &TimeZone, moment: Timestamp) -> Result<(), TestCaseError> {
    let day = date_of(moment, zone);
    prop_assert_eq!(day, moment.to_zoned(zone.clone()).date());
    let span = DaySpan::of(day, zone);
    prop_assert!(span.is_some_and(|span| span.holds(moment)));
    prop_assert!(matches!(hours_of(span), Some(23..=25)));
    let next = day.tomorrow().ok().and_then(|next| DaySpan::of(next, zone));
    prop_assert_eq!(span.map(|span| span.end), next.map(|next| next.start));
    if let Some(span) = span {
        prop_assert_eq!(date_of(span.start, zone), day);
        let before = span.start - SignedDuration::from_nanos(1);
        prop_assert_eq!(date_of(before, zone).tomorrow().ok(), Some(day));
    }
    Ok(())
}

proptest! {
    #[test]
    fn time_08_the_day_of_a_moment_is_the_date_in_the_persons_zone_across_daylight_saving_changes(
        nth in 0usize..30,
        offset in -AROUND..AROUND,
        second in FROM..UNTIL,
        which in 0..ZONES.len(),
    ) {
        let zone = zone_named(ZONES[which]);
        // Near a change of the clocks, where a day has 23 or 25 hours, and anywhere in 15 years.
        check_the_day(&zone, near_a_change(&zone, nth, offset))?;
        let moment = Timestamp::from_second(second).unwrap_or(Timestamp::UNIX_EPOCH);
        check_the_day(&zone, moment)?;
    }

    #[test]
    fn time_08_a_week_holds_its_date_and_seven_days_from_the_first_day(
        days in 0i64..20_000,
        which in 0usize..3,
    ) {
        let first_day = [FirstDay::Monday, FirstDay::Sunday, FirstDay::Saturday][which];
        let day = date(1990, 1, 1).checked_add(Span::new().days(days)).unwrap_or(date(1990, 1, 1));
        let week = Week::of(day, first_day);
        prop_assert!(week.is_some());
        if let Some(week) = week {
            let all = week.days();
            prop_assert_eq!(all.len(), DAYS_PER_WEEK);
            prop_assert!(all.contains(&day));
            prop_assert_eq!(week.first().weekday(), first_day.weekday());
            for pair in all.windows(2) {
                prop_assert_eq!(pair[0].tomorrow().ok(), Some(pair[1]));
            }
        }
    }
}
