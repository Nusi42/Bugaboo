//! Working hours and time on issues in the sample (M6 WP-12, TIME-01 to TIME-13): estimates on a
//! third of the issues, the weeks of working hours of every employee, the time entries with
//! *Activity* and *Cost center*, a few days changed afterwards with a reason (by the person and by
//! *HR*), and some timers left running.
//!
//! All of it goes through the use cases the screens call, one change a second of the sample's clock.
//! The clock is set back to the first day of the weeks and walks forward day by day, so every
//! person taps their day at a moment of their own (their day in their own time zone) and a day
//! changed afterwards is a real past day that asks for its reason. The counts are those of
//! [`TimePlan::counts`], which the judge of the dump reads too.

use std::collections::BTreeMap;

use bb_app::access::StartingRole;
use bb_app::auth::AppError;
use bb_app::issues::{ChangeIssue, IssueQuery, change_issue, issue};
use bb_app::settings::{ChangeSetting, ResetSetting, SettingsScope, change_setting, reset_setting};
use bb_app::time::{AddTime, EntryDetails, StartTimer, add_time, start_timer};
use bb_app::working_hours::{
    ChangeWorkDay, StartWorkDay, Tap, TapWorkDay, WorkingHoursStore as _, change_work_day,
    start_work_day, tap_work_day,
};
use bb_core::given::SetTo;
use bb_core::settings::{SECURITY_SESSION_IDLE_MINUTES, SettingValue};
use bb_core::time::Minutes;
use bb_core::working_time::Span;
use bb_dev_words::sample::TimePlan;
use jiff::SignedDuration;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde_json::json;

use crate::error::{SampleError, Step};
use crate::issues::Maker;
use crate::issues_hands::Hands;
use crate::issues_setup::json;
use crate::issues_time::Calendar;
use crate::random::SeededRandom;
use crate::time_fields::{Starter, starter_fields};

/// A different stream of random numbers than the issues and the work together use.
const SALT: u64 = 0x54_49_4D_45;
/// The lengths of an estimate, in minutes.
const ESTIMATES: [u32; 6] = [60, 120, 240, 480, 960, 1920];
/// The lengths of a time entry, in minutes.
const ENTRY_MINUTES: [u32; 7] = [15, 30, 45, 60, 60, 90, 120];
/// How many other issues a booking tries when a person may not book on the first one.
const TRIES_PER_ENTRY: usize = 400;
/// How many of a hundred bookings of a person go to an issue they booked on before: people book
/// on the same few issues, and some see few issues at all.
const REPEAT_PERCENT: u32 = 90;
/// How many issues a person is remembered to have booked on.
const KNOWN_ISSUES: usize = 30;
/// How long after the last day the days are changed and the timers are started, in seconds.
const LATER_SECONDS: i64 = SECONDS_PER_HOUR;
/// How much later a day ends when it is changed afterwards, in minutes.
const LATER_END_MINUTES: i64 = 30;
/// Seconds after the end of a day that the person books its time.
const BOOKS_AFTER: i64 = 300;
/// How long sessions may idle while the days are recorded, in minutes: the longest the setting
/// allows, so a person who taps their day a few times is signed in once a day, not at every tap.
const SESSION_IDLE_MINUTES: i64 = 1440;
const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: i64 = 3600;
const MINUTES_PER_HOUR: usize = 60;

/// The starter fields of time entries and their choices, as the entries are filled in.
struct Fields {
    activity: (String, Vec<String>),
    cost_center: (String, Vec<String>),
}

/// What happens in a day of a person.
#[derive(Clone, Copy, Debug)]
enum Happens {
    Start,
    BreakStart,
    BreakEnd,
    End,
    /// The person books this many time entries.
    Books(usize),
}

/// One thing that happens at a moment (seconds since 1970).
#[derive(Clone, Copy, Debug)]
struct Event {
    moment: i64,
    person: usize,
    date: Date,
    happens: Happens,
}

/// A day a person worked, kept for the days that are changed afterwards.
#[derive(Clone, Copy, Debug)]
struct Worked {
    person: usize,
    date: Date,
}

/// The date of a day number (days since 1970).
fn date_of(day: i64) -> Result<Date, SampleError> {
    Date::constant(1970, 1, 1)
        .checked_add(jiff::Span::new().days(day))
        .map_err(|_| SampleError::Start(day))
}

/// The moment (seconds since 1970) of a time of day on a date in a zone.
fn moment_of(date: Date, zone: &TimeZone, minutes: i64) -> Result<i64, SampleError> {
    let hours = i8::try_from(minutes / 60).unwrap_or(0);
    let minute = i8::try_from(minutes % 60).unwrap_or(0);
    date.at(hours, minute, 0, 0)
        .to_zoned(zone.clone())
        .map(|zoned| zoned.timestamp().as_second())
        .map_err(|_| SampleError::Start(minutes))
}

impl Maker<'_, '_> {
    /// Estimates, working hours, time entries, changed days and running timers (see the module).
    pub(crate) async fn work_hours(
        &mut self,
        calendar: &Calendar,
        asked: u32,
        full: u32,
    ) -> Result<(), SampleError> {
        let plan = self.names.work.time.clone();
        let counts = plan.counts(asked, full);
        let mut random = SeededRandom::new(self.world.seed ^ SALT);
        let fields = self.time_fields(&plan).await?;
        self.estimate(&plan, &mut random).await?;
        self.session_idle(Some(SESSION_IDLE_MINUTES)).await?;
        let mut hands = Hands::new(self.world, calendar.clone(), 0)?
            .idling((SESSION_IDLE_MINUTES - 60) * SECONDS_PER_MINUTE);
        let mut employees = self.employees();
        employees.sort_by_key(|at| !self.world.people[*at].person.is_fixed());
        // A small sample records the hours of fewer people, the people of the demos first.
        let share = employees.len() * usize::try_from(asked).unwrap_or(0)
            / usize::try_from(full).unwrap_or(1).max(1);
        employees.truncate(share.max(usize::try_from(plan.employees_at_least).unwrap_or(0)));
        let weeks = usize::try_from(counts.weeks).unwrap_or(usize::MAX);
        let days: Vec<i64> = calendar
            .days()
            .iter()
            .rev()
            .take(weeks.saturating_mul(5))
            .rev()
            .copied()
            .collect();
        let (events, worked) =
            self.schedule(&plan, &mut random, &employees, &days, counts.entries)?;
        self.walk(&events, &fields, &mut hands, &mut random).await?;
        let last = events.last().map_or(0, |event| event.moment);
        self.world.move_clock_to(last + LATER_SECONDS)?;
        self.change_days(
            &plan,
            &mut hands,
            &mut random,
            &worked,
            counts.changed_by_person,
            counts.changed_by_hr,
        )
        .await?;
        self.leave_timers(&mut hands, &mut random, &employees, counts.running_timers)
            .await?;
        self.session_idle(None).await
    }

    /// The organization lets sessions idle this long (minutes), or goes back to its default.
    async fn session_idle(&mut self, minutes: Option<i64>) -> Result<(), SampleError> {
        let administrator = self.administrator()?;
        let token = self.hands.token(administrator).await?;
        let key = SECURITY_SESSION_IDLE_MINUTES.definition().key().to_owned();
        self.world.tick();
        let ctx = self.world.services.ctx(self.world.request());
        match minutes {
            Some(minutes) => change_setting(
                &ctx,
                ChangeSetting {
                    version: None,
                    session_token: token,
                    scope: SettingsScope::Organization,
                    key,
                    value: SettingValue::Integer(minutes),
                },
            )
            .await
            .map(drop),
            None => reset_setting(
                &ctx,
                ResetSetting {
                    session_token: token,
                    scope: SettingsScope::Organization,
                    key,
                },
            )
            .await
            .map(drop),
        }
        .map_err(|error| SampleError::step(Step::Settings, &error))
    }

    /// The people who record their working hours: everyone who works in a project (the Viewers
    /// and Guests only read, and *HR* does not work in projects), without the people who sign in
    /// with a code app.
    fn employees(&self) -> Vec<usize> {
        self.world
            .people
            .iter()
            .enumerate()
            .filter(|(_, made)| {
                !made.person.code_app
                    && [
                        StartingRole::Administrator,
                        StartingRole::ProjectLead,
                        StartingRole::TeamMember,
                        StartingRole::Contributor,
                    ]
                    .contains(&made.person.role)
            })
            .map(|(at, _)| at)
            .collect()
    }

    /// The first administrator's place in the plan of the people.
    pub(crate) fn administrator(&self) -> Result<usize, SampleError> {
        self.world.administrator()
    }

    /// The IDs of *Activity* and *Cost center* and of their choices, as the entries need them.
    async fn time_fields(&mut self, plan: &TimePlan) -> Result<Fields, SampleError> {
        let administrator = self.administrator()?;
        let token = self.hands.token(administrator).await?;
        let made = starter_fields(self.world, &token, &plan.cost_centers).await?;
        let of = |starter: &Starter| {
            (
                starter.field.to_string(),
                starter.choices.iter().map(ToString::to_string).collect(),
            )
        };
        Ok(Fields {
            activity: of(&made.activity),
            cost_center: of(&made.cost_center),
        })
    }

    /// An *Estimated time* on the share of the issues the plan says, by an administrator.
    async fn estimate(
        &mut self,
        plan: &TimePlan,
        random: &mut SeededRandom,
    ) -> Result<(), SampleError> {
        let administrator = self.administrator()?;
        for at in 0..self.issues.len() {
            let Some(made) = self.issues.get(at).and_then(Option::as_ref) else {
                continue;
            };
            // Evenly spread: exactly the share the plan says of the issues have an estimate.
            let percent = usize::try_from(plan.estimated_percent).unwrap_or(0);
            if (at + 1) * percent / 100 == at * percent / 100 {
                continue;
            }
            let minutes = ESTIMATES[random.below(ESTIMATES.len())];
            let id = made.id;
            let token = self.hands.token(administrator).await?;
            self.world.tick();
            // The version the issue has now: the stages before this one changed many of them.
            let current = json(
                issue(
                    &self.world.services.ctx(self.world.request()),
                    IssueQuery {
                        session_token: token.clone(),
                        issue: id.to_string(),
                    },
                )
                .await,
                Step::Change,
            )?;
            let version = current["issue"]["version"].as_i64().unwrap_or(1);
            json(
                change_issue(
                    &self.world.services.ctx(self.world.request()),
                    ChangeIssue {
                        session_token: token,
                        issue: id.to_string(),
                        version,
                        issue_type: None,
                        title: None,
                        status: None,
                        priority: None,
                        estimated_time: Some(SetTo::Value(Minutes::new(minutes))),
                        time_left: None,
                        assigned_to: None,
                        fields: BTreeMap::new(),
                    },
                )
                .await,
                Step::Change,
            )?;
            if let Some(Some(made)) = self.issues.get_mut(at) {
                made.version = version + 1;
            }
        }
        Ok(())
    }

    /// Plans every day of every employee: when it starts, has its break, ends, and when its time is
    /// booked. Leave days have nothing. The events are in the order they happen.
    fn schedule(
        &self,
        plan: &TimePlan,
        random: &mut SeededRandom,
        employees: &[usize],
        days: &[i64],
        entries: u64,
    ) -> Result<(Vec<Event>, Vec<Worked>), SampleError> {
        let mut events: Vec<Event> = Vec::new();
        let mut worked: Vec<Worked> = Vec::new();
        for day in days {
            let date = date_of(*day)?;
            for person in employees {
                if random.chance(plan.leave_percent) {
                    continue;
                }
                let zone = TimeZone::get(&self.world.people[*person].person.region.time_zone)
                    .map_err(|_| SampleError::NotMade("time zone".to_owned()))?;
                Self::plan_day(plan, random, (*person, date), &zone, &mut events)?;
                worked.push(Worked {
                    person: *person,
                    date,
                });
            }
        }
        events.sort_by_key(|event| (event.moment, event.person));
        spread(&mut events, entries);
        Ok((events, worked))
    }

    /// The events of one worked day, in the zone of the person.
    fn plan_day(
        plan: &TimePlan,
        random: &mut SeededRandom,
        (person, date): (usize, Date),
        zone: &TimeZone,
        events: &mut Vec<Event>,
    ) -> Result<(), SampleError> {
        let at = |minutes: usize| moment_of(date, zone, i64::try_from(minutes).unwrap_or(0));
        let start = 7 * MINUTES_PER_HOUR + 30 + random.below(90);
        let breaking = random.chance(plan.rule_breaking_percent);
        // A day that breaks a rule is either too long, or long and without a break.
        let long = breaking && random.chance(50);
        let without_break = breaking && !long;
        let length = if long {
            12 * MINUTES_PER_HOUR
        } else {
            8 * MINUTES_PER_HOUR + random.below(45)
        };
        let pause = if random.chance(50) { 30 } else { 45 };
        let break_start = start + 4 * MINUTES_PER_HOUR + random.below(45);
        let end = start + length + if without_break { 0 } else { pause };
        let mut push = |minutes: usize, happens: Happens| -> Result<(), SampleError> {
            events.push(Event {
                moment: at(minutes)?,
                person,
                date,
                happens,
            });
            Ok(())
        };
        push(start, Happens::Start)?;
        if !without_break {
            push(break_start, Happens::BreakStart)?;
            push(break_start + pause, Happens::BreakEnd)?;
        }
        push(end, Happens::End)?;
        let booked = at(end)? + BOOKS_AFTER;
        events.push(Event {
            moment: booked,
            person,
            date,
            happens: Happens::Books(1),
        });
        Ok(())
    }

    /// Does what the events say, one after the other, on the clock set to each moment.
    async fn walk(
        &mut self,
        events: &[Event],
        fields: &Fields,
        hands: &mut Hands<'_, '_>,
        random: &mut SeededRandom,
    ) -> Result<(), SampleError> {
        let step = |error: &dyn std::fmt::Display| SampleError::step(Step::Time, &error);
        let mut versions: BTreeMap<usize, i64> = BTreeMap::new();
        let mut known: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut last = i64::MIN / 2;
        for event in events {
            let moment = event.moment.max(last + 1);
            last = moment;
            self.world.move_clock_to(moment)?;
            let token = hands.token(event.person).await?;
            let ctx = self.world.services.ctx(self.world.request());
            let tapped = |version: i64, tap: Tap| TapWorkDay {
                session_token: token.clone(),
                version,
                tap,
            };
            match event.happens {
                Happens::Start => {
                    start_work_day(
                        &ctx,
                        StartWorkDay {
                            session_token: token.clone(),
                            device_zone: None,
                        },
                    )
                    .await
                    .map_err(|error| step(&error))?;
                }
                Happens::BreakStart | Happens::BreakEnd | Happens::End => {
                    let version = versions.get(&event.person).copied().unwrap_or(1);
                    let tap = match event.happens {
                        Happens::BreakStart => Tap::StartBreak,
                        Happens::BreakEnd => Tap::EndBreak,
                        _ => Tap::End,
                    };
                    tap_work_day(&ctx, tapped(version, tap))
                        .await
                        .map_err(|error| step(&error))?;
                }
                Happens::Books(count) => {
                    self.book(event, count, (fields, &mut known), hands, random)
                        .await?;
                    continue;
                }
            }
            // The version of the day, for the next tap: as the person's screen has it.
            let stored = self
                .world
                .storage
                .work_day(self.world.people[event.person].id, event.date)
                .await
                .map_err(|error| step(&error))?;
            if let Some(stored) = stored {
                versions.insert(event.person, stored.day.version);
            }
        }
        Ok(())
    }

    /// Books `count` time entries of a person on a date, on issues the person may book on.
    async fn book(
        &mut self,
        event: &Event,
        count: usize,
        (fields, known): (&Fields, &mut BTreeMap<usize, Vec<usize>>),
        hands: &mut Hands<'_, '_>,
        random: &mut SeededRandom,
    ) -> Result<(), SampleError> {
        for _ in 0..count {
            let mut booked = false;
            for _ in 0..TRIES_PER_ENTRY {
                let before = known.entry(event.person).or_default();
                let at = if !before.is_empty() && random.chance(REPEAT_PERCENT) {
                    before[random.below(before.len())]
                } else {
                    random.below(self.issues.len())
                };
                let Some(Some(issue)) = self.issues.get(at) else {
                    continue;
                };
                let id = issue.id;
                let token = hands.token(event.person).await?;
                let mut details = EntryDetails {
                    note: None,
                    billable: None,
                    fields: BTreeMap::new(),
                };
                for (key, choices) in [&fields.activity, &fields.cost_center] {
                    if let Some(choice) = choices.get(random.below(choices.len().max(1))) {
                        details.fields.insert(key.clone(), json!(choice));
                    }
                }
                self.world.tick();
                let answer = add_time(
                    &self.world.services.ctx(self.world.request()),
                    AddTime {
                        session_token: token,
                        issue: id.to_string(),
                        date: event.date,
                        minutes: Minutes::new(ENTRY_MINUTES[random.below(ENTRY_MINUTES.len())]),
                        details,
                    },
                )
                .await;
                match answer {
                    Ok(_) => {
                        let before = known.entry(event.person).or_default();
                        if before.len() < KNOWN_ISSUES && !before.contains(&at) {
                            before.push(at);
                        }
                        booked = true;
                        break;
                    }
                    // A person who may not book on this issue (a Contributor outside their
                    // project) tries another one.
                    Err(AppError::Refused(_)) => {}
                    Err(error) => return Err(SampleError::step(Step::Time, &error)),
                }
            }
            if !booked {
                return Err(SampleError::NotMade("a time entry".to_owned()));
            }
        }
        Ok(())
    }

    /// Days of the past that people changed afterwards, with a reason: first by the person, then
    /// by somebody with the role *HR*. Each is a real past day, so the reason is asked for.
    async fn change_days(
        &mut self,
        plan: &TimePlan,
        hands: &mut Hands<'_, '_>,
        random: &mut SeededRandom,
        worked: &[Worked],
        by_person: u64,
        by_hr: u64,
    ) -> Result<(), SampleError> {
        if worked.is_empty() {
            return Ok(());
        }
        // The older half of the days: a day of the past, whichever number of weeks there are.
        let mut candidates = worked.iter().take(worked.len() / 2);
        let hr = self
            .world
            .people
            .iter()
            .position(|made| made.person.role == StartingRole::Hr)
            .ok_or(SampleError::PersonMissing)?;
        for (count, hr_changes) in [(by_person, false), (by_hr, true)] {
            for _ in 0..count {
                let day = candidates
                    .next()
                    .ok_or_else(|| SampleError::NotMade("a day to change".to_owned()))?;
                let (reasons, changer) = if hr_changes {
                    (&plan.hr_reasons, hr)
                } else {
                    (&plan.person_reasons, day.person)
                };
                let reason = reasons[random.below(reasons.len().max(1))].clone();
                self.change_day(hands, *day, changer, hr_changes, reason)
                    .await?;
            }
        }
        Ok(())
    }

    /// One day made an hour longer at its end, with a reason.
    async fn change_day(
        &mut self,
        hands: &mut Hands<'_, '_>,
        day: Worked,
        changer: usize,
        of_somebody_else: bool,
        reason: String,
    ) -> Result<(), SampleError> {
        let owner = self.world.people[day.person].id;
        let stored = self
            .world
            .storage
            .work_day(owner, day.date)
            .await
            .map_err(|error| SampleError::step(Step::Time, &error))?
            .ok_or_else(|| SampleError::NotMade(format!("the work day of {}", day.date)))?;
        let mut periods = stored.times.periods.clone();
        if let Some(Span { end: Some(end), .. }) = periods.last_mut() {
            *end = end
                .checked_add(SignedDuration::from_secs(
                    LATER_END_MINUTES * SECONDS_PER_MINUTE,
                ))
                .map_err(|_| SampleError::NotMade("a later end".to_owned()))?;
        }
        let token = hands.token(changer).await?;
        self.world.tick();
        change_work_day(
            &self.world.services.ctx(self.world.request()),
            ChangeWorkDay {
                session_token: token,
                person: of_somebody_else.then_some(owner),
                date: day.date,
                version: stored.day.version,
                periods,
                breaks: stored.times.breaks.clone(),
                reason: Some(reason),
                device_zone: None,
            },
        )
        .await
        .map_err(|error| SampleError::step(Step::Time, &error))?;
        Ok(())
    }

    /// A few people leave their timer running on an issue.
    async fn leave_timers(
        &mut self,
        hands: &mut Hands<'_, '_>,
        random: &mut SeededRandom,
        employees: &[usize],
        count: u64,
    ) -> Result<(), SampleError> {
        let wanted = usize::try_from(count).unwrap_or(usize::MAX);
        let mut started = 0_usize;
        let mut tries = 0_usize;
        // A Team member, so the issue is one the person may book on.
        let team: Vec<usize> = employees
            .iter()
            .copied()
            .filter(|person| self.world.people[*person].person.role == StartingRole::TeamMember)
            .collect();
        while started < wanted && tries < wanted * TRIES_PER_ENTRY {
            tries += 1;
            let person = team[started % team.len().max(1)];
            let at = random.below(self.issues.len());
            let Some(Some(issue)) = self.issues.get(at) else {
                continue;
            };
            let id = issue.id;
            let token = hands.token(person).await?;
            self.world.tick();
            match start_timer(
                &self.world.services.ctx(self.world.request()),
                StartTimer {
                    session_token: token,
                    issue: id.to_string(),
                },
            )
            .await
            {
                Ok(_) => started += 1,
                Err(AppError::Refused(_)) => {}
                Err(error) => return Err(SampleError::step(Step::Time, &error)),
            }
        }
        if started < wanted {
            return Err(SampleError::NotMade("a running timer".to_owned()));
        }
        Ok(())
    }
}

/// Gives the events that book time their share of the entries: the same number for each, the
/// rest to the first ones, so the sum is the number the plan says.
fn spread(events: &mut [Event], entries: u64) {
    let booking: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, event)| matches!(event.happens, Happens::Books(_)))
        .map(|(at, _)| at)
        .collect();
    let days = u64::try_from(booking.len()).unwrap_or(1).max(1);
    for (place, at) in booking.into_iter().enumerate() {
        let place = u64::try_from(place).unwrap_or(0);
        let share = entries / days + u64::from(place < entries % days);
        events[at].happens = Happens::Books(usize::try_from(share).unwrap_or(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn booking(count: usize) -> Vec<Event> {
        (0..count)
            .map(|at| Event {
                moment: i64::try_from(at).unwrap_or(0),
                person: 0,
                date: Date::constant(2027, 1, 4),
                happens: Happens::Books(1),
            })
            .collect()
    }

    fn total(events: &[Event]) -> usize {
        events
            .iter()
            .map(|event| match event.happens {
                Happens::Books(count) => count,
                _ => 0,
            })
            .sum()
    }

    #[test]
    fn the_entries_are_shared_out_and_add_up_to_the_plan() {
        for (days, entries) in [(7, 20), (10, 10), (3, 1000), (5, 2)] {
            let mut events = booking(days);
            spread(&mut events, entries);
            assert_eq!(total(&events) as u64, entries, "{days} {entries}");
        }
    }

    #[test]
    fn a_time_of_day_is_a_moment_in_the_zone_of_the_person() {
        let date = Date::constant(2027, 1, 4);
        let utc = moment_of(date, &TimeZone::UTC, 8 * 60).unwrap();
        let berlin = moment_of(date, &TimeZone::get("Europe/Berlin").unwrap(), 8 * 60).unwrap();
        assert_eq!(utc - berlin, SECONDS_PER_HOUR);
        assert_eq!(date_of(0).unwrap(), Date::constant(1970, 1, 1));
        assert_eq!(date_of(1).unwrap(), Date::constant(1970, 1, 2));
    }
}
