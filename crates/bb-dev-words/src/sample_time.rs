//! What TEST-SAMPLE-DATA holds the working hours and the time on issues of the sample against
//! (M6 WP-12): the part `issues.work.time` of the dump, judged by [`TimePlan::judge`] as part of
//! [`WorkPlan::judge`](crate::sample::WorkPlan).

use serde_json::Value;

use crate::sample::TimePlan;
use crate::sample_issues::{Judge, scaled};

/// The fewest weeks of working hours a sample records, however small it is.
const WEEKS_AT_LEAST: u64 = 2;

/// What the working hours of a sample asked for `asked` issues out of `full` come to: the counts of
/// the plan scaled like the issues. The program that makes the sample and the judge of its dump
/// both read them from here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeCounts {
    /// Weeks of working days.
    pub weeks: u64,
    /// Time entries.
    pub entries: u64,
    /// Days changed by the person, with a reason.
    pub changed_by_person: u64,
    /// Days changed by *HR*, with a reason.
    pub changed_by_hr: u64,
    /// Timers left running.
    pub running_timers: u64,
}

impl TimePlan {
    /// The counts for a sample of `asked` issues out of `full`; each is at least one (the weeks at
    /// least two), so even a small sample has every kind of record.
    #[must_use]
    pub fn counts(&self, asked: u32, full: u32) -> TimeCounts {
        let scale = |count: u32| scaled(count, full, asked).max(1);
        TimeCounts {
            weeks: scale(self.weeks).max(WEEKS_AT_LEAST),
            entries: scale(self.entries),
            changed_by_person: scale(self.changed_by_person),
            changed_by_hr: scale(self.changed_by_hr),
            running_timers: scale(self.running_timers),
        }
    }

    /// Judges the part `time` of the dump for a sample of `asked` issues out of `full`.
    pub(crate) fn judge(&self, judge: &mut Judge<'_>, time: &Value, asked: u32, full: u32) {
        let number = |value: &Value| value.as_u64().unwrap_or(0);
        let counts = self.counts(asked, full);
        judge.present("recorded work days", number(&time["work_days"]));
        judge.exactly("time entries", number(&time["entries"]), counts.entries);
        judge.exactly(
            "time entries with an Activity",
            number(&time["entries_with_activity"]),
            counts.entries,
        );
        judge.exactly(
            "time entries with a Cost center",
            number(&time["entries_with_cost_center"]),
            counts.entries,
        );
        judge.exactly(
            "days a person changed afterwards, with a reason",
            number(&time["changed_by_person"]),
            counts.changed_by_person,
        );
        judge.exactly(
            "days HR changed, with a reason",
            number(&time["changed_by_hr"]),
            counts.changed_by_hr,
        );
        judge.exactly(
            "timers left running",
            number(&time["running_timers"]),
            counts.running_timers,
        );
        judge.near(
            "the issues with an estimate",
            number(&time["estimated_issues"]),
            scaled(self.estimated_percent, 100, asked.max(1)).max(1),
        );
    }
}
