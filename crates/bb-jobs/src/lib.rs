//! The background job runner (ADR-0017): a worker that takes the jobs that are due from the queue
//! in the database and does them, a few at a time, as long as there are some.
//!
//! The queue, the lease and the retries are the store's (`bb-store`, behind the port `JobStore`);
//! which use case does the work of which kind of job is `bb-app`'s (`Ctx::run_due_jobs`). This
//! crate only owns the loop and the name of the worker, so the server starts it and stops it.

#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use bb_app::auth::{AuthServices, AuthStore, RequestInfo};
use bb_app::context::SystemIds;
use bb_core::id::JobId;
use bb_core::technical::TechnicalName;
use jiff::SignedDuration;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;

/// How often the worker looks for jobs that are due.
pub const POLL_EVERY: Duration = Duration::from_secs(2);

/// How long a worker holds the jobs it took before another worker may take them: a step of a job
/// is far shorter, and a worker that stops loses its jobs to another after this.
const LEASE: SignedDuration = SignedDuration::from_secs(120);

/// How many jobs a worker takes at a time.
const TAKE: u64 = 5;

/// The name of this worker: one for each process, so two servers on one database never report each
/// other's jobs. `None` only if the random source gave an ID that is no technical name, which it
/// never does (the ID is lower-case hexadecimal).
fn worker_name() -> Option<TechnicalName> {
    let id = JobId::generate(&mut SystemIds).ok()?;
    TechnicalName::new(&format!("worker.{}", id.to_string().replace('-', "_"))).ok()
}

/// Starts the worker; it looks for jobs when it starts and then every [`POLL_EVERY`]. The caller
/// stops it with [`JoinHandle::abort`] when the server stops.
pub fn spawn<S: AuthStore>(services: Arc<AuthServices<S>>, every: Duration) -> JoinHandle<()> {
    let Some(worker) = worker_name() else {
        tracing::error!("no name for the job worker; jobs will not run");
        return tokio::spawn(async {});
    };
    tokio::spawn(async move {
        let mut ticks = tokio::time::interval(every);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticks.tick().await;
            loop {
                match services
                    .ctx(RequestInfo::default())
                    .run_due_jobs(&worker, LEASE, TAKE)
                    .await
                {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(error) => {
                        tracing::warn!(cause = %error, "the jobs could not be taken; the next round tries again");
                        break;
                    }
                }
            }
        }
    })
}
