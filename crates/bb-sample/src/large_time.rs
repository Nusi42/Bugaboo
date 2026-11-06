//! Time on issues and working hours in the large dataset (M6 WP-12, PERF-01): the people book time
//! on the issues they work on, and the people who work record their days. Written in big
//! transactions like the issues, without the use cases; the events are built by the functions of
//! the use cases (`bb_app::event_members`), so an event of the dataset says what the event of a
//! person says (ADR-0007).
//!
//! The time entries are part of the bundle of an issue ([`Booked`]); the work days are written
//! after the issues by [`write_days`].

use bb_app::event_members as events;
use bb_app::time::TimeStore as _;
use bb_app::working_hours::WorkingHoursStore as _;
use bb_core::event::{DomainEvent, RecordKind};
use bb_core::id::{EntityId, FieldId, IssueId, TimeEntryId, UserId, WorkDayId};
use bb_core::own_fields::FieldType;
use bb_core::own_fields_value::{Cell, FieldValue};
use bb_core::settings::Layers;
use bb_core::time::Minutes;
use bb_core::time::entry::TimeEntry;
use bb_core::version::FIRST_VERSION;
use bb_core::working_time::record::{StoredDay, WorkDay};
use bb_core::working_time::{Rules, Span, Times, warnings};
use bb_dev_words::LargePlan;
use bb_store::AuthStorage;
use jiff::civil::Date;
use jiff::{SignedDuration, Timestamp};

use crate::error::SampleError;
use crate::generate::World;
use crate::large_build::{Maker, event};
use crate::large_ids::{ordered, rng_for};
use crate::large_org::Org;
use crate::random::SeededRandom;
use crate::time_fields::StarterFields;
use bb_core::time::SECONDS_PER_DAY;

/// Different streams of random numbers than the issues use, whatever the seed.
const ENTRIES: u64 = 11;
const ENTRY_IDS: u64 = 12;
const DAYS: u64 = 13;
const DAY_IDS: u64 = 14;
/// A time entry is booked within this many days of the making of its issue.
const BOOKED_WITHIN_DAYS: i64 = 20;
/// The lengths of a time entry, in minutes.
const ENTRY_MINUTES: [u32; 7] = [15, 30, 45, 60, 60, 90, 120];
/// How many days of a transaction of work days.
const DAYS_AT_A_TIME: usize = 5000;
const MINUTE: i64 = 60;
/// When a day starts at the earliest, in minutes after midnight, and how much later it may.
const START_FROM: i64 = 7 * 60 + 30;
const START_WITHIN: usize = 90;
/// An ordinary day: eight hours of work with a break of half an hour at noon.
const WORK_MINUTES: i64 = 8 * 60;
const BREAK_AT: i64 = 4 * 60;
const BREAK_MINUTES: i64 = 30;
/// A day that is too long (a rule is broken): twelve hours, without a break.
const LONG_WORK_MINUTES: i64 = 12 * 60;

/// A time entry with the values of its own fields, to be written with its issue.
pub(crate) struct Booked {
    pub(crate) entry: TimeEntry,
    pub(crate) values: Vec<(FieldId, Vec<Cell>)>,
}

impl Maker<'_> {
    /// The time `person` booked on the issue `issue` that was made at `created`: none for an issue
    /// nobody works on. Each entry has its event, appended to `events`.
    pub(crate) fn book_time(
        &self,
        index: u32,
        (issue, person, created): (IssueId, Option<UserId>, i64),
        events: &mut Vec<DomainEvent>,
    ) -> Result<Vec<Booked>, SampleError> {
        let (Some(person), Some(fields)) = (person, self.bank.time.as_ref()) else {
            return Ok(Vec::new());
        };
        let mut random = rng_for(self.seed, ENTRIES, u64::from(index));
        let mut ids = rng_for(self.seed, ENTRY_IDS, u64::from(index));
        let most = usize::try_from(self.bank.plan.time_entries_per_100 / 100 * 2).unwrap_or(0);
        let count = random.below(most + 1);
        let mut booked = Vec::with_capacity(count);
        for _ in 0..count {
            let at =
                (created + random.below_i64(BOOKED_WITHIN_DAYS * SECONDS_PER_DAY)).min(self.end);
            let time = Timestamp::from_second(at).unwrap_or(Timestamp::UNIX_EPOCH);
            let entry = TimeEntry {
                id: ordered(TimeEntryId::from_parts, time, &mut ids)?,
                person,
                issue,
                date: time.to_zoned(jiff::tz::TimeZone::UTC).date(),
                minutes: Minutes::new(ENTRY_MINUTES[random.below(ENTRY_MINUTES.len())]),
                note: String::new(),
                billable: false,
                started_at: None,
                created_at: time,
                version: FIRST_VERSION,
            };
            let (own, values) = values_of(fields, &mut random);
            events.push(event(
                &mut ids,
                time,
                person,
                RecordKind::TimeEntry,
                entry.id,
                events::time_entry_booked(&entry, own),
            )?);
            booked.push(Booked { entry, values });
        }
        Ok(booked)
    }
}

/// A choice of *Activity* and of *Cost center*, as the changes of the event and the cells to store.
fn values_of(
    fields: &StarterFields,
    random: &mut SeededRandom,
) -> (Vec<bb_core::event::FieldChange>, Vec<(FieldId, Vec<Cell>)>) {
    let mut changes = Vec::new();
    let mut values = Vec::new();
    for starter in [&fields.activity, &fields.cost_center] {
        if starter.choices.is_empty() {
            continue;
        }
        let choice = starter.choices[random.below(starter.choices.len())];
        let value = FieldValue::Ids(vec![choice.to_bytes()]);
        changes.push(events::own_value_set(
            starter.field,
            FieldType::PickOne,
            &value,
        ));
        values.push((starter.field, value.to_cells()));
    }
    (changes, values)
}

/// The working days (days since 1970) of the last `weeks` weeks before `end`, in order.
fn working_days(end: i64, weeks: u32) -> Vec<i64> {
    let last = end.div_euclid(SECONDS_PER_DAY) - 1;
    let first = last - i64::from(weeks) * 7 + 1;
    // Day 0 of 1970 was a Thursday, so a day is Monday to Friday when this is below five.
    (first..=last)
        .filter(|day| (day + 3).rem_euclid(7) < 5)
        .collect()
}

/// The times of one worked day, in UTC: when it starts is the person's own, and a few days are
/// too long.
fn times_of(date_start: i64, random: &mut SeededRandom, long: bool) -> Times {
    let start =
        date_start + (START_FROM + i64::try_from(random.below(START_WITHIN)).unwrap_or(0)) * MINUTE;
    let at = |seconds: i64| Timestamp::from_second(seconds).unwrap_or(Timestamp::UNIX_EPOCH);
    if long {
        let end = start + LONG_WORK_MINUTES * MINUTE;
        return Times {
            periods: vec![Span::closed(at(start), at(end))],
            breaks: Vec::new(),
        };
    }
    let end = start + (WORK_MINUTES + BREAK_MINUTES) * MINUTE;
    let break_start = start + BREAK_AT * MINUTE;
    Times {
        periods: vec![Span::closed(at(start), at(end))],
        breaks: vec![Span::closed(
            at(break_start),
            at(break_start + BREAK_MINUTES * MINUTE),
        )],
    }
}

/// Writes the working days of everybody who works: the last `plan.work_weeks` weeks before the end
/// of the dataset, a day on leave now and then and a long day now and then, each with its warnings
/// as the use case stores them and with its event.
///
/// # Errors
/// [`SampleError`] when a write fails.
pub(crate) async fn write_days(
    world: &World<'_>,
    org: &Org,
    plan: &LargePlan,
    end: i64,
) -> Result<(), SampleError> {
    let rules = Rules::of(&Layers::default());
    let mut batch: Vec<(StoredDay, DomainEvent)> = Vec::new();
    let days = working_days(end, plan.work_weeks);
    let workers: Vec<UserId> = org
        .folk
        .iter()
        .filter(|folk| folk.works())
        .map(|folk| folk.id)
        .collect();
    for (at, person) in workers.iter().enumerate() {
        let mut random = rng_for(world.seed, DAYS, u64::try_from(at).unwrap_or(0));
        let mut ids = rng_for(world.seed, DAY_IDS, u64::try_from(at).unwrap_or(0));
        let mut before: Option<Times> = None;
        for day in &days {
            if random.chance(plan.leave_percent) {
                continue;
            }
            let long = random.chance(plan.long_day_percent);
            let times = times_of(*day * SECONDS_PER_DAY, &mut random, long);
            let date = Date::constant(1970, 1, 1)
                .checked_add(jiff::Span::new().days(*day))
                .map_err(|_| SampleError::Start(*day))?;
            let updated = times
                .periods
                .last()
                .and_then(|span| span.end)
                .unwrap_or(Timestamp::UNIX_EPOCH);
            let id: WorkDayId = ordered(WorkDayId::from_parts, updated, &mut ids)?;
            let stored = StoredDay {
                day: WorkDay {
                    id,
                    person: *person,
                    date,
                    zone: "UTC".to_owned(),
                    warnings: warnings(&times, before.as_ref(), rules.as_ref()),
                    updated_at: updated,
                    version: FIRST_VERSION,
                },
                times: times.clone(),
            };
            let made = event(
                &mut ids,
                updated + SignedDuration::from_secs(1),
                *person,
                RecordKind::WorkDay,
                EntityId::from(id),
                events::work_day_recorded(*person, date, None, &times, None),
            )?;
            before = Some(times);
            batch.push((stored, made));
            if batch.len() >= DAYS_AT_A_TIME {
                write(world.storage, std::mem::take(&mut batch)).await?;
            }
        }
    }
    write(world.storage, batch).await?;
    // The clock stands where the dataset ends, for what is read from the events next.
    world
        .clock
        .set(Timestamp::from_second(end).unwrap_or(world.start));
    Ok(())
}

async fn write(
    storage: &AuthStorage,
    mut days: Vec<(StoredDay, DomainEvent)>,
) -> Result<(), SampleError> {
    if days.is_empty() {
        return Ok(());
    }
    let mut events: Vec<DomainEvent> = days.iter().map(|(_, event)| event.clone()).collect();
    events.sort_by_key(|event| event.time);
    storage
        .database()
        .record_changes(&events, async |transaction| {
            for (stored, _) in days.drain(..) {
                AuthStorage::insert_work_day(transaction, &stored).await?;
            }
            Ok(())
        })
        .await?;
    Ok(())
}

/// Inserts the time entries of a bundle (used by the writer of a batch).
pub(crate) async fn insert_entries(
    transaction: &mut <AuthStorage as bb_app::ports::ChangeStore>::Transaction,
    booked: &[Booked],
) -> Result<(), <AuthStorage as bb_app::ports::ChangeStore>::Error> {
    use bb_app::fields::FieldStore as _;
    use bb_core::event::EntityKind;
    for each in booked {
        AuthStorage::insert_time_entry(transaction, &each.entry).await?;
        for (field, cells) in &each.values {
            AuthStorage::put_field_value(
                transaction,
                EntityKind::from(RecordKind::TimeEntry),
                each.entry.id.into(),
                *field,
                cells,
            )
            .await?;
        }
    }
    Ok(())
}
