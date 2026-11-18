//! Which backups the schedule keeps (ADR-0046, decision 3): the newest backup of each of the
//! newest `keep-daily` days and of each of the newest `keep-weekly` weeks, counted in UTC (a week
//! as ISO 8601 counts it). The backup a restore made of the present stays for the keep period of
//! the security log and goes after it (decision 4). A name the module [`super::name`] does not read
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
}

/// The backups among `backups` that the rotation deletes at `now`, in the order given.
#[must_use]
pub fn to_delete(backups: &[BackupName], keep: Keep, now: Timestamp) -> Vec<&BackupName> {
    let mut regular: Vec<&BackupName> = backups
        .iter()
        .filter(|backup| backup.purpose == Purpose::Regular)
        .collect();
    regular.sort_by_key(|backup| core::cmp::Reverse(backup.moment));
    let mut days = Vec::new();
    let mut weeks = Vec::new();
    let mut kept: Vec<&BackupName> = Vec::new();
    for backup in regular {
        let date = backup.moment.to_zoned(TimeZone::UTC).date();
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
        }
    }

    #[test]
    fn the_rotation_keeps_the_newest_of_seven_days_and_four_weeks() {
        let backups: Vec<BackupName> = (0..40)
            .map(|day| BackupName::new(Purpose::Regular, at(day), "0.6.0"))
            .collect();
        let deleted = to_delete(&backups, keep(), at(40));
        let kept: Vec<i64> = (0..40)
            .filter(|day| !deleted.iter().any(|gone| gone.moment == at(*day)))
            .collect();
        // Day 39 is a Friday: the seven newest days, and the newest backup (the Sunday) of each
        // of the two weeks before them.
        assert_eq!(kept, [20, 27, 33, 34, 35, 36, 37, 38, 39]);
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
        let deleted = to_delete(&backups, keep(), at(6));
        assert_eq!(deleted, [&backups[1]]);
        let a_year_later = at(366);
        let deleted = to_delete(&backups, keep(), a_year_later);
        assert_eq!(deleted, [&backups[1], &backups[2]]);
    }
}
