use jiff::civil::date;
use serde_json::json;

use super::*;
use crate::settings::{OrganizationValue, SettingValue, TIME_OF_DAY_24_HOUR};

fn layers(person: &[(&str, &str)], organization: &[(&str, &str)]) -> Layers {
    let mut layers = Layers::default();
    for (key, value) in person {
        layers
            .person
            .insert((*key).to_owned(), SettingValue::Text((*value).to_owned()));
    }
    for (key, value) in organization {
        layers.organization.insert(
            (*key).to_owned(),
            OrganizationValue {
                value: SettingValue::Text((*value).to_owned()),
                locked: false,
            },
        );
    }
    layers
}

const MOMENT: &str = "2026-10-05T07:30:00Z";

fn moment_in(layers: &Layers) -> Option<String> {
    Region::of(layers).worded_for_mail(FieldType::DateTime, &json!(MOMENT))
}

fn usual() -> Region {
    Region::from_choices(&Choices::default())
}

#[test]
fn m4_f140_a_moment_is_in_the_zone_of_the_person_and_names_it() {
    let own = layers(
        &[
            ("region.time-zone", "Europe/Berlin"),
            ("region.date-format", "day-month-year"),
        ],
        &[],
    );
    assert_eq!(
        moment_in(&own).as_deref(),
        Some("05.10.2026, 09:30 (Europe/Berlin)")
    );
}

#[test]
fn m4_f140_a_person_on_device_gets_the_zone_of_the_organization() {
    let on_device = layers(
        &[("region.time-zone", "device")],
        &[("region.time-zone", "Europe/Berlin")],
    );
    assert_eq!(
        moment_in(&on_device).as_deref(),
        Some("2026-10-05, 09:30 (Europe/Berlin)")
    );
    // Nothing chosen by the person gives the same.
    let nothing = layers(&[], &[("region.time-zone", "Europe/Berlin")]);
    assert_eq!(moment_in(&nothing), moment_in(&on_device));
}

#[test]
fn m4_f140_where_nobody_chose_a_zone_it_is_utc() {
    assert_eq!(
        moment_in(&layers(&[], &[])).as_deref(),
        Some("2026-10-05, 07:30 (UTC)")
    );
    // A name the server does not know counts as none.
    assert_eq!(
        moment_in(&layers(&[("region.time-zone", "Mars/Olympus")], &[])).as_deref(),
        Some("2026-10-05, 07:30 (UTC)")
    );
}

#[test]
fn m4_f140_the_zone_data_follows_summer_time() {
    let berlin = Region::from_choices(&Choices {
        time_zone: Some("Europe/Berlin"),
        ..Choices::default()
    });
    let winter = berlin.worded(FieldType::DateTime, &json!("2026-12-05T07:30:00Z"));
    assert_eq!(winter.as_deref(), Some("2026-12-05, 08:30"));
}

#[test]
fn cfg_04_the_time_of_day_follows_the_region_and_its_default_follows_the_date_format() {
    let at = |date_format, time_of_day| {
        let region = Region::from_choices(&Choices {
            date_format,
            time_of_day,
            ..Choices::default()
        });
        region.time_of_day(MOMENT.parse().unwrap_or(Timestamp::UNIX_EPOCH))
    };
    // Every fixed date format starts with the clock of 24 hours.
    for date_format in ["day-month-year", "month-day-year", "year-month-day"] {
        assert_eq!(at(Some(date_format), None), "07:30", "{date_format}");
        assert_eq!(at(Some(date_format), Some("device")), "07:30");
        assert_eq!(at(Some(date_format), Some(TIME_OF_DAY_24_HOUR)), "07:30");
        assert_eq!(at(Some(date_format), Some(TIME_OF_DAY_12_HOUR)), "7:30 AM");
    }
    // A server does not know the device, so *device* on both is the fixed format of a server.
    assert_eq!(at(None, None), "07:30");
    // The organization's clock stands for a person who left it on *device*.
    let twelve = layers(&[], &[("region.time-of-day", "12-hour")]);
    assert_eq!(
        moment_in(&twelve).as_deref(),
        Some("2026-10-05, 7:30 AM (UTC)")
    );
}

#[test]
fn a_number_is_rounded_half_up_on_its_digits() {
    let said = |number: f64| usual().worded(FieldType::Decimal, &json!(number));
    assert_eq!(said(0.0625).as_deref(), Some("0.063"));
    assert_eq!(said(0.9996).as_deref(), Some("1"));
    assert_eq!(said(-0.0625).as_deref(), Some("-0.063"));
}

#[test]
fn the_hours_of_a_moment_are_on_the_clock_the_person_chose() {
    let twelve = Region::from_choices(&Choices {
        time_of_day: Some(TIME_OF_DAY_12_HOUR),
        ..Choices::default()
    });
    let at = |region: &Region, moment: &str| region.worded(FieldType::DateTime, &json!(moment));
    assert_eq!(
        at(&twelve, "2026-10-05T00:00:00Z").as_deref(),
        Some("2026-10-05, 12:00 AM")
    );
    assert_eq!(
        at(&twelve, "2026-10-05T12:05:00Z").as_deref(),
        Some("2026-10-05, 12:05 PM")
    );
    assert_eq!(
        at(&twelve, "2026-10-05T13:05:00Z").as_deref(),
        Some("2026-10-05, 1:05 PM")
    );
    assert_eq!(
        at(&usual(), "2026-10-05T00:00:00Z").as_deref(),
        Some("2026-10-05, 00:00")
    );
    assert_eq!(
        at(&usual(), "2026-10-05T13:05:00Z").as_deref(),
        Some("2026-10-05, 13:05")
    );
}

#[test]
fn the_two_fixed_orders_of_a_day_are_written_with_their_marks() {
    let day = |date_format| {
        Region::from_choices(&Choices {
            date_format: Some(date_format),
            ..Choices::default()
        })
        .worded(FieldType::Date, &json!("2026-10-05"))
    };
    assert_eq!(
        day(DATE_FORMAT_DAY_MONTH_YEAR).as_deref(),
        Some("05.10.2026")
    );
    assert_eq!(
        day(DATE_FORMAT_MONTH_DAY_YEAR).as_deref(),
        Some("10/05/2026")
    );
}

#[test]
fn time_08_the_week_and_the_decimal_mark_come_from_the_region() {
    let german = Region::of(&layers(
        &[
            ("region.first-day", "sunday"),
            ("region.number-format", "comma"),
        ],
        &[],
    ));
    assert_eq!(
        german.week_of(date(2026, 10, 7)).map(|week| week.first()),
        Some(date(2026, 10, 4))
    );
    assert_eq!(german.decimal_mark(), DecimalMark::Comma);
    assert_eq!(
        usual().week_of(date(2026, 10, 7)).map(|week| week.first()),
        Some(date(2026, 10, 5))
    );
    assert_eq!(usual().decimal_mark(), DecimalMark::Point);
}
