use jiff::civil::Date;

use super::{Holiday, Region};
use crate::message::Message;

/// The official lists of 2026 to 2030: test data written out by hand (see the file).
const OFFICIAL: &str = include_str!("official_2026_2030.txt");

fn official() -> impl Iterator<Item = (Region, i16, Vec<Date>)> {
    OFFICIAL
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| {
            let mut words = line.split_whitespace();
            let region = Region::from_code(words.next().unwrap()).unwrap();
            let year: i16 = words.next().unwrap().parse().unwrap();
            let days = words
                .map(|day| {
                    let (month, day) = day.split_once('-').unwrap();
                    Date::new(year, month.parse().unwrap(), day.parse().unwrap()).unwrap()
                })
                .collect();
            (region, year, days)
        })
}

#[test]
fn plan_11_german_holidays_match_the_official_lists_2026_to_2030() {
    let mut lines = 0;
    for (region, year, days) in official() {
        let worked_out: Vec<Date> = region
            .holidays_in(year)
            .iter()
            .map(|holiday| holiday.date)
            .collect();
        assert_eq!(worked_out, days, "{} {year}", region.code());
        lines += 1;
    }
    // Germany and the 16 states, for each of the five years.
    assert_eq!(lines, 17 * 5);
    assert_eq!(Region::ALL.len(), 18);
}

#[test]
fn a_holiday_is_found_by_its_date_and_between_two_dates() {
    let ascension = Date::new(2026, 5, 14).unwrap();
    assert_eq!(
        Region::Bavaria.holiday_on(ascension),
        Some(Holiday::AscensionDay)
    );
    assert_eq!(Region::None.holiday_on(ascension), None);
    assert_eq!(
        Region::Germany.holiday_on(Date::new(2026, 6, 4).unwrap()),
        None
    );
    assert_eq!(
        Region::Hesse.holiday_on(Date::new(2026, 6, 4).unwrap()),
        Some(Holiday::CorpusChristi)
    );
    // A stretch over the turn of the year has the days of both years, in order.
    let days = Region::Germany.holidays_between(
        Date::new(2026, 12, 24).unwrap(),
        Date::new(2027, 1, 1).unwrap(),
    );
    let found: Vec<Date> = days.iter().map(|day| day.date).collect();
    assert_eq!(
        found,
        [
            Date::new(2026, 12, 25).unwrap(),
            Date::new(2026, 12, 26).unwrap(),
            Date::new(2027, 1, 1).unwrap(),
        ]
    );
    assert!(
        Region::Germany
            .holidays_between(
                Date::new(2027, 1, 1).unwrap(),
                Date::new(2026, 1, 1).unwrap()
            )
            .is_empty()
    );
}

#[test]
fn every_region_and_holiday_has_its_own_name_in_the_catalog() {
    for region in Region::ALL {
        assert_eq!(
            region.name().as_str(),
            format!("holidays.region.{}", region.code())
        );
        assert!(!Message::from(region.name()).render().is_empty());
    }
    for holiday in Holiday::ALL {
        assert_eq!(
            holiday.name().as_str(),
            format!("holiday.{}", holiday.code())
        );
        assert!(!Message::from(holiday.name()).render().is_empty());
    }
}
