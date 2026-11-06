//! Every change of a work day (TIME-08, TIME-10, ADR-0035, decisions 5 to 7): the taps *Start work
//! day*, *Start break*, *End break* and *End work day*, which record the moment of the tap, and the
//! times typed afterwards, one's own or another person's.
//!
//! Each change is one change of the store with its event. The event keeps the times before and
//! after, and the reason when one was given, so the history of the day shows the old value; a
//! change by somebody else tells the person (the reader of the outbox, `notifications`). Inside the
//! change, after the lock of the person's days, the day is checked on what is stored: the version,
//! the limit of changes a second, the shape of the day and that it overlaps neither the day before
//! nor the day after it. The working-time rules are checked there too, for the day and the day after
//! it, with the rules in force at that change, and stored with the days. A rule never refuses
//! anything (TIME-09).

use bb_core::event::{DomainEvent, FieldChange, RecordKind};
use bb_core::id::{UserId, WorkDayId};
use bb_core::time::day::date_of;
use bb_core::time::zone;
use bb_core::version::{FIELD as VERSION, FIRST_VERSION, counted_up, next};
use bb_core::wording::Region;
use bb_core::working_time::record::{StoredDay, WorkDay};
use bb_core::working_time::{Rules, Times, is_past, warnings};
use bb_core::{ErrorCode, time::day::DaySpan};
use bb_policy::{Authorized, Place};
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use serde_json::{Value, json};

use super::day::{check, in_order, not_in_the_future, started, tapped};
use super::types::{ChangeWorkDay, ChangedWorkDay, StartWorkDay, Tap, TapWorkDay};
use super::whose::Whose;
use super::{CHANGES_PER_SECOND, REASON_CHARS_MAX};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Stops, by, change_failed, entity, internal, set_to};
use crate::editing::Stale;
use crate::input::clean_text;
use crate::member::work_day as member;
use crate::ports::BoxFuture;
use crate::projects::Why;

/// One change of a work day, ready to be written.
struct Plan {
    /// Whose day.
    person: UserId,
    /// Its date.
    date: Date,
    /// The zone it started in, by name.
    zone: String,
    /// The day as it was read before the change; none for a new day.
    stored: Option<StoredDay>,
    /// The times after the change, in order.
    times: Times,
    /// Why, in the words of whoever changes it.
    reason: Option<String>,
}

/// What a written change is answered with: why it stopped, if it did, and what the answer needs.
struct Finish {
    why: Why,
    viewer: UserId,
    stored: Option<StoredDay>,
    now: Timestamp,
}

impl Finish {
    /// The day as it is now, or what refused the change.
    fn answer<E>(
        self,
        written: Result<StoredDay, E>,
        failed: impl FnOnce(&E) -> AppError,
        whose: &Whose,
    ) -> Result<Authorized<ChangedWorkDay>, AppError> {
        match written {
            Ok(day) => Ok(whose.evaluation.authorize(ChangedWorkDay {
                day: super::read::view_of(&day, self.now, false),
            })),
            Err(error) => Err(match self.why.said() {
                Some(ErrorCode::VersionChanged) => stale(self.viewer, self.stored.as_ref()),
                Some(code) => AppError::Refused(code),
                None => failed(&error),
            }),
        }
    }
}

/// The times of a day as they go into its events.
fn json_of(times: &Times) -> Value {
    json!({ "periods": times.periods, "breaks": times.breaks })
}

/// The shortest time between two changes of one person's days.
const fn shortest_gap() -> SignedDuration {
    SignedDuration::from_millis(1000 / CHANGES_PER_SECOND)
}

/// The zone a stored day started in; UTC for a name the zone data no longer has.
fn zone_of(name: &str) -> TimeZone {
    zone::named(name).unwrap_or(TimeZone::UTC)
}

/// The zone of a name of the zone data, and the name; `None` for one the server does not know.
fn named(name: &str) -> Option<(String, TimeZone)> {
    zone::named(name).map(|zone| (name.to_owned(), zone))
}

/// The reason as it is kept: trimmed, at most [`REASON_CHARS_MAX`] characters, none when empty.
fn reason_of(text: Option<&str>) -> Result<Option<String>, AppError> {
    match text.map(str::trim) {
        None | Some("") => Ok(None),
        Some(text) => clean_text(text, REASON_CHARS_MAX).map(Some),
    }
}

/// The answer to a change made from a version the day no longer has: the structured conflict for
/// a stored day, the plain refusal for a day that was new to the person.
fn stale(viewer: UserId, stored: Option<&StoredDay>) -> AppError {
    match stored {
        Some(stored) => AppError::Stale(Box::new(Stale {
            record: entity(RecordKind::WorkDay, stored.day.id),
            place: Place::Organization,
            viewer,
        })),
        None => AppError::Refused(ErrorCode::VersionChanged),
    }
}

/// What a change does inside the store, after the lock of the person's days: checks the day on
/// what is stored, writes it with its warnings and the warnings of the day after it.
fn writing<S: AuthStore>(
    plan: Plan,
    id: WorkDayId,
    zone: TimeZone,
    now: Timestamp,
    why: Why,
) -> impl for<'t> FnOnce(&'t mut S::Transaction) -> BoxFuture<'t, Result<StoredDay, S::Error>>
+ Send
+ 'static {
    let expected = plan.stored.as_ref().map(|stored| stored.day.version);
    let Plan {
        person,
        date,
        zone: zone_name,
        times,
        ..
    } = plan;
    move |transaction: &mut S::Transaction| {
        Box::pin(async move {
            S::lock_work_days_of(transaction, person).await?;
            let current = S::work_day_in(transaction, person, date).await?;
            if current.as_ref().map(|found| found.day.version) != expected {
                return Err(why.stop::<S>(ErrorCode::VersionChanged));
            }
            let last = S::last_work_day_change_in(transaction, person).await?;
            if last.is_some_and(|last| now.duration_since(last) < shortest_gap()) {
                return Err(why.stop::<S>(ErrorCode::LimitReached));
            }
            let before = S::work_day_before_in(transaction, person, date).await?;
            let after = S::work_day_after_in(transaction, person, date).await?;
            let before_times = before.as_ref().map(|found| &found.times);
            let after_times = after.as_ref().map(|found| &found.times);
            let checked = check(&times, date, &zone, before_times, after_times)
                .and_then(|()| not_in_the_future(&times, now));
            if let Err(code) = checked {
                return Err(why.stop::<S>(code));
            }
            let rules = Rules::of(&S::setting_layers_in(transaction, None, None).await?);
            let day = StoredDay {
                day: WorkDay {
                    id,
                    person,
                    date,
                    zone: zone_name,
                    warnings: warnings(&times, before_times, rules.as_ref()),
                    updated_at: now,
                    version: expected.map_or(FIRST_VERSION, next),
                },
                times,
            };
            match expected {
                Some(version) => {
                    if !S::update_work_day(transaction, &day, version).await? {
                        return Err(why.stop::<S>(ErrorCode::VersionChanged));
                    }
                }
                None => S::insert_work_day(transaction, &day).await?,
            }
            if let Some(next_day) = after {
                let found = warnings(&next_day.times, Some(&day.times), rules.as_ref());
                S::set_work_day_warnings(transaction, next_day.day.id, &found).await?;
            }
            Ok(day)
        })
    }
}

/// The change of a plan, in the form every command writes it: the event, what the store does, and
/// what the answer needs.
type Planned<S> = (
    DomainEvent,
    Box<
        dyn for<'t> FnOnce(
                &'t mut <S as crate::ports::ChangeStore>::Transaction,
            ) -> BoxFuture<
                't,
                Result<StoredDay, <S as crate::ports::ChangeStore>::Error>,
            > + Send,
    >,
    Finish,
);

impl<S: AuthStore> Ctx<'_, S> {
    /// The zone of a new day of `person`: the one they chose under *Region*, or the
    /// organization's, else the zone of the device when it is one the server knows, else UTC.
    async fn zone_for(
        &self,
        person: UserId,
        device: Option<&str>,
    ) -> Result<(String, TimeZone), AppError> {
        let layers = self.setting_layers_of(Some(person)).await?;
        let region = Region::of(&layers);
        let chosen = region
            .zone()
            .iana_name()
            .and_then(named)
            .filter(|(name, _)| name != zone::UTC);
        Ok(chosen
            .or_else(|| device.and_then(named))
            .unwrap_or_else(|| (zone::UTC.to_owned(), TimeZone::UTC)))
    }

    /// The person's last day, if a period of it runs.
    async fn open_day(&self, person: UserId) -> Result<Option<StoredDay>, AppError> {
        self.store()
            .open_work_day(person)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// The plan of a tap on the person's day that runs, when it is at the version the person
    /// looked at.
    async fn tap_plan(
        &self,
        actor: &Actor,
        version: i64,
        tap: Tap,
    ) -> Result<(Whose, Plan), AppError> {
        let whose = self.days_to_change(actor, None).await?;
        let day = self
            .open_day(whose.person)
            .await?
            .ok_or(ErrorCode::WorkDayNotRunning)?;
        if day.day.version != version {
            return Err(stale(whose.person, Some(&day)));
        }
        let times = tapped(&day.times, tap, self.now())?;
        let plan = Plan {
            person: whose.person,
            date: day.day.date,
            zone: day.day.zone.clone(),
            stored: Some(day),
            times,
            reason: None,
        };
        Ok((whose, plan))
    }

    /// The event of a plan, the change that writes it, and what its answer needs.
    fn planned(&self, actor: &Actor, plan: Plan) -> Result<Planned<S>, AppError> {
        let now = self.now();
        let zone = zone::named(&plan.zone).ok_or(ErrorCode::InvalidRequest)?;
        let (id, version) = match &plan.stored {
            Some(stored) => (stored.day.id, Some(stored.day.version)),
            None => (
                WorkDayId::generate(&mut self.ids())
                    .map_err(|error| AppError::Internal(error.to_string()))?,
                None,
            ),
        };
        let changes = vec![
            set_to(member::PERSON, json!(plan.person.to_string())),
            set_to(member::DATE, json!(plan.date.to_string())),
            FieldChange {
                field: member::TIMES.to_owned(),
                before: plan.stored.as_ref().map(|stored| json_of(&stored.times)),
                after: Some(json_of(&plan.times)),
            },
            version.map_or_else(|| set_to(VERSION, json!(FIRST_VERSION)), counted_up),
        ];
        let mut event = self.event(
            by(actor.person.id),
            entity(RecordKind::WorkDay, id),
            changes,
        )?;
        event.reason.clone_from(&plan.reason);
        let why = Why::default();
        let finish = Finish {
            why: why.clone(),
            viewer: actor.person.id,
            stored: plan.stored.clone(),
            now,
        };
        Ok((
            event,
            Box::new(writing::<S>(plan, id, zone, now, why)),
            finish,
        ))
    }
}

/// What a failed change of the store answers when it did not say why itself.
fn failed<S: AuthStore>(error: &S::Error) -> AppError {
    change_failed::<S>(error, Stops::both(ErrorCode::VersionChanged))
}

/// *Start work day*: a new day for the person's day of now, or a new period of it when it was
/// ended already. A day that was left open on another date is never ended here: the answer names
/// it, and the person says when it ended first (ADR-0035, decision 5).
///
/// # Errors
/// `working-hours.day-left-open` (with the date of that day) and `working-hours.day-running`.
pub async fn start_work_day<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: StartWorkDay,
) -> Result<Authorized<ChangedWorkDay>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let whose = ctx.days_to_change(&actor, None).await?;
    let person = whose.person;
    let now = ctx.now();
    if let Some(open) = ctx.open_day(person).await? {
        let zone = zone_of(&open.day.zone);
        let runs_today = DaySpan::of(open.day.date, &zone).is_some_and(|span| span.holds(now));
        return Err(if runs_today {
            ErrorCode::WorkDayRunning.into()
        } else {
            AppError::DayLeftOpen {
                date: open.day.date,
            }
        });
    }
    let (zone_name, zone) = ctx.zone_for(person, input.device_zone.as_deref()).await?;
    let date = date_of(now, &zone);
    let stored = ctx
        .store()
        .work_day(person, date)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let plan = Plan {
        person,
        date,
        zone: stored
            .as_ref()
            .map_or(zone_name, |stored| stored.day.zone.clone()),
        times: started(stored.as_ref().map(|stored| &stored.times), now),
        stored,
        reason: None,
    };
    let (event, change, finish) = ctx.planned(&actor, plan)?;
    let written = ctx.store().record_change(&event, change).await;
    finish.answer(written, failed::<S>, &whose)
}

/// A tap on the person's day that runs: *Start break*, *End break* or *End work day*, which ends
/// the break that runs too.
///
/// # Errors
/// `working-hours.day-not-running`, `working-hours.break-running`, `working-hours.no-break-running`,
/// `limit.reached` for one break too many, `working-hours.not-a-work-day` when the day would last
/// longer than a work day may (the person says when it really ended instead), and the structured
/// conflict for a version the day no longer has.
pub async fn tap_work_day<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: TapWorkDay,
) -> Result<Authorized<ChangedWorkDay>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let (whose, plan) = ctx.tap_plan(&actor, input.version, input.tap).await?;
    let (event, change, finish) = ctx.planned(&actor, plan)?;
    let written = ctx.store().record_change(&event, change).await;
    finish.answer(written, failed::<S>, &whose)
}

/// New times of a work day, typed afterwards: one's own, or another person's with *Change working
/// hours of others*. A change of another person's day, or of one's own past day, needs a reason,
/// and a day left open is ended this way (ADR-0035, decision 7).
///
/// # Errors
/// `working-hours.reason-needed`, `working-hours.not-a-work-day`, `limit.reached`, the refusals of
/// [`Ctx::days_to_change`], and the structured conflict for a version the day no longer has.
pub async fn change_work_day<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeWorkDay,
) -> Result<Authorized<ChangedWorkDay>, AppError> {
    let actor = ctx.asking(&input.session_token, input.person).await?;
    let whose = ctx.days_to_change(&actor, input.person).await?;
    let reason = reason_of(input.reason.as_deref())?;
    let stored = ctx
        .store()
        .work_day(whose.person, input.date)
        .await
        .map_err(|error| internal::<S>(&error))?;
    if stored.as_ref().map_or(0, |stored| stored.day.version) != input.version {
        return Err(stale(actor.person.id, stored.as_ref()));
    }
    let (zone_name, zone) = match &stored {
        Some(stored) => (stored.day.zone.clone(), zone_of(&stored.day.zone)),
        None => {
            ctx.zone_for(whose.person, input.device_zone.as_deref())
                .await?
        }
    };
    if reason.is_none() && (!whose.own || is_past(input.date, &zone, ctx.now())) {
        return Err(ErrorCode::ReasonNeeded.into());
    }
    let plan = Plan {
        person: whose.person,
        date: input.date,
        zone: zone_name,
        stored,
        times: in_order(Times {
            periods: input.periods,
            breaks: input.breaks,
        }),
        reason,
    };
    let (event, change, finish) = ctx.planned(&actor, plan)?;
    let written = ctx.store().record_change(&event, change).await;
    finish.answer(written, failed::<S>, &whose)
}
