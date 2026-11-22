//! The two lanes of the queue (ADR-0017, amendment of 2026-11-22): a preview, the file of
//! *Download my data* and the daily backup may take seconds or minutes each, so they are taken
//! apart from everything else, and an e-mail, a step of an agent, a removal or a clean-up never
//! waits behind them. The runner works both lanes side by side.

use bb_core::id::JobId;
use bb_core::job::FailureOutcome;
use bb_core::technical::TechnicalName;

use super::{JOBS_AT_A_TIME, JobKind, LEASE};
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// A lane of the queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    /// The jobs that are done in a moment: e-mail, agents, privacy, keep periods, steps of many
    /// changes. A kind that this program does not know runs here too, so it is reported.
    Short,
    /// The jobs that may take long: previews, exports, backups.
    Long,
}

/// Which kinds a lane takes, as the store filters them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LaneKinds {
    /// Only these kinds.
    Only(Vec<TechnicalName>),
    /// Every kind but these.
    AllBut(Vec<TechnicalName>),
}

impl LaneKinds {
    /// Whether a job of `kind` belongs to the lane.
    #[must_use]
    pub fn takes(&self, kind: &TechnicalName) -> bool {
        match self {
            Self::Only(names) => names.contains(kind),
            Self::AllBut(names) => !names.contains(kind),
        }
    }
}

impl JobKind {
    /// The lane of the kind. A `match` without a catch-all, so a new kind has to say.
    #[must_use]
    pub const fn lane(self) -> Lane {
        match self {
            Self::PreviewFile | Self::DataExport | Self::BackupsDaily => Lane::Long,
            Self::ManyChangesStep
            | Self::RenewIds
            | Self::PurgeDrafts
            | Self::CatchUpHistory
            | Self::MailSend
            | Self::PurgeNotifications
            | Self::PurgeDeadJobs
            | Self::PruneSecurityLog
            | Self::DeleteOldAwayTimes
            | Self::PurgeSessions
            | Self::RotateServerLog
            | Self::CleanUpFiles
            | Self::CleanOldGrants
            | Self::RefreshStatistics
            | Self::TimersLongRunning
            | Self::HandOverRunsOut
            | Self::RuleRunsOut
            | Self::RuleAboutToRunOut
            | Self::RuleMatches
            | Self::RulesGetAnEnd
            | Self::DeleteOldExports
            | Self::FinishRemoval => Lane::Short,
        }
    }
}

impl Lane {
    /// Both lanes, the short one first.
    pub const ALL: [Self; 2] = [Self::Short, Self::Long];

    /// The kinds the lane takes. The short lane is "every kind but the long ones", so a job of a
    /// kind this program does not know is taken and reported, never left in the queue.
    ///
    /// # Errors
    /// An internal error when a word is not a technical name (it is one, by the tests).
    pub fn kinds(self) -> Result<LaneKinds, AppError> {
        let long = JobKind::ALL
            .iter()
            .filter(|kind| kind.lane() == Self::Long)
            .map(|kind| kind.name())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(match self {
            Self::Long => LaneKinds::Only(long),
            Self::Short => LaneKinds::AllBut(long),
        })
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Runs the due jobs of both lanes once, the short lane first: what a test or a command does
    /// that has no runner. Returns how many it took.
    ///
    /// # Errors
    /// The error of the store while jobs are taken.
    pub async fn run_due_jobs(&self) -> Result<usize, AppError> {
        let mut taken = 0;
        for lane in Lane::ALL {
            taken += self.run_lane(lane).await?;
        }
        Ok(taken)
    }

    /// Runs the due jobs of one lane once: takes up to [`JOBS_AT_A_TIME`] of its jobs for a worker
    /// that has a name of its own for this round, does each, and reports it done or failed.
    /// Returns how many it took.
    ///
    /// # Errors
    /// The error of the store while jobs are taken.
    pub async fn run_lane(&self, lane: Lane) -> Result<usize, AppError> {
        if lane == Lane::Short {
            // The notifications come first, so the e-mail they make is queued before the jobs
            // are taken.
            if let Err(error) = self.catch_up_notifications().await {
                tracing::warn!(cause = %error, "the notifications could not catch up");
            }
            // The index of the text search follows the changes within a round, and so is caught
            // up after a start as soon as the first round runs (it has no daily job of its own).
            if let Err(error) = self.catch_up_text_index().await {
                tracing::warn!(cause = %error, "the text index could not catch up");
            }
        }
        let worker = self.worker_name()?;
        let jobs = self
            .store()
            .lease_jobs(&worker, self.now(), LEASE, JOBS_AT_A_TIME, &lane.kinds()?)
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
    use super::*;

    #[test]
    fn adr_0017_every_kind_is_in_exactly_one_lane_and_an_unknown_kind_runs_short() {
        let short = Lane::Short.kinds().unwrap();
        let long = Lane::Long.kinds().unwrap();
        for kind in JobKind::ALL {
            let name = kind.name().unwrap();
            assert_ne!(short.takes(&name), long.takes(&name), "{kind:?}");
            assert_eq!(long.takes(&name), kind.lane() == Lane::Long, "{kind:?}");
        }
        for slow in [
            JobKind::PreviewFile,
            JobKind::DataExport,
            JobKind::BackupsDaily,
        ] {
            assert_eq!(slow.lane(), Lane::Long);
        }
        assert_eq!(JobKind::MailSend.lane(), Lane::Short);
        let unknown = TechnicalName::new("some.kind-of-tomorrow").unwrap();
        assert!(short.takes(&unknown));
        assert!(!long.takes(&unknown));
    }
}
