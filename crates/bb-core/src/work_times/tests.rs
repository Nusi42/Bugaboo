use jiff::Timestamp;
use jiff::civil::Date;

use super::record::AwayTime;
use super::{
    AwayProblem, Halves, Share, UsualDay, UsualDays, WORK_MINUTES_MAX, away_cutoff, minutes_on,
    sum_of_shares,
};
use crate::holidays::Holiday;
use crate::id::{AwayTimeId, UserId};

fn date(year: i16, month: i8, day: i8) -> Date {
    Date::new(year, month, day).unwrap()
}

fn away(first: Date, last: Date, afternoon_only: bool, morning_only: bool) -> AwayTime {
    AwayTime {
        id: AwayTimeId::from_parts(Timestamp::from_second(1_800_000_000).unwrap(), [1; 10])
            .unwrap(),
        person: UserId::from_random([1; 16]),
        first,
        last,
        first_afternoon_only: afternoon_only,
        last_morning_only: morning_only,
        note: None,
        entered_by: UserId::from_random([1; 16]),
        version: 1,
    }
}

#[test]
fn the_usual_work_days_start_on_monday_to_friday_with_8_hours() {
    let usual = UsualDays::start();
    for (at, weekday) in [
        jiff::civil::Weekday::Monday,
        jiff::civil::Weekday::Tuesday,
        jiff::civil::Weekday::Wednesday,
        jiff::civil::Weekday::Thursday,
        jiff::civil::Weekday::Friday,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(usual.minutes_on(weekday), 480, "{at}");
    }
    assert_eq!(usual.minutes_on(jiff::civil::Weekday::Saturday), 0);
    assert_eq!(usual.minutes_on(jiff::civil::Weekday::Sunday), 0);
}

#[test]
fn a_day_that_is_not_ticked_keeps_its_hours_and_counts_as_nothing() {
    let mut days = [UsualDay {
        works: true,
        minutes: 360,
    }; 7];
    days[2].works = false;
    let usual = UsualDays::new(days).unwrap();
    assert_eq!(usual.minutes_on(jiff::civil::Weekday::Wednesday), 0);
    assert_eq!(usual.day(jiff::civil::Weekday::Wednesday).minutes, 360);
    days[3].minutes = WORK_MINUTES_MAX + 1;
    assert!(UsualDays::new(days).is_none());
}

#[test]
fn a_share_is_0_to_100_percent() {
    assert_eq!(Share::new(0).unwrap().percent(), 0);
    assert_eq!(Share::new(100).unwrap().percent(), 100);
    assert!(Share::new(101).is_none());
    assert_eq!(Share::START.percent(), 100);
    let shares = [Share::new(60).unwrap(), Share::new(50).unwrap()];
    assert_eq!(sum_of_shares(shares), 110);
}

#[test]
fn an_away_time_covers_whole_and_half_days_at_its_ends() {
    let time = away(date(2026, 10, 26), date(2026, 10, 30), true, true);
    assert_eq!(time.away_on(date(2026, 10, 25)), Halves::NONE);
    assert_eq!(
        time.away_on(date(2026, 10, 26)),
        Halves {
            morning: false,
            afternoon: true
        }
    );
    assert_eq!(time.away_on(date(2026, 10, 28)), Halves::BOTH);
    assert_eq!(
        time.away_on(date(2026, 10, 30)),
        Halves {
            morning: true,
            afternoon: false
        }
    );
    assert_eq!(time.away_on(date(2026, 10, 31)), Halves::NONE);
}

#[test]
fn an_away_time_of_one_day_takes_one_half_or_the_whole_day() {
    let day = date(2026, 10, 26);
    assert_eq!(away(day, day, false, false).away_on(day), Halves::BOTH);
    assert_eq!(
        away(day, day, true, false).away_on(day),
        Halves {
            morning: false,
            afternoon: true
        }
    );
    assert_eq!(
        away(day, day, false, true).away_on(day),
        Halves {
            morning: true,
            afternoon: false
        }
    );
}

#[test]
fn an_away_time_is_checked_before_it_is_stored() {
    let (first, last) = (date(2026, 10, 26), date(2026, 10, 30));
    assert_eq!(AwayTime::check(first, last, false, false), Ok(()));
    assert_eq!(
        AwayTime::check(last, first, false, false),
        Err(AwayProblem::LastBeforeFirst)
    );
    assert_eq!(
        AwayTime::check(first, first, true, true),
        Err(AwayProblem::NoTimeLeft)
    );
    assert_eq!(
        AwayTime::check(first, date(2028, 1, 1), false, false),
        Err(AwayProblem::TooLong)
    );
}

/// Monday to Thursday 6 hours, Friday 4 hours, the weekend not worked.
fn a_week() -> UsualDays {
    UsualDays::new([
        UsualDay {
            works: true,
            minutes: 360,
        },
        UsualDay {
            works: true,
            minutes: 360,
        },
        UsualDay {
            works: true,
            minutes: 360,
        },
        UsualDay {
            works: true,
            minutes: 360,
        },
        UsualDay {
            works: true,
            minutes: 240,
        },
        UsualDay {
            works: false,
            minutes: 0,
        },
        UsualDay {
            works: false,
            minutes: 0,
        },
    ])
    .unwrap()
}

// The port of the plan reads these minutes, so the table is the contract (ADR-0030, decision 3):
// work hours of the day, times the share, minus away times and holidays.
#[test]
fn plan_15_the_minutes_of_a_day_count_work_hours_share_away_times_and_holidays() {
    let usual = a_week();
    let monday = date(2026, 10, 26);
    let friday = date(2026, 10, 30);
    let saturday = date(2026, 10, 31);
    let half = away(monday, monday, true, false);
    let whole = away(monday, date(2026, 10, 27), false, false);
    let table = [
        // (what, date, share in percent, away times, holiday, minutes)
        (
            "a work day with the full share",
            monday,
            100,
            vec![],
            None,
            360,
        ),
        ("a share of 60 percent", monday, 60, vec![], None, 216),
        (
            "a share of 0 percent counts nothing",
            monday,
            0,
            vec![],
            None,
            0,
        ),
        ("a day with fewer hours", friday, 100, vec![], None, 240),
        (
            "a day that is not a work day",
            saturday,
            100,
            vec![],
            None,
            0,
        ),
        (
            "an away time over the whole day",
            monday,
            100,
            vec![whole.clone()],
            None,
            0,
        ),
        (
            "an away time of the afternoon",
            monday,
            100,
            vec![half.clone()],
            None,
            180,
        ),
        (
            "an away time of the afternoon and a share",
            monday,
            50,
            vec![half.clone()],
            None,
            90,
        ),
        (
            "two away times that cover both halves",
            monday,
            100,
            vec![half, away(monday, monday, false, true)],
            None,
            0,
        ),
        (
            "a public holiday",
            monday,
            100,
            vec![],
            Some(Holiday::UnityDay),
            0,
        ),
        (
            "a public holiday and an away time",
            monday,
            100,
            vec![whole],
            Some(Holiday::UnityDay),
            0,
        ),
    ];
    for (what, day, share, times, holiday, expected) in table {
        let share = Share::new(share).unwrap();
        assert_eq!(
            minutes_on(&usual, share, day, &times, holiday.is_some()),
            expected,
            "{what}"
        );
    }
}

#[test]
fn an_away_time_is_kept_until_one_year_after_its_last_day() {
    // Deleted from the day after the year is over: a last day exactly a year ago is kept.
    let today = date(2027, 11, 12);
    let cutoff = away_cutoff(today).unwrap();
    assert_eq!(cutoff, date(2026, 11, 12));
    assert!(date(2026, 11, 11) < cutoff);
    assert!(date(2026, 11, 12) >= cutoff);
    // A leap day goes back to the end of February.
    assert_eq!(away_cutoff(date(2028, 2, 29)).unwrap(), date(2027, 2, 28));
}
