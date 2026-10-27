//! The background job runner (ADR-0017): a worker that takes the jobs that are due from the queue
//! in the database and does them, a few at a time, as long as there are some.
//!
//! The queue, the lease and the retries are the store's (`bb-store`, behind the port `JobStore`);
//! which use case does the work of which kind of job, and how long a round holds its jobs, is
//! `bb-app`'s (`Ctx::run_due_jobs`). This crate only owns the loop, so the server starts it and
//! stops it.

#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use bb_app::auth::{AuthServices, AuthStore, RequestInfo};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::MissedTickBehavior;

/// How often the worker looks for jobs that are due.
pub const POLL_EVERY: Duration = Duration::from_secs(2);

/// The longest wait between two tries to make the text index again. The wait starts at the
/// polling time of the worker and doubles after each failure up to this.
pub const REBUILD_RETRY_AT_MOST: Duration = Duration::from_secs(60);

/// Starts the worker; it looks for jobs when it starts and then every `every`. The caller stops it
/// with [`JoinHandle::abort`] when the server stops.
///
/// Beside the jobs, the first thing it does is make the index of the text search again when a
/// migration asked for it (M5-F53): that takes as long as the data is large, and the jobs do not
/// wait for it. A failed try is made again after a wait that starts at `every` and doubles up to
/// [`REBUILD_RETRY_AT_MOST`]; a try that panics is logged and made again the same way.
pub fn spawn<S: AuthStore>(services: Arc<AuthServices<S>>, every: Duration) -> JoinHandle<()> {
    tokio::spawn(async move {
        // The set goes away with this task, and takes the rebuild with it when the worker stops.
        let mut beside = JoinSet::new();
        let mut retry_in = every;
        beside.spawn(rebuild_text_index(Arc::clone(&services), every));
        let mut ticks = tokio::time::interval(every);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticks.tick().await;
            // A finished rebuild is joined; one that panicked is logged (the panic would
            // otherwise be lost with the task) and, after a wait, started again.
            while let Some(done) = beside.try_join_next() {
                if let Err(error) = done {
                    tracing::error!(
                        cause = %error,
                        "the task that makes the text index again stopped; it is started again"
                    );
                    // The wait is inside the new task: the jobs do not wait for it.
                    let (services, wait) = (Arc::clone(&services), retry_in);
                    beside.spawn(async move {
                        tokio::time::sleep(wait).await;
                        rebuild_text_index(services, every).await;
                    });
                    retry_in = (retry_in * 2).min(REBUILD_RETRY_AT_MOST);
                }
            }
            loop {
                match services.ctx(RequestInfo::default()).run_due_jobs().await {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(error) => {
                        tracing::warn!(
                            cause = %error,
                            "the jobs could not be taken; the next round tries again"
                        );
                        break;
                    }
                }
            }
        }
    })
}

/// Makes the text index again if a migration asked for it, until it has gone through once.
async fn rebuild_text_index<S: AuthStore>(services: Arc<AuthServices<S>>, every: Duration) {
    let mut wait = every;
    loop {
        match services
            .ctx(RequestInfo::default())
            .rebuild_text_index_if_wanted()
            .await
        {
            Ok(_) => return,
            Err(error) => {
                tracing::warn!(
                    cause = %error,
                    "the text index could not be made again; the next try follows"
                );
                tokio::time::sleep(wait).await;
                wait = (wait * 2).min(REBUILD_RETRY_AT_MOST);
            }
        }
    }
}
