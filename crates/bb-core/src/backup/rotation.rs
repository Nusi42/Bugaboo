//! Which backups the schedule keeps (ADR-0046, decision 3): the newest backup of each of the
//! newest `keep-daily` days and of each of the newest `keep-weekly` weeks, counted in the time zone
//! of the organization (*Region › Time zone*, UTC when none is set) and a week as ISO 8601 counts
//! it. The backup a restore made of the present stays for the keep period of
//! the security log and goes after it (decision 4); of the backups made before an upgrade the
//! newest `keep-before-upgrade` stay (decision 5). A name the module [`super::name`] does not read
//! never comes here, so it is never deleted.

use jiff::civil::ISOWeekDate;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

use super::name::{BackupName, Purpose};

/// How many backups the schedule keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Keep {
    /// How many days keep their newest backup.
    pub daily: usize,
    /// How many weeks keep their newest backup.
    pub weekly: usize,
    /// How long a backup made before a restore is kept: the keep period of the security log.
    pub before_restore: SignedDuration,
    /// How many of the backups made before an upgrade are kept, the newest.
    pub before_upgrade: usize,
}

/// The backups among `backups` that the rotation deletes at `now`, in the order given, with the
/// days and weeks counted in `zone`.
#[must_use]
pub fn to_delete<'a>(
    backups: &'a [BackupName],
    keep: Keep,
    zone: &TimeZone,
    now: Timestamp,
) -> Vec<&'a BackupName> {
    let newest_first = |purpose: Purpose| {
        let mut of: Vec<&BackupName> = backups
            .iter()
            .filter(|backup| backup.purpose == purpose)
            .collect();
        of.sort_by_key(|backup| core::cmp::Reverse(backup.moment));
        of
    };
    let regular = newest_first(Purpose::Regular);
    let upgrades = newest_first(Purpose::BeforeUpgrade);
    let kept_upgrades = &upgrades[..keep.before_upgrade.min(upgrades.len())];
    let mut days = Vec::new();
    let mut weeks = Vec::new();
    let mut kept: Vec<&BackupName> = Vec::new();
    for backup in regular {
        let date = backup.moment.to_zoned(zone.clone()).date();
        let week = ISOWeekDate::from_date(date);
        let week = (week.year(), week.week());
        let mut keep_it = false;
        if !days.contains(&date) && days.len() < keep.daily {
            days.push(date);
            keep_it = true;
        }
        if !weeks.contains(&week) && weeks.len() < keep.weekly {
            weeks.push(week);
            keep_it = true;
        }
        if keep_it {
            kept.push(backup);
        }
    }
    backups
        .iter()
        .filter(|backup| match backup.purpose {
            Purpose::Regular => !kept.iter().any(|kept| std::ptr::eq(*kept, *backup)),
            Purpose::BeforeRestore => backup
                .moment
                .checked_add(keep.before_restore)
                .is_ok_and(|until| until < now),
            Purpose::BeforeUpgrade => !kept_upgrades
                .iter()
                .any(|kept| std::ptr::eq(*kept, *backup)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86_400;

    fn at(day: i64) -> Timestamp {
        // 2026-01-05 is a Monday; each backup is at three in the morning.
        Timestamp::from_second(1_767_582_000 + 3 * 3_600 + day * DAY).unwrap()
    }

    fn keep() -> Keep {
        Keep {
            daily: 7,
            weekly: 4,
            before_restore: SignedDuration::from_hours(24 * 365),
            before_upgrade: 3,
        }
    }

    #[test]
    fn the_rotation_keeps_the_newest_of_seven_days_and_four_weeks() {
        let backups: Vec<BackupName> = (0..40)
            .map(|day| BackupName::new(Purpose::Regular, at(day), "0.6.0"))
            .collect();
        let deleted = to_delete(&backups, keep(), &TimeZone::UTC, at(40));
        let kept: Vec<i64> = (0..40)
            .filter(|day| !deleted.iter().any(|gone| gone.moment == at(*day)))
            .collect();
        // Day 39 is a Friday: the seven newest days, and the newest backup (the Sunday) of each
        // of the two weeks before them.
        assert_eq!(kept, [20, 27, 33, 34, 35, 36, 37, 38, 39]);
    }

    #[test]
    fn ops_07_the_days_of_the_rotation_are_the_days_of_the_organization() {
        // 23:30 and 22:30 in UTC on one day; in Berlin the first is past midnight, the next day.
        let late = Timestamp::from_second(1_767_571_200 + 23 * 3_600 + 1_800).unwrap();
        let earlier = late.checked_sub(SignedDuration::from_hours(1)).unwrap();
        let backups = vec![
            BackupName::new(Purpose::Regular, late, "0.6.0"),
            BackupName::new(Purpose::Regular, earlier, "0.6.0"),
        ];
        let now = late.checked_add(SignedDuration::from_hours(2)).unwrap();
        assert_eq!(
            to_delete(&backups, keep(), &TimeZone::UTC, now),
            [&backups[1]],
            "one day in UTC"
        );
        let berlin = TimeZone::get("Europe/Berlin").unwrap();
        assert!(
            to_delete(&backups, keep(), &berlin, now).is_empty(),
            "two days in Berlin"
        );
    }

    #[test]
    fn a_second_backup_of_a_day_goes_and_a_backup_before_a_restore_stays_its_period() {
        let backups = vec![
            BackupName::new(Purpose::Regular, at(5), "0.6.0"),
            BackupName::new(
                Purpose::Regular,
                at(5).checked_sub(SignedDuration::from_hours(1)).unwrap(),
                "0.6.0",
            ),
            BackupName::new(Purpose::BeforeRestore, at(0), "0.6.0"),
        ];
        let deleted = to_delete(&backups, keep(), &TimeZone::UTC, at(6));
        assert_eq!(deleted, [&backups[1]]);
        let a_year_later = at(366);
        let deleted = to_delete(&backups, keep(), &TimeZone::UTC, a_year_later);
        assert_eq!(deleted, [&backups[1], &backups[2]]);
    }

    #[test]
    fn ops_06_the_newest_three_backups_before_an_upgrade_are_kept() {
        let backups: Vec<BackupName> = (0..5)
            .map(|day| BackupName::new(Purpose::BeforeUpgrade, at(day), "0.6.0"))
            .chain([BackupName::new(Purpose::Regular, at(4), "0.6.0")])
            .collect();
        let deleted = to_delete(&backups, keep(), &TimeZone::UTC, at(5));
        assert_eq!(deleted, [&backups[0], &backups[1]], "the two oldest go");
        let none = Keep {
            before_upgrade: 0,
            ..keep()
        };
        assert_eq!(
            to_delete(&backups, none, &TimeZone::UTC, at(5)).len(),
            5,
            "the regular backup stays by its own rule"
        );
    }
}
