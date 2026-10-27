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

/// Starts the worker; it looks for jobs when it starts and then every `every`. The caller stops it
/// with [`JoinHandle::abort`] when the server stops.
///
/// Beside the jobs, the first thing it does is make the index of the text search again when a
/// migration asked for it (M5-F53): that takes as long as the data is large, and the jobs do not
/// wait for it. A failed try is made again after `every`.
pub fn spawn<S: AuthStore>(services: Arc<AuthServices<S>>, every: Duration) -> JoinHandle<()> {
    tokio::spawn(async move {
        // The set goes away with this task, and takes the rebuild with it when the worker stops.
        let mut beside = JoinSet::new();
        let for_index = Arc::clone(&services);
        beside.spawn(async move {
            loop {
                match for_index
                    .ctx(RequestInfo::default())
                    .rebuild_text_index_if_wanted()
                    .await
                {
                    Ok(_) => break,
                    Err(error) => {
                        tracing::warn!(
                            cause = %error,
                            "the text index could not be made again; the next try follows"
                        );
                        tokio::time::sleep(every).await;
                    }
                }
            }
        });
        let mut ticks = tokio::time::interval(every);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticks.tick().await;
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
