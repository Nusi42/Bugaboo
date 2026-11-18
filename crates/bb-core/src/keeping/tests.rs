use std::collections::BTreeSet;

use jiff::civil::date;
use jiff::tz::TimeZone;

use super::*;
use crate::settings::{OrganizationValue, SettingValue};

/// The layers with the organization's value of one setting.
fn with(setting: &Setting<Integer>, value: i64) -> Layers {
    let mut layers = Layers::default();
    layers.organization.insert(
        setting.definition().key().to_owned(),
        OrganizationValue {
            value: SettingValue::Integer(value),
            locked: false,
        },
    );
    layers
}

#[test]
fn priv_05_every_kind_of_data_has_one_row_and_one_group() {
    let kinds: BTreeSet<_> = KEEP_KINDS.iter().map(|row| row.kind).collect();
    assert_eq!(kinds.len(), KEEP_KINDS.len(), "a kind has two rows");
    assert_eq!(kinds.len(), KeepKind::ALL.len(), "a kind has no row");
    for kind in KeepKind::ALL {
        assert_eq!(rule(*kind).kind, *kind);
    }
    // Business records are deleted only by an administrator (ADR-0045, decision 1, M9-Q1).
    let by_an_administrator: BTreeSet<_> = KEEP_KINDS
        .iter()
        .filter(|row| row.group == KeepGroup::ByAnAdministrator)
        .map(|row| row.kind)
        .collect();
    assert_eq!(
        by_an_administrator,
        BTreeSet::from([KeepKind::WorkDays, KeepKind::TimeEntries])
    );
    // The page lists what Bugaboo deletes first, then what an administrator deletes.
    let groups: Vec<_> = KEEP_KINDS.iter().map(|row| row.group).collect();
    let mut sorted = groups.clone();
    sorted.sort();
    assert_eq!(groups, sorted);
}

#[test]
fn priv_05_a_period_below_its_floor_is_refused() {
    let mut floors = 0;
    for row in KEEP_KINDS {
        let (Some(setting), Some(floor)) = (row.setting(), row.floor()) else {
            assert!(
                matches!(row.period, Period::Fixed(_)),
                "{}: a setting without a floor",
                row.kind.code()
            );
            continue;
        };
        let definition = setting.definition();
        assert!(
            definition.accepts(&SettingValue::Integer(floor)),
            "{}",
            row.kind.code()
        );
        assert!(
            !definition.accepts(&SettingValue::Integer(floor - 1)),
            "{}: below its floor",
            row.kind.code()
        );
        floors += 1;
    }
    assert!(floors >= 6);
    assert_eq!(
        rule(KeepKind::Trash).floor(),
        Some(1),
        "the trash keeps a day"
    );
    // Work days keep the law's two years with *Germany*, whatever the setting says.
    let short = with(&WORKING_HOURS_KEEP_YEARS, 1);
    assert_eq!(rule(KeepKind::WorkDays).length(&short), Length::years(2));
}

#[test]
fn priv_05_each_period_follows_its_setting() {
    for row in KEEP_KINDS {
        let Some(setting) = row.setting() else {
            continue;
        };
        let Some((_, most)) = setting.definition().bounds() else {
            continue;
        };
        let changed = with(setting, most);
        assert_eq!(
            row.length(&changed).count,
            most,
            "{} does not read its setting",
            row.kind.code()
        );
    }
}

#[test]
fn priv_05_backups_are_kept_as_long_as_their_rotation_keeps_one() {
    use crate::settings::{BACKUPS_KEEP_DAILY, BACKUPS_KEEP_WEEKLY};

    let row = rule(KeepKind::Backups);
    assert_eq!(row.group, KeepGroup::ByItself);
    // The setting of the row is the one of the daily backups; its floor is a day.
    assert_eq!(row.floor(), Some(1));
    let mut layers = with(&BACKUPS_KEEP_DAILY, 3);
    layers.organization.insert(
        BACKUPS_KEEP_WEEKLY.definition().key().to_owned(),
        OrganizationValue {
            value: SettingValue::Integer(0),
            locked: false,
        },
    );
    assert_eq!(row.length(&layers), Length::days(3), "no weekly backups");
    let weeks = with(&BACKUPS_KEEP_WEEKLY, 10);
    assert_eq!(row.length(&weeks), Length::days(70), "ten weeks");
}

#[test]
fn priv_05_the_periods_start_as_decided() {
    let layers = Layers::default();
    let length = |kind| rule(kind).length(&layers);
    assert_eq!(length(KeepKind::IpAddresses), Length::days(7));
    assert_eq!(length(KeepKind::Notifications), Length::days(90));
    assert_eq!(length(KeepKind::Trash), Length::days(30));
    assert_eq!(length(KeepKind::DeadJobs), Length::days(30));
    assert_eq!(length(KeepKind::AwayTimes), Length::years(1));
    assert_eq!(length(KeepKind::SecurityLog), Length::days(365));
    // The newest backup of each of four weeks outlives the seven daily ones.
    assert_eq!(length(KeepKind::Backups), Length::days(28));
    assert_eq!(length(KeepKind::WorkDays), Length::years(2));
    assert_eq!(length(KeepKind::TimeEntries), Length::years(10));
}

#[test]
fn priv_05_kept_until_follows_the_period_of_the_kind() {
    let layers = Layers::default();
    // (kind, the record's date, the last day it is kept)
    let cases = [
        (KeepKind::TimeEntries, date(2026, 3, 10), date(2036, 3, 11)),
        (KeepKind::WorkDays, date(2026, 3, 10), date(2028, 3, 11)),
        (KeepKind::AwayTimes, date(2026, 3, 10), date(2027, 3, 11)),
        (KeepKind::Trash, date(2026, 3, 10), date(2026, 4, 10)),
        (KeepKind::Notifications, date(2026, 3, 10), date(2026, 6, 9)),
        (KeepKind::SignUpLinks, date(2026, 3, 10), date(2026, 3, 12)),
    ];
    for (kind, recorded, last) in cases {
        let row = rule(kind);
        assert_eq!(
            row.kept_until(&layers, recorded),
            KeptUntil::Day(last),
            "{}",
            kind.code()
        );
        // On the last day it is still kept; on the day after, it is past its period.
        assert_eq!(row.kept_from(&layers, last), Some(recorded));
        let next = last.tomorrow().unwrap();
        assert!(row.kept_from(&layers, next).unwrap() > recorded);
    }
    // The rotation keeps the newest backups whatever their age: a backup has no fixed last day.
    assert_eq!(
        rule(KeepKind::Backups).kept_until(&layers, date(2026, 3, 10)),
        KeptUntil::NoFixedEnd
    );
    // A day past the end of the calendar cannot be named either.
    assert_eq!(
        rule(KeepKind::TimeEntries).kept_until(&layers, date(9995, 1, 1)),
        KeptUntil::NoFixedEnd
    );
}

#[test]
fn priv_05_the_cutoff_counts_days_as_lengths_and_years_on_the_calendar() {
    let zone = TimeZone::get("Europe/Berlin").unwrap();
    let now: Timestamp = "2028-03-01T12:00:00Z".parse().unwrap();
    assert_eq!(
        Length::days(30).before(now, &zone),
        Some("2028-01-31T12:00:00Z".parse().unwrap())
    );
    assert_eq!(
        Length::hours(24).before(now, &zone),
        Some("2028-02-29T12:00:00Z".parse().unwrap())
    );
    assert_eq!(
        Length::years(1).before(now, &zone),
        Some("2027-03-01T12:00:00Z".parse().unwrap())
    );
}
