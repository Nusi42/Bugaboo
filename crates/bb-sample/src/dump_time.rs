//! The part of the canonical dump that is about working hours and time on issues (M6 WP-12): the
//! recorded work days, the time entries with their *Activity* and *Cost center*, the days changed
//! afterwards with a reason (by the person, by *HR*), the issues with an estimate and the timers
//! that run. Only counts: IDs are random by design.
//!
//! TEST-SAMPLE-DATA holds these numbers against the plan in `tools/dev/sample.json`
//! (`bb_dev_words::sample::TimePlan`).

use std::collections::BTreeMap;

use bb_app::access::StartingRole;
use bb_app::time::TimeStore as _;
use bb_core::event::{DomainEvent, RecordKind};
use bb_core::id::UserId;
use bb_core::issue::Issue;
use bb_core::time::entry::StarterField;
use serde::Serialize;

use crate::error::{SampleError, Step};
use crate::generate::Sample;

/// The working hours and the time of the sample.
#[derive(Debug, Default, Serialize)]
pub(crate) struct DumpTime {
    work_days: u64,
    entries: u64,
    entries_with_activity: u64,
    entries_with_cost_center: u64,
    /// Days a person changed afterwards, on their own, with a reason.
    changed_by_person: u64,
    /// Days somebody with the role *HR* changed, with a reason.
    changed_by_hr: u64,
    estimated_issues: u64,
    running_timers: u64,
}

impl Sample {
    /// Counts working hours and time in the events, and reads the timers from the store.
    pub(crate) async fn time_of(
        &self,
        roles: &BTreeMap<UserId, String>,
        events: &[DomainEvent],
        issues: &[Issue],
    ) -> Result<DumpTime, SampleError> {
        let mut time = DumpTime {
            estimated_issues: issues
                .iter()
                .filter(|issue| issue.estimated_time.is_some())
                .count() as u64,
            ..DumpTime::default()
        };
        let marks = self
            .storage
            .starter_fields()
            .await
            .map_err(|error| SampleError::step(Step::Time, &error))?;
        let field_of = |starter: StarterField| {
            marks
                .iter()
                .find(|mark| mark.starter == starter)
                .map(|mark| mark.field.to_string())
                .unwrap_or_default()
        };
        let (activity, cost_center) = (
            field_of(StarterField::Activity),
            field_of(StarterField::CostCenter),
        );
        let (work_day, entry) = (RecordKind::WorkDay.code(), RecordKind::TimeEntry.code());
        let hr = StartingRole::Hr.code();
        for event in events {
            let kind = event.entity.kind.as_str();
            let made = event.changes.iter().all(|change| change.before.is_none());
            if kind == work_day && made {
                time.work_days += 1;
            }
            if kind == work_day && event.reason.is_some() {
                let by_hr = match event.actor {
                    bb_core::event::Actor::Person(person) => {
                        roles.get(&person).is_some_and(|role| role == hr)
                    }
                    bb_core::event::Actor::System => false,
                };
                if by_hr {
                    time.changed_by_hr += 1;
                } else {
                    time.changed_by_person += 1;
                }
            }
            if kind == entry && made {
                time.entries += 1;
                let has = |field: &str| event.changes.iter().any(|change| change.field == field);
                time.entries_with_activity += u64::from(has(&activity));
                time.entries_with_cost_center += u64::from(has(&cost_center));
            }
        }
        for person in &self.people {
            let timer = self
                .storage
                .timer(person.id)
                .await
                .map_err(|error| SampleError::step(Step::Time, &error))?;
            time.running_timers += u64::from(timer.is_some_and(|timer| timer.running.is_some()));
        }
        Ok(time)
    }
}
