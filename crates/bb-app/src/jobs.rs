//! The queue of background jobs as the use cases see it (ADR-0017): what puts a job in the queue,
//! and what a runner needs to take a job, report it done or report it failed.
//!
//! A job is a kind and a payload. The kinds are named once, here, and [`Ctx::run_due_jobs`] is the
//! one place that says which use case does the work of which kind, so a runner (`bb-jobs`) knows
//! nothing of them.

use std::future::Future;

use bb_core::id::JobId;
use bb_core::job::{FailureKind, FailureOutcome, Job};
use bb_core::technical::TechnicalName;
use bb_core::wording::Region;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::editing::{PurgeDraftsQuery, purge_drafts};
use crate::ports::ChangeStore;
use crate::store_macros::{reads, writes};

/// How many starts a job gets before it goes to the dead-letter list.
pub const JOB_ATTEMPTS_MAX: u32 = 5;

bb_core::words! {
    /// The kinds of job, each with the name the queue stores. [`JobKind::is_daily`] says which
    /// come back every day; `Ctx::do_job` routes by it, and `Ctx::job_gave_up` ends a dead job
    /// of a kind that is not daily; these matches have no catch-all, so a new kind has to be given
    /// its work and its end there.
    pub enum JobKind {
        /// One step of a change to many issues; the payload names the run.
        ManyChangesStep => "many-changes.step",
        /// One step of giving the issues of a project new IDs after its mode of IDs changed
        /// (`issues::ids`); the payload names the project and the person who changed the mode.
        RenewIds => "issues.renew-ids",
        /// Clearing what outlived its keep period (drafts, notes for people who were editing, lock
        /// rows that ran out): the use case `purge_drafts`.
        PurgeDrafts => "editing.purge-drafts",
        /// The history catching up with the events (M3-F71): `Ctx::catch_up_history`.
        CatchUpHistory => "history.catch-up",
        /// One e-mail (`notifications`): the payload names the kind of e-mail, who gets it and
        /// about what. Queued by the reader of notifications and by signing up.
        MailSend => "mail.send",
        /// Deleting the notifications that outlived their keep period, and the payloads of the
        /// jobs that died before theirs ran out (PRIV-05, M1-F21).
        PurgeNotifications => "notifications.purge",
        /// Deleting the oldest blocks of the security log that outlived its keep period (M1-F21).
        PruneSecurityLog => "security-log.prune",
        /// Deleting the sessions nobody can use any more and the links of finished sign-ups
        /// (M1-F43).
        PurgeSessions => "sessions.purge",
        /// Starting a new file of the log of a server that has no console (M1-F61).
        RotateServerLog => "server-log.rotate",
        /// The preview of an attachment, made in a process of its own (`files::jobs`, ADR-0032);
        /// the payload names the attachment.
        PreviewFile => "files.preview",
        /// Clearing old uploads, attachments that stayed in the trash and files nothing points to
        /// (`files::jobs`).
        CleanUpFiles => "files.clean-up",
        /// Making the statistics the database chooses its indexes by again, from every row, for each
        /// table that is out of date (SQLite; PostgreSQL keeps its own): `statistics`.
        RefreshStatistics => "statistics.refresh",
    }
}

impl JobKind {
    /// Whether the kind comes back every day (see [`DAILY`]). A `match` without a catch-all, so a
    /// new kind has to say.
    #[must_use]
    pub const fn is_daily(self) -> bool {
        match self {
            Self::CatchUpHistory
            | Self::CleanUpFiles
            | Self::RefreshStatistics
            | Self::PurgeDrafts
            | Self::PurgeNotifications
            | Self::PruneSecurityLog
            | Self::PurgeSessions
            | Self::RotateServerLog => true,
            Self::ManyChangesStep | Self::RenewIds | Self::MailSend | Self::PreviewFile => false,
        }
    }

    /// When the rounds of the kind come (only the daily kinds have rounds). A `match` without a
    /// catch-all, so a new kind has to say.
    const fn rhythm(self) -> Rhythm {
        match self {
            // Its first round on a large database holds SQLite's write lock for seconds, so it is
            // kept away from the start, when people sign in again (M5-F168).
            Self::RefreshStatistics => Rhythm::QuietHour,
            Self::CatchUpHistory
            | Self::CleanUpFiles
            | Self::PurgeDrafts
            | Self::PurgeNotifications
            | Self::PruneSecurityLog
            | Self::PurgeSessions
            | Self::RotateServerLog
            | Self::ManyChangesStep
            | Self::RenewIds
            | Self::MailSend
            | Self::PreviewFile => Rhythm::FromTheStart,
        }
    }

    /// The kinds that come back every day.
    pub fn daily() -> impl Iterator<Item = Self> {
        Self::ALL.iter().copied().filter(|kind| kind.is_daily())
    }

    /// The name of the kind as the queue stores it.
    ///
    /// # Errors
    /// An internal error when the word is not a technical name (it is one, by the tests).
    pub fn name(self) -> Result<TechnicalName, AppError> {
        TechnicalName::new(self.code()).map_err(|error| AppError::Internal(error.to_string()))
    }
}

/// How long after a daily job its next round is due.
pub const DAILY: SignedDuration = SignedDuration::from_hours(24);

/// How long after a start of the server the first round of a kind that keeps to the quiet hour
/// runs: after the start, when people sign in again, but on the same day.
pub const QUIET_KIND_AFTER_START: SignedDuration = SignedDuration::from_mins(5);

/// The hour of the day, in the organization's time zone (*Region*), at which a kind that keeps to
/// the quiet hour runs every day.
pub const QUIET_HOUR: i8 = 3;

/// When the rounds of a daily kind come: the one schedule of the daily jobs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rhythm {
    /// A round at every start of the server, then one every [`DAILY`].
    FromTheStart,
    /// A round [`QUIET_KIND_AFTER_START`] after every start, then one every day at
    /// [`QUIET_HOUR`].
    QuietHour,
}

impl Rhythm {
    /// When the round that a start of the server queues is due.
    fn first_round(self, now: Timestamp) -> Result<Timestamp, AppError> {
        match self {
            Self::FromTheStart => Ok(now),
            Self::QuietHour => now
                .checked_add(QUIET_KIND_AFTER_START)
                .map_err(|error| AppError::Internal(error.to_string())),
        }
    }

    /// When the round after the one that runs at `now` is due; `zone` is the organization's.
    fn next_round(self, now: Timestamp, zone: &TimeZone) -> Result<Timestamp, AppError> {
        let failed = |error: jiff::Error| AppError::Internal(error.to_string());
        match self {
            Self::FromTheStart => now.checked_add(DAILY).map_err(failed),
            Self::QuietHour => {
                let today = now.to_zoned(zone.clone()).date();
                let at_quiet_hour = |day: jiff::civil::Date| {
                    day.at(QUIET_HOUR, 0, 0, 0)
                        .to_zoned(zone.clone())
                        .map(|zoned| zoned.timestamp())
                };
                let quiet_today = at_quiet_hour(today).map_err(failed)?;
                if quiet_today > now {
                    Ok(quiet_today)
                } else {
                    at_quiet_hour(today.tomorrow().map_err(failed)?).map_err(failed)
                }
            }
        }
    }
}

/// How long a worker holds the jobs it took before another may take them: a step of a job is far
/// shorter, and a worker that stops loses its jobs to another after this.
const LEASE: SignedDuration = SignedDuration::from_secs(120);

/// How many jobs a round takes.
const JOBS_AT_A_TIME: u64 = 5;

/// The code a job reports when its kind is not one any use case does.
const UNKNOWN_KIND: &str = "job.unknown_kind";

/// The code a job reports when its work failed.
const FAILED: &str = "job.failed";

/// The failure of a job that went to the wrong function of the routing in `do_job`: a mistake of
/// the code, not of the job. It stops a test or a debug build at once instead of letting the job
/// die as `unknown_kind`.
fn misrouted(job: &Job, kind: JobKind) -> JobFailure {
    debug_assert!(
        false,
        "the job kind {kind:?} was routed to the wrong function"
    );
    JobFailure::unknown_kind(job)
}

/// A job to put in the queue together with the change that needs the work done (the notifications
/// reader queues the e-mails with the rows they are about).
#[derive(Clone, Debug)]
pub struct QueuedJob {
    /// The ID of the job.
    pub id: JobId,
    /// The kind of work.
    pub kind: TechnicalName,
    /// What the work needs to know.
    pub payload: Value,
    /// The earliest moment it may run.
    pub run_at: Timestamp,
}

/// What a job needs from storage.
pub trait JobStore: ChangeStore + 'static {
    reads! {
        /// Takes up to `limit` jobs that are due and not leased, for `worker`, until `now + lease`.
        fn lease_jobs(
            worker: &TechnicalName,
            now: Timestamp,
            lease: SignedDuration,
            limit: u64
        ) -> Vec<Job>;
        /// Reports a job as done; it leaves the queue. `false` when `worker` does not hold the
        /// lease any more.
        fn complete_job(id: JobId, worker: &TechnicalName) -> bool;
        /// Reports a job as failed, with a technical code, and says what happened to it: it runs
        /// again later or it is on the dead-letter list (after its last start, or at once for a
        /// failure that is permanent). `None` when `worker` does not hold the lease any more.
        fn fail_job(
            id: JobId,
            worker: &TechnicalName,
            failure: &TechnicalName,
            kind: FailureKind,
            now: Timestamp
        ) -> Option<FailureOutcome>;
        /// Whether a job of this kind waits in the queue (not dead) for a time after `after`.
        fn later_job_waits(kind: &TechnicalName, after: Timestamp) -> bool;
        /// Empties the payload of the jobs on the dead-letter list whose last start was before
        /// `before` (their keep period, M1-F21): it can hold the address and the text of a person.
        /// The job stays on the list. Returns how many were emptied.
        fn clear_dead_job_payloads(before: Timestamp) -> u64;
        /// Puts a job in the queue on its own: for the rest of a piece of work that a job did a
        /// step of (the step is bookkeeping of a run that is stored already, not a domain change).
        fn queue_job(
            id: JobId,
            kind: &TechnicalName,
            payload: &Value,
            run_at: Timestamp,
            now: Timestamp
        ) -> ();
    }

    writes! {
        /// Puts a job in the queue inside the change that needs the work done: both are written
        /// together or neither.
        fn enqueue_job(
            id: JobId,
            kind: &TechnicalName,
            payload: &Value,
            run_at: Timestamp,
            now: Timestamp
        ) -> ();
    }
}

/// How a job failed: the technical code the queue keeps, and whether another start can help.
struct JobFailure {
    code: TechnicalName,
    kind: FailureKind,
}

impl JobFailure {
    /// The failure of `error`: a refusal (the payload names nothing, the request is not allowed)
    /// can never succeed as it was asked for, so it is not tried again; every other error (the
    /// store broke, the database was busy with its upkeep) may pass.
    fn of(error: &AppError, code: &str, job: &Job) -> Self {
        Self {
            code: TechnicalName::new(code).unwrap_or_else(|_| job.kind.clone()),
            kind: match error {
                busy if busy.is_busy() => FailureKind::Temporary,
                AppError::Refused(_) => FailureKind::Permanent,
                _ => FailureKind::Temporary,
            },
        }
    }

    fn unknown_kind(job: &Job) -> Self {
        Self {
            code: TechnicalName::new(UNKNOWN_KIND).unwrap_or_else(|_| job.kind.clone()),
            kind: FailureKind::Permanent,
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What a worker does with a job it holds: `Err` says how it failed, and the queue tries the
    /// job again later or, after its last start (or at once, for a failure that is permanent),
    /// puts it on the dead-letter list.
    async fn do_job(&self, job: &Job) -> Result<(), JobFailure> {
        match JobKind::from_code(job.kind.as_str()) {
            Some(kind) if kind.is_daily() => self.do_daily_job(job, kind).await,
            Some(kind) => self.do_one_off_job(job, kind).await,
            None => {
                tracing::error!(kind = job.kind.as_str(), "a job of a kind nobody knows");
                Err(JobFailure::unknown_kind(job))
            }
        }
    }

    /// The jobs that are queued by the work that needs them and do not come back by themselves
    /// (the kinds for which [`JobKind::is_daily`] is false).
    async fn do_one_off_job(&self, job: &Job, kind: JobKind) -> Result<(), JobFailure> {
        match kind {
            JobKind::ManyChangesStep => Box::pin(self.run_many_changes_step(&job.payload))
                .await
                .map_err(|error| {
                    tracing::warn!(cause = %error, "a step of a change to many issues failed");
                    JobFailure::of(&error, FAILED, job)
                }),
            JobKind::RenewIds => {
                Box::pin(self.renew_ids_step(&job.payload))
                    .await
                    .map_err(|error| {
                        tracing::warn!(cause = %error, "a step of renewing issue IDs failed");
                        JobFailure::of(&error, FAILED, job)
                    })
            }
            JobKind::PreviewFile => Box::pin(self.run_preview_step(&job.payload)).await.map_err(
                |error| {
                    tracing::warn!(cause = %error, "the preview of a file could not be created");
                    JobFailure::of(&error, FAILED, job)
                },
            ),
            JobKind::MailSend => Box::pin(self.run_mail_job(&job.payload))
                .await
                .map_err(|error| {
                    // The class only: the answer of the mail server can hold an address.
                    tracing::warn!(
                        refused = matches!(error, AppError::Refused(_)),
                        "an e-mail did not go"
                    );
                    JobFailure::of(&error, FAILED, job)
                }),
            // The daily kinds go to `do_daily_job`; listed so that a new kind has to be placed.
            JobKind::PurgeDrafts
            | JobKind::CatchUpHistory
            | JobKind::PurgeNotifications
            | JobKind::PruneSecurityLog
            | JobKind::PurgeSessions
            | JobKind::RotateServerLog
            | JobKind::CleanUpFiles
            | JobKind::RefreshStatistics => Err(misrouted(job, kind)),
        }
    }

    /// The daily jobs (the kinds for which [`JobKind::is_daily`] is true): old drafts, the history
    /// catching up, old notifications, the security log, sessions, the server log, old files and
    /// the statistics of the database. Each queues its next round first (`run_daily`).
    async fn do_daily_job(&self, job: &Job, kind: JobKind) -> Result<(), JobFailure> {
        match kind {
            JobKind::PurgeDrafts => Box::pin(self.run_daily(JobKind::PurgeDrafts, async {
                purge_drafts(self, PurgeDraftsQuery).await.map(|drafts| {
                    tracing::info!(drafts, "old drafts were cleared");
                })
            }))
            .await
            .map_err(|error| {
                tracing::warn!(cause = %error, "old drafts could not be cleared");
                JobFailure::of(&error, FAILED, job)
            }),
            JobKind::CatchUpHistory => Box::pin(self.run_daily(JobKind::CatchUpHistory, async {
                self.catch_up_history().await.map(|events| {
                    tracing::info!(events, "the history caught up with the events");
                })
            }))
            .await
            .map_err(|error| {
                tracing::warn!(cause = %error, "the history could not catch up");
                JobFailure::of(&error, FAILED, job)
            }),
            JobKind::PurgeNotifications => {
                Box::pin(self.run_daily(JobKind::PurgeNotifications, async {
                    self.purge_old_notifications().await.map(|purged| {
                        tracing::info!(
                            notifications = purged.notifications,
                            payloads = purged.payloads,
                            "old notifications and the payloads of dead jobs were deleted"
                        );
                    })
                }))
                .await
                .map_err(|error| {
                    tracing::warn!(cause = %error, "old notifications could not be deleted");
                    JobFailure::of(&error, FAILED, job)
                })
            }
            JobKind::PruneSecurityLog => {
                Box::pin(self.run_daily(JobKind::PruneSecurityLog, async {
                    self.prune_old_security_log().await.map(|entries| {
                        tracing::info!(entries, "old entries of the security log were deleted");
                    })
                }))
                .await
                .map_err(|error| {
                    tracing::warn!(cause = %error, "the security log could not be pruned");
                    JobFailure::of(&error, FAILED, job)
                })
            }
            JobKind::PurgeSessions => Box::pin(self.run_daily(JobKind::PurgeSessions, async {
                self.purge_dead_sessions().await.map(|()| {
                    tracing::info!("sessions nobody can use were deleted");
                })
            }))
            .await
            .map_err(|error| {
                tracing::warn!(cause = %error, "old sessions could not be deleted");
                JobFailure::of(&error, FAILED, job)
            }),
            JobKind::RotateServerLog => Box::pin(self.run_daily(JobKind::RotateServerLog, async {
                self.rotate_old_server_log().await.map(|rotated| {
                    tracing::info!(rotated, "the server log was looked at");
                })
            }))
            .await
            .map_err(|error| {
                tracing::warn!(cause = %error, "the server log could not be rotated");
                JobFailure::of(&error, FAILED, job)
            }),
            JobKind::CleanUpFiles => Box::pin(self.run_daily(JobKind::CleanUpFiles, async {
                self.clean_up_files().await.map(|cleaned| {
                    tracing::info!(
                        uploads = cleaned.uploads,
                        attachments = cleaned.attachments,
                        files = cleaned.files,
                        "old files were cleared"
                    );
                })
            }))
            .await
            .map_err(|error| {
                tracing::warn!(cause = %error, "old files could not be cleared");
                JobFailure::of(&error, FAILED, job)
            }),
            JobKind::RefreshStatistics => self.refresh_statistics_daily(job).await,
            // The other kinds go to `do_one_off_job`; listed so that a new kind has to be placed.
            JobKind::ManyChangesStep
            | JobKind::RenewIds
            | JobKind::MailSend
            | JobKind::PreviewFile => Err(misrouted(job, kind)),
        }
    }

    /// The daily round of `statistics.refresh` (in a function of its own, so `do_daily_job` stays
    /// short).
    async fn refresh_statistics_daily(&self, job: &Job) -> Result<(), JobFailure> {
        Box::pin(self.run_daily(JobKind::RefreshStatistics, async {
            self.refresh_statistics().await.map(|refreshed| {
                tracing::info!(refreshed, "the statistics of the database were looked at");
            })
        }))
        .await
        .map_err(|error| {
            tracing::warn!(cause = %error, "the statistics could not be refreshed");
            JobFailure::of(&error, FAILED, job)
        })
    }

    /// What happens when a job is on the dead-letter list: the work that people can see ends in
    /// a state they can see. A change to many issues ends as failed with what it did so far; the
    /// renewal of issue IDs has no record of its own, so its death is an event of the project that
    /// the person who asked is told of (M3-F150).
    async fn job_gave_up(&self, job: &Job) {
        let ended = match JobKind::from_code(job.kind.as_str()) {
            Some(JobKind::ManyChangesStep) => self.fail_many_changes_run(&job.payload).await,
            Some(JobKind::RenewIds) => {
                tracing::error!("renewing issue IDs gave up; the job is on the dead-letter list");
                self.renewal_gave_up(&job.payload).await
            }
            Some(JobKind::MailSend) => self.mail_job_gave_up(&job.payload).await,
            Some(JobKind::PreviewFile) => self.preview_gave_up(&job.payload).await,
            // The daily kinds come back by themselves and leave no work that people can see; a
            // new kind has to be placed in one of these arms.
            Some(
                kind @ (JobKind::PurgeDrafts
                | JobKind::CatchUpHistory
                | JobKind::PurgeNotifications
                | JobKind::PruneSecurityLog
                | JobKind::PurgeSessions
                | JobKind::RotateServerLog
                | JobKind::CleanUpFiles
                | JobKind::RefreshStatistics),
            ) => {
                debug_assert!(kind.is_daily(), "the job kind {kind:?} is not a daily one");
                Ok(())
            }
            None => Ok(()),
        };
        if let Err(error) = ended {
            tracing::error!(cause = %error, "the work of a dead job could not be ended");
        }
    }

    /// Does the work of a job of a daily kind. Its next round is queued first (when its rhythm
    /// says), so a round that fails (and is tried again by the queue) or a worker that stops does
    /// not end the daily rounds; nothing is queued when a round for a later time waits already,
    /// so a server that starts again within the day (see [`Ctx::start_daily_jobs`]) does not add
    /// a second chain.
    async fn run_daily(
        &self,
        kind: JobKind,
        work: impl Future<Output = Result<(), AppError>>,
    ) -> Result<(), AppError> {
        let rhythm = kind.rhythm();
        let kind = kind.name()?;
        let now = self.now();
        let waits = self
            .store()
            .later_job_waits(&kind, now)
            .await
            .map_err(|error| internal::<S>(&error))?;
        if !waits {
            let layers = self.setting_layers_of(None).await?;
            let next = rhythm.next_round(now, Region::of(&layers).zone())?;
            self.queue_daily(&kind, next).await?;
        }
        work.await
    }

    async fn queue_daily(&self, kind: &TechnicalName, at: Timestamp) -> Result<(), AppError> {
        let id = JobId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        self.store()
            .queue_job(id, kind, &Value::Null, at, self.now())
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Queues a round of every daily job: the server calls it when it starts, so old drafts are
    /// cleared and the history is caught up at every start, and the rounds after that come every
    /// [`DAILY`] from the jobs themselves. A kind that keeps to the quiet hour (the statistics of
    /// the database) is not in this round: its round is queued [`QUIET_KIND_AFTER_START`] later.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub async fn start_daily_jobs(&self) -> Result<(), AppError> {
        for kind in JobKind::daily() {
            let at = kind.rhythm().first_round(self.now())?;
            self.queue_daily(&kind.name()?, at).await?;
        }
        Ok(())
    }

    /// Runs the due jobs once: takes up to [`JOBS_AT_A_TIME`] jobs for a worker that has a name of
    /// its own for this round, does each, and reports it done or failed. Returns how many it took.
    ///
    /// # Errors
    /// The error of the store while jobs are taken.
    pub async fn run_due_jobs(&self) -> Result<usize, AppError> {
        // The notifications come first, so the e-mail they make is queued before the jobs are
        // taken.
        if let Err(error) = self.catch_up_notifications().await {
            tracing::warn!(cause = %error, "the notifications could not catch up");
        }
        // The index of the text search follows the changes within a round, and so is caught up
        // after a start as soon as the first round runs (it has no daily job of its own).
        if let Err(error) = self.catch_up_text_index().await {
            tracing::warn!(cause = %error, "the text index could not catch up");
        }
        let worker = self.worker_name()?;
        let jobs = self
            .store()
            .lease_jobs(&worker, self.now(), LEASE, JOBS_AT_A_TIME)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let taken = jobs.len();
        for job in jobs {
            // `true` when the worker still held the lease and the report counted.
            // A job is one request (ADR-0025, decision 3): it gets a context of its own, so what
            // one job looked up for one person is never there for the next job, for another.
            let reported = match self.services.ctx(self.request.clone()).do_job(&job).await {
                Ok(()) => self.store().complete_job(job.id, &worker).await,
                Err(failure) => {
                    let outcome = self
                        .store()
                        .fail_job(job.id, &worker, &failure.code, failure.kind, self.now())
                        .await;
                    if matches!(outcome, Ok(Some(FailureOutcome::Dead))) {
                        self.job_gave_up(&job).await;
                    }
                    outcome.map(|found| found.is_some())
                }
            };
            match reported {
                Ok(true) => {}
                // The lease ran out and another worker has the job: it reports, not this one.
                Ok(false) => tracing::warn!("a job was reported after its lease ran out"),
                Err(error) => {
                    tracing::error!(cause = %S::describe(&error), "a job could not be reported");
                }
            }
        }
        Ok(taken)
    }

    /// The name a worker holds jobs under for one round: a new one each time, so two rounds, also
    /// of two servers on one database, never report each other's jobs.
    fn worker_name(&self) -> Result<TechnicalName, AppError> {
        let id = JobId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        TechnicalName::new(&format!("worker.{}", id.to_string().replace('-', "_")))
            .map_err(|error| AppError::Internal(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use bb_core::ErrorCode;

    use super::*;

    fn at(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    #[test]
    fn adr_0017_a_quiet_kind_comes_next_at_the_quiet_hour_of_the_zone_and_the_others_a_day_later() {
        let berlin = TimeZone::get("Europe/Berlin").unwrap();
        // 01:30 in Berlin (winter, UTC+1): the quiet hour of the same night.
        let night = at("2026-01-10T00:30:00Z");
        assert_eq!(
            Rhythm::QuietHour.next_round(night, &berlin).unwrap(),
            at("2026-01-10T02:00:00Z")
        );
        // Just after the quiet hour: the one of the next night.
        let after = at("2026-01-10T02:00:00Z");
        assert_eq!(
            Rhythm::QuietHour.next_round(after, &berlin).unwrap(),
            at("2026-01-11T02:00:00Z")
        );
        // Summer time (UTC+2).
        assert_eq!(
            Rhythm::QuietHour
                .next_round(at("2026-07-01T12:00:00Z"), &berlin)
                .unwrap(),
            at("2026-07-02T01:00:00Z")
        );
        assert_eq!(
            Rhythm::FromTheStart.next_round(night, &berlin).unwrap(),
            night.checked_add(DAILY).unwrap()
        );
    }

    #[test]
    fn adr_0017_only_the_statistics_wait_after_the_start() {
        let now = at("2026-01-10T12:00:00Z");
        for kind in JobKind::daily() {
            let first = kind.rhythm().first_round(now).unwrap();
            if kind == JobKind::RefreshStatistics {
                assert_eq!(first, now.checked_add(QUIET_KIND_AFTER_START).unwrap());
            } else {
                assert_eq!(first, now, "{}", kind.code());
            }
        }
    }

    #[test]
    fn adr_0017_a_job_that_met_the_busy_database_is_tried_again_and_a_refusal_is_not() {
        let now = at("2026-01-10T12:00:00Z");
        let job = Job {
            id: JobId::generate(&mut crate::context::SystemIds).unwrap(),
            kind: JobKind::ManyChangesStep.name().unwrap(),
            payload: Value::Null,
            run_at: now,
            attempts: 1,
            max_attempts: JOB_ATTEMPTS_MAX,
            state: bb_core::job::JobState::Queued,
            lease_owner: None,
            lease_until: None,
            last_failure: None,
            created_at: now,
        };
        let busy = JobFailure::of(&AppError::busy(), FAILED, &job);
        assert_eq!(busy.kind, FailureKind::Temporary);
        let refused = JobFailure::of(&AppError::Refused(ErrorCode::Forbidden), FAILED, &job);
        assert_eq!(refused.kind, FailureKind::Permanent);
    }
}
