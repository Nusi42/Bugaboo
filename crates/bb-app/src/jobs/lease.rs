//! The lease of a job that may work longer than [`LEASE`] (the file of *Download my data*, a
//! backup): its worker renews it while it works, so a worker that stops still loses the job to
//! another within [`LEASE`].

use bb_core::id::JobId;
use bb_core::job::Job;
use bb_core::technical::TechnicalName;
use jiff::Timestamp;

use super::LEASE;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::backups::KeepLease;

/// The lease a worker holds on the job it runs, renewed when half of it has passed.
pub(crate) struct HeldJob {
    job: JobId,
    worker: Option<TechnicalName>,
    renewed: Timestamp,
}

impl HeldJob {
    /// What tells this try of the job from every other: the worker that holds it, a new one each
    /// round, or the job itself for a job run by hand.
    pub(crate) fn attempt(&self) -> String {
        self.worker
            .as_ref()
            .map_or_else(|| self.job.to_string(), |worker| worker.as_str().to_owned())
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// How a backup made by `job` keeps the lease of the job while it works: renewed every half
    /// of [`LEASE`]. `None` for a job run by hand, which holds no lease.
    pub(crate) fn keep_lease<'a>(&'a self, job: &Job) -> Option<KeepLease<'a>> {
        let worker = job.lease_owner.clone()?;
        let id = job.id;
        Some(KeepLease {
            every: (LEASE / 2).unsigned_abs(),
            renew: Box::new(move || {
                let worker = worker.clone();
                Box::pin(async move { self.renew_lease(id, &worker).await })
            }),
        })
    }

    /// Renews the lease of a job for [`LEASE`] from now; `false` when another worker holds it. A
    /// store that fails this time is tried again at the next renewal.
    pub(super) async fn renew_lease(&self, id: JobId, worker: &TechnicalName) -> bool {
        let Ok(until) = self.now().checked_add(LEASE) else {
            return true;
        };
        match self.store().hold_job(id, worker, until).await {
            Ok(held) => held,
            Err(error) => {
                tracing::warn!(cause = %S::describe(&error), "the lease of a job could not be renewed");
                true
            }
        }
    }

    /// The lease on `job`, as the worker took it now.
    pub(crate) fn held(&self, job: &Job) -> HeldJob {
        HeldJob {
            job: job.id,
            worker: job.lease_owner.clone(),
            renewed: self.now(),
        }
    }

    /// Renews the lease on a job that is still working, once half of it has passed.
    ///
    /// # Errors
    /// An internal error when another worker holds the job now: this one stops, so the job is never
    /// done twice at once; the error of the store.
    pub(crate) async fn hold_on(&self, held: &mut HeldJob) -> Result<(), AppError> {
        let now = self.now();
        if now.duration_since(held.renewed) < LEASE / 2 {
            return Ok(());
        }
        let Some(worker) = &held.worker else {
            return Ok(());
        };
        let until = now
            .checked_add(LEASE)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let still = self
            .store()
            .hold_job(held.job, worker, until)
            .await
            .map_err(|error| internal::<S>(&error))?;
        if !still {
            return Err(AppError::Internal(
                "another worker holds the job now".to_owned(),
            ));
        }
        held.renewed = now;
        Ok(())
    }
}
