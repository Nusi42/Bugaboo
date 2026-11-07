//! The timer (TIME-01, ADR-0035 decision 3): one per person, kept on the server, so every device
//! shows the same timer and a closed browser does not stop it.
//!
//! *Start* (on an issue the person sees and may book on), *Stop* (saves an entry), *Switch issue*
//! (saves the time so far with the defaults of its issue and starts on the new one), *Change start
//! time* (never in the future, never before the end of the person's last entry with a start) and
//! *Throw away* (saves nothing). A timer is saved as whole minutes, rounded down; under a minute
//! nothing is saved. Every change is made on the version that was looked at, with its event; the
//! store holds one timer row for each person, so two devices that start at once get one timer and
//! one refusal.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::{FieldId, UserId};
use bb_core::time::day::date_of;
use bb_core::time::entry::{Running, Timer};
use bb_core::time::{Minutes, TIME_ENTRY_MINUTES_MAX, TIME_ENTRY_MINUTES_MIN};
use bb_core::version::{FIRST_VERSION, Version, counted_up, next};
use bb_core::wording::Region;
use bb_policy::Authorized;
use jiff::Timestamp;
use jiff::civil::Date;
use serde_json::{Value, json};

use super::entries::{EntryTime, NewEntry, failed_change, timer_stale};
use super::limits::{long_timer_at, may_change_timer};
use super::types::{
    ChangeStartTime, EntryDetails, RunningView, StartTimer, StopTimer, SwitchTimer, ThrowAwayTimer,
    TimerQuery, TimerSaved, TimerView,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::jobs::JobKind;
use crate::member::{time_entry as entry_member, timer as member};
use crate::projects::Why;

/// What the job of a long timer is told: whose timer, and the start it was queued for.
pub(crate) mod payload {
    /// The person whose timer it is.
    pub(crate) const PERSON: &str = "person";
    /// The start of the timer the job was queued for.
    pub(crate) const STARTED_AT: &str = "started_at";
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The timer of a person, running or not.
    async fn timer_of(&self, person: UserId) -> Result<Option<Timer>, AppError> {
        self.store()
            .timer(person)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// The running timer of the actor at the version they looked at: `access.forbidden` for a key
    /// that may only look, `timer.not-running` when it does not run, `edit.version-changed` when
    /// it changed since.
    async fn running_timer(
        &self,
        actor: &Actor,
        version: i64,
    ) -> Result<(Timer, Running), AppError> {
        actor.may_change_own()?;
        let timer = self.timer_of(actor.person.id).await?;
        let Some((timer, running)) =
            timer.and_then(|timer| timer.running().copied().map(|running| (timer, running)))
        else {
            return Err(ErrorCode::TimerNotRunning.into());
        };
        if timer.version != version {
            return Err(timer_stale(actor));
        }
        Ok((timer, running))
    }

    /// The date a moment is on for a person (their day of it, in their time zone), with the
    /// person's *Region* it was worked out by.
    pub(super) async fn day_of(
        &self,
        person: UserId,
        moment: Timestamp,
    ) -> Result<(Date, Region), AppError> {
        let region = Region::of(&self.setting_layers_of(Some(person)).await?);
        Ok((date_of(moment, region.zone()), region))
    }

    /// The marks of the starter fields with the own field each marks, by the code of the mark.
    async fn starter_keys(&self) -> Result<BTreeMap<String, FieldId>, AppError> {
        Ok(self
            .store()
            .starter_fields()
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|mark| (mark.starter.code().to_owned(), mark.field))
            .collect())
    }

    /// The timer as its person sees it.
    async fn timer_view(
        &self,
        actor: &Actor,
        timer: Option<&Timer>,
    ) -> Result<TimerView, AppError> {
        let running = match timer.and_then(Timer::running) {
            Some(running) => {
                let readable_id = self.readable_id_for(actor, running.issue).await?;
                // An access key that does not see the issue is not told which one it is.
                let issue = (actor.key.is_none() || readable_id.is_some()).then_some(running.issue);
                Some(RunningView {
                    issue,
                    readable_id,
                    started_at: running.started_at,
                    runs_long: running.warned_at.is_some(),
                })
            }
            None => None,
        };
        Ok(TimerView {
            running,
            version: timer.map_or(0, |timer| timer.version),
            starters: self.starter_keys().await?,
        })
    }

    /// The job that tells the person when the timer of `person` that started at `started_at` runs
    /// long.
    fn long_timer_job(
        &self,
        person: UserId,
        started_at: Timestamp,
    ) -> Result<LongTimerJob, AppError> {
        Ok(LongTimerJob {
            id: bb_core::id::JobId::generate(&mut self.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?,
            kind: JobKind::TimersLongRunning.name()?,
            payload: json!({
                payload::PERSON: person.to_string(),
                payload::STARTED_AT: started_at.to_string(),
            }),
            run_at: long_timer_at(started_at)?,
        })
    }
}

/// A job of a long timer, ready to be queued inside the change that starts the timer.
struct LongTimerJob {
    id: bb_core::id::JobId,
    kind: bb_core::technical::TechnicalName,
    payload: Value,
    run_at: Timestamp,
}

impl LongTimerJob {
    /// Queues the job inside the change.
    async fn queue<S: AuthStore>(
        &self,
        transaction: &mut S::Transaction,
        now: Timestamp,
    ) -> Result<(), S::Error> {
        S::enqueue_job(
            transaction,
            self.id,
            &self.kind,
            &self.payload,
            self.run_at,
            now,
        )
        .await
    }
}

/// Writes the timer of a person that is still at `version`; one that changed meanwhile stops the
/// change with `edit.version-changed`.
async fn write_timer<S: AuthStore>(
    transaction: &mut S::Transaction,
    timer: &Timer,
    version: i64,
    why: &Why,
) -> Result<(), S::Error> {
    if S::put_timer(transaction, timer, Some(Version::new(version))).await? {
        Ok(())
    } else {
        Err(why.stop::<S>(ErrorCode::VersionChanged))
    }
}

/// The change of a timer that stands still from now on: it ran on `running`'s issue.
fn issue_cleared(running: &Running) -> FieldChange {
    FieldChange {
        field: member::ISSUE.to_owned(),
        before: Some(json!(running.issue.to_string())),
        after: None,
    }
}

/// The changes of the timer inside the event of the entry a stop or a switch saved, named as the
/// timer's own (`timer_…`): so they do not read as the entry's (a `version` there would say the
/// new entry counted up). Each member is named here; one that is not is a defect of the program,
/// not a member to guess a name for.
fn as_the_timers(changes: Vec<FieldChange>) -> Result<Vec<FieldChange>, AppError> {
    changes
        .into_iter()
        .map(|mut change| {
            let name = match change.field.as_str() {
                member::ISSUE | entry_member::TIMER_ISSUE => entry_member::TIMER_ISSUE,
                member::STARTED_AT => entry_member::TIMER_STARTED_AT,
                bb_core::version::FIELD => entry_member::TIMER_VERSION,
                other => {
                    return Err(AppError::Internal(format!(
                        "the timer's member `{other}` has no name in the event of an entry"
                    )));
                }
            };
            name.clone_into(&mut change.field);
            Ok(change)
        })
        .collect()
}

/// The whole minutes a timer that started at `started_at` ran until `now`, rounded down.
fn ran(started_at: Timestamp, now: Timestamp) -> Minutes {
    Minutes::from_seconds(now.duration_since(started_at).as_secs()).unwrap_or_default()
}

/// The timer of the person who asks.
///
/// # Errors
/// `auth.not-signed-in` without a session.
pub async fn timer<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: TimerQuery,
) -> Result<Authorized<TimerView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let timer = ctx.timer_of(actor.person.id).await?;
    let view = ctx.timer_view(&actor, timer.as_ref()).await?;
    ctx.own_answer(&actor, view).await
}

/// What starting the timer comes to, judged and not yet written.
struct StartPlan {
    started: Timer,
    before: Option<Version>,
    changes: Vec<FieldChange>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Judges a start of the timer on an issue: the issue is one the actor sees and may book on, and
    /// the timer does not run. The one judgment [`start_timer`] writes and its preview answers.
    async fn plan_timer_start(&self, actor: &Actor, address: &str) -> Result<StartPlan, AppError> {
        let (issue, _) = self.bookable_issue(actor, address).await?;
        let now = self.now();
        let current = self.timer_of(actor.person.id).await?;
        if let Some(timer) = &current {
            if timer.running.is_some() {
                return Err(ErrorCode::TimerRunning.into());
            }
            may_change_timer(timer.changed_at, now)?;
        }
        let started = Timer {
            person: actor.person.id,
            running: Some(Running {
                issue: issue.id,
                started_at: now,
                warned_at: None,
            }),
            changed_at: now,
            version: current
                .as_ref()
                .map_or(FIRST_VERSION, |timer| next(timer.version)),
        };
        let mut changes = vec![
            set_to(member::ISSUE, json!(issue.id.to_string())),
            set_to(member::STARTED_AT, json!(now.to_string())),
        ];
        let before = current.as_ref().map(|timer| Version::new(timer.version));
        if let Some(timer) = &current {
            changes.push(counted_up(timer.version));
        }
        Ok(StartPlan {
            started,
            before,
            changes,
        })
    }

    /// What starting the timer on an issue would record, answered without saving (AI-21).
    ///
    /// # Errors
    /// As [`start_timer`].
    pub(crate) async fn preview_start_timer(
        &self,
        actor: &Actor,
        input: &StartTimer,
    ) -> Result<Vec<FieldChange>, AppError> {
        Ok(self.plan_timer_start(actor, &input.issue).await?.changes)
    }

    /// What stopping the timer would record, answered without saving (AI-21).
    ///
    /// # Errors
    /// As [`stop_timer`].
    pub(crate) async fn preview_stop_timer(
        &self,
        actor: &Actor,
        input: &StopTimer,
    ) -> Result<Vec<FieldChange>, AppError> {
        let (timer, running) = self.running_timer(actor, input.version).await?;
        let saving = self
            .saving(actor, &running, self.now(), &input.details)
            .await?;
        let changes = vec![issue_cleared(&running), counted_up(timer.version)];
        stop_changes(&saving, changes)
    }
}

/// Starts the person's timer on an issue they see and may book on.
///
/// # Errors
/// `resource.not-found` for an issue the person does not see, `access.forbidden` without *Book
/// time*, `timer.running` when their timer runs (also when another device started it a moment
/// before), `timer.too-fast` within a second of the last start or switch.
pub async fn start_timer<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: StartTimer,
) -> Result<Authorized<TimerView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let StartPlan {
        started,
        before,
        changes,
    } = ctx.plan_timer_start(&actor, &input.issue).await?;
    let now = started.changed_at;
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Timer, actor.person.id),
        changes,
    )?;
    let job = ctx.long_timer_job(actor.person.id, now)?;
    let why = Why::default();
    let (stopped, written) = (why.clone(), started.clone());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if !S::put_timer(transaction, &written, before).await? {
                    return Err(stopped.stop::<S>(ErrorCode::TimerRunning));
                }
                job.queue::<S>(transaction, now).await
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::TimerRunning))?;
    let view = ctx.timer_view(&actor, Some(&started)).await?;
    ctx.own_answer(&actor, view).await
}

/// What saving the time of a running timer comes to: the entry, or none for less than a minute.
struct Saving {
    entry: Option<NewEntry>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The entry the time of `running` saves at `now`, with `details` filled in: none for less than
    /// a minute.
    ///
    /// # Errors
    /// `timer.too-long` for a timer that ran longer than one entry can be, the refusals of a new
    /// entry, and those of an issue the person no longer sees or may not book on.
    async fn saving(
        &self,
        actor: &Actor,
        running: &Running,
        now: Timestamp,
        details: &EntryDetails,
    ) -> Result<Saving, AppError> {
        let minutes = ran(running.started_at, now);
        if minutes > TIME_ENTRY_MINUTES_MAX {
            return Err(ErrorCode::TimerTooLong.into());
        }
        if minutes < TIME_ENTRY_MINUTES_MIN {
            return Ok(Saving { entry: None });
        }
        let (issue, held) = self
            .bookable_issue(actor, &running.issue.to_string())
            .await?;
        let (date, _) = self.day_of(actor.person.id, running.started_at).await?;
        let entry = self
            .new_entry(
                actor,
                (&issue, &held),
                EntryTime {
                    date,
                    minutes,
                    started_at: Some(running.started_at),
                },
                details,
            )
            .await?;
        Ok(Saving { entry: Some(entry) })
    }

    /// The answer of a stop or a switch: the timer as it is now and the entry that was saved.
    async fn saved(
        &self,
        actor: &Actor,
        timer: &Timer,
        entry: Option<&bb_core::time::entry::TimeEntry>,
    ) -> Result<Authorized<TimerSaved>, AppError> {
        let entry = match entry {
            Some(entry) => Some(self.entry_view(actor, entry).await?),
            None => None,
        };
        let timer = self.timer_view(actor, Some(timer)).await?;
        self.own_answer(actor, TimerSaved { timer, entry }).await
    }
}

/// The event of a change of the timer that saves an entry: the entry's, with the change of the
/// timer in it; or the timer's alone when nothing is saved.
fn saving_event<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    actor: &Actor,
    saving: &Saving,
    timer_changes: Vec<FieldChange>,
) -> Result<bb_core::event::DomainEvent, AppError> {
    let changes = stop_changes(saving, timer_changes)?;
    match &saving.entry {
        Some(made) => ctx.event(
            by(actor.person.id),
            entity(RecordKind::TimeEntry, made.entry.id),
            changes,
        ),
        None => ctx.event(
            by(actor.person.id),
            entity(RecordKind::Timer, actor.person.id),
            changes,
        ),
    }
}

/// The changes of a stop: the entry's, with the change of the timer in it; or the timer's alone
/// when nothing is saved.
fn stop_changes(
    saving: &Saving,
    timer_changes: Vec<FieldChange>,
) -> Result<Vec<FieldChange>, AppError> {
    match &saving.entry {
        Some(made) => {
            let mut changes = made.changes();
            changes.extend(as_the_timers(timer_changes)?);
            Ok(changes)
        }
        None => Ok(timer_changes),
    }
}

/// Stops the timer and saves the time as an entry with what the person filled in; under a minute
/// nothing is saved and the answer has no entry.
///
/// # Errors
/// `timer.not-running`, `edit.version-changed` for a timer that changed since it was read,
/// `timer.too-long` for one that ran longer than an entry can be (change the start time first), the
/// refusals of a new entry.
pub async fn stop_timer<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: StopTimer,
) -> Result<Authorized<TimerSaved>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (timer, running) = ctx.running_timer(&actor, input.version).await?;
    let now = ctx.now();
    let saving = ctx.saving(&actor, &running, now, &input.details).await?;
    let stopped = Timer {
        running: None,
        version: next(timer.version),
        ..timer.clone()
    };
    let changes = vec![issue_cleared(&running), counted_up(timer.version)];
    let event = saving_event(ctx, &actor, &saving, changes)?;
    let made = saving.entry.as_ref().map(|made| made.entry.clone());
    let why = Why::default();
    let (halted, written) = (why.clone(), stopped.clone());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                write_timer::<S>(transaction, &written, timer.version, &halted).await?;
                match &saving.entry {
                    Some(entry) => entry.write::<S>(transaction, &halted).await,
                    None => Ok(()),
                }
            })
        })
        .await
        .map_err(|error| failed_change::<S>(&why, &error, || timer_stale(&actor)))?;
    ctx.saved(&actor, &stopped, made.as_ref()).await
}

/// Saves the time so far as an entry with the defaults of its issue, and starts the timer on
/// another issue. Under a minute nothing is saved and the new timer starts. A required field
/// without a default on the issue refuses the switch at that field, so the screen asks for it and
/// switches again with what the person filled in.
///
/// # Errors
/// `timer.not-running`, `edit.version-changed`, `timer.too-fast` within a second of the last start
/// or switch, `timer.too-long`, the refusals of the new issue and of a new entry.
pub async fn switch_timer<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SwitchTimer,
) -> Result<Authorized<TimerSaved>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (timer, running) = ctx.running_timer(&actor, input.version).await?;
    let now = ctx.now();
    may_change_timer(timer.changed_at, now)?;
    let (issue, _) = ctx.bookable_issue(&actor, &input.issue).await?;
    let saving = ctx.saving(&actor, &running, now, &input.details).await?;
    let switched = Timer {
        running: Some(Running {
            issue: issue.id,
            started_at: now,
            warned_at: None,
        }),
        changed_at: now,
        version: next(timer.version),
        ..timer.clone()
    };
    let changes = vec![
        FieldChange {
            field: entry_member::TIMER_ISSUE.to_owned(),
            before: Some(json!(running.issue.to_string())),
            after: Some(json!(issue.id.to_string())),
        },
        set_to(member::STARTED_AT, json!(now.to_string())),
        counted_up(timer.version),
    ];
    let event = saving_event(ctx, &actor, &saving, changes)?;
    let made = saving.entry.as_ref().map(|made| made.entry.clone());
    let job = ctx.long_timer_job(actor.person.id, now)?;
    let why = Why::default();
    let (halted, written) = (why.clone(), switched.clone());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                write_timer::<S>(transaction, &written, timer.version, &halted).await?;
                if let Some(entry) = &saving.entry {
                    entry.write::<S>(transaction, &halted).await?;
                }
                job.queue::<S>(transaction, now).await
            })
        })
        .await
        .map_err(|error| failed_change::<S>(&why, &error, || timer_stale(&actor)))?;
    ctx.saved(&actor, &switched, made.as_ref()).await
}

/// Moves the start of the running timer: never into the future, never before the end of the
/// person's last entry that has a start (entries made by hand do not count).
///
/// # Errors
/// `timer.not-running`, `edit.version-changed`, `timer.start-in-the-future`,
/// `timer.start-before-last-entry`.
pub async fn change_start_time<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeStartTime,
) -> Result<Authorized<TimerView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (timer, running) = ctx.running_timer(&actor, input.version).await?;
    let now = ctx.now();
    if input.started_at > now {
        return Err(ErrorCode::TimerStartInFuture.into());
    }
    let last_end = ctx
        .store()
        .last_timed_end(actor.person.id)
        .await
        .map_err(|error| internal::<S>(&error))?;
    if last_end.is_some_and(|end| input.started_at < end) {
        return Err(ErrorCode::TimerStartBeforeLastEntry.into());
    }
    let moved = Timer {
        running: Some(Running {
            started_at: input.started_at,
            ..running
        }),
        version: next(timer.version),
        ..timer.clone()
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Timer, actor.person.id),
        vec![
            FieldChange {
                field: member::STARTED_AT.to_owned(),
                before: Some(json!(running.started_at.to_string())),
                after: Some(json!(input.started_at.to_string())),
            },
            counted_up(timer.version),
        ],
    )?;
    let job = ctx.long_timer_job(actor.person.id, input.started_at)?;
    let why = Why::default();
    let (halted, written) = (why.clone(), moved.clone());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                write_timer::<S>(transaction, &written, timer.version, &halted).await?;
                job.queue::<S>(transaction, now).await
            })
        })
        .await
        .map_err(|error| failed_change::<S>(&why, &error, || timer_stale(&actor)))?;
    let view = ctx.timer_view(&actor, Some(&moved)).await?;
    ctx.own_answer(&actor, view).await
}

/// Stops the timer and saves nothing.
///
/// # Errors
/// `timer.not-running`, `edit.version-changed`.
pub async fn throw_away_timer<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ThrowAwayTimer,
) -> Result<Authorized<TimerView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (timer, running) = ctx.running_timer(&actor, input.version).await?;
    let stopped = Timer {
        running: None,
        version: next(timer.version),
        ..timer.clone()
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Timer, actor.person.id),
        vec![issue_cleared(&running), counted_up(timer.version)],
    )?;
    let why = Why::default();
    let (halted, written) = (why.clone(), stopped.clone());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                write_timer::<S>(transaction, &written, timer.version, &halted).await
            })
        })
        .await
        .map_err(|error| failed_change::<S>(&why, &error, || timer_stale(&actor)))?;
    let view = ctx.timer_view(&actor, Some(&stopped)).await?;
    ctx.own_answer(&actor, view).await
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The work of the job of a long timer: marks the timer named by `payload` as running long, with
    /// the event the person's one notification is built from. A timer that stopped, started again
    /// or moved its start since the job was queued, or that was marked already, leaves nothing to
    /// do.
    ///
    /// # Errors
    /// `request.invalid` for a payload that names no person and start, an internal error when the
    /// store fails.
    pub(crate) async fn run_long_timer(&self, job: &Value) -> Result<(), AppError> {
        let person: UserId = job
            .get(payload::PERSON)
            .and_then(Value::as_str)
            .and_then(|text| text.parse().ok())
            .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
        let started_at: Timestamp = job
            .get(payload::STARTED_AT)
            .and_then(Value::as_str)
            .and_then(|text| text.parse().ok())
            .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
        let Some(timer) = self.timer_of(person).await? else {
            return Ok(());
        };
        let Some(running) = timer.running().copied() else {
            return Ok(());
        };
        if running.started_at != started_at || running.warned_at.is_some() {
            return Ok(());
        }
        let now = self.now();
        let marked = Timer {
            running: Some(Running {
                warned_at: Some(now),
                ..running
            }),
            version: next(timer.version),
            ..timer.clone()
        };
        let event = self.event(
            bb_core::event::Actor::System,
            entity(RecordKind::Timer, person),
            vec![
                set_to(member::ISSUE, json!(running.issue.to_string())),
                set_to(member::WARNED_AT, json!(now.to_string())),
                counted_up(timer.version),
            ],
        )?;
        let version = Version::new(timer.version);
        let result = self
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    if S::put_timer(transaction, &marked, Some(version)).await? {
                        Ok(())
                    } else {
                        Err(S::aborted())
                    }
                })
            })
            .await;
        match result {
            // The person changed the timer at the same moment: the next run of a long timer is
            // queued by that change, if it still runs.
            Err(error) if S::is_aborted(&error) => Ok(()),
            other => other.map_err(|error| internal::<S>(&error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use bb_core::event::FieldChange;
    use bb_core::version::counted_up;
    use serde_json::json;

    use super::{as_the_timers, entry_member, member};
    use crate::auth::{AppError, set_to};

    #[test]
    fn time_01_the_timers_changes_in_the_event_of_an_entry_keep_their_own_names()
    -> Result<(), AppError> {
        let renamed = as_the_timers(vec![
            set_to(member::ISSUE, json!("i")),
            set_to(member::STARTED_AT, json!("s")),
            counted_up(3),
        ])?;
        let fields: Vec<&str> = renamed.iter().map(|change| change.field.as_str()).collect();
        assert_eq!(
            fields,
            [
                entry_member::TIMER_ISSUE,
                entry_member::TIMER_STARTED_AT,
                entry_member::TIMER_VERSION
            ]
        );
        // A member with no name of its own here is refused, not named as another one.
        let warned = FieldChange {
            field: member::WARNED_AT.to_owned(),
            before: None,
            after: Some(json!("w")),
        };
        assert!(matches!(
            as_the_timers(vec![warned]),
            Err(AppError::Internal(_))
        ));
        Ok(())
    }
}
