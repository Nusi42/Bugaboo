//! The background job runner (ADR-0017): a worker that takes the jobs that are due from the queue
//! in the database and does them, a few at a time, as long as there are some.
//!
//! The queue, the lease and the retries are the store's (`bb-store`, behind the port `JobStore`);
//! which use case does the work of which kind of job, and how long a round holds its jobs, is
//! `bb-app`'s (`Ctx::run_due_jobs`). This crate only owns the loop, so the server starts it and
//! stops it.

#![forbid(unsafe_code)]

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use bb_app::auth::{AuthServices, AuthStore, RequestInfo};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::MissedTickBehavior;

/// How often the worker looks for jobs that are due.
pub const POLL_EVERY: Duration = Duration::from_secs(2);

/// The longest wait between two tries to make the text index again. The wait starts at the
/// polling time of the worker and doubles after each failure up to this.
pub const REBUILD_RETRY_AT_MOST: Duration = Duration::from_mins(1);

/// The wait after a wait: twice as long, never longer than [`REBUILD_RETRY_AT_MOST`]. The one place
/// that doubles a wait.
fn next_wait(wait: Duration) -> Duration {
    (wait * 2).min(REBUILD_RETRY_AT_MOST)
}

/// Starts the worker; it looks for jobs when it starts and then every `every`. The caller stops it
/// with [`JoinHandle::abort`] when the server stops.
///
/// Beside the jobs, the first thing it does is make the index of the text search again when a
/// migration asked for it (M5-F53): that takes as long as the data is large, and the jobs do not
/// wait for it. A failed try is made again after a wait that starts at `every` and doubles up to
/// [`REBUILD_RETRY_AT_MOST`]; a try that panics is logged and made again the same way.
pub fn spawn<S: AuthStore>(services: Arc<AuthServices<S>>, every: Duration) -> JoinHandle<()> {
    supervise(services, every, rebuild_text_index)
}

/// [`spawn`] with the task that makes the index again given, so a test can make it panic. The task
/// takes the services and the wait to start from, and ends when the index has been made.
#[cfg(feature = "testing")]
pub fn spawn_with<S, R, F>(
    services: Arc<AuthServices<S>>,
    every: Duration,
    rebuild: R,
) -> JoinHandle<()>
where
    S: AuthStore,
    R: Fn(Arc<AuthServices<S>>, Duration) -> F + Clone + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    supervise(services, every, rebuild)
}

/// What a test hands [`spawn_with`] after its own task has failed: the real task.
#[cfg(feature = "testing")]
pub async fn rebuild_text_index_for_a_test<S: AuthStore>(
    services: Arc<AuthServices<S>>,
    wait: Duration,
) {
    rebuild_text_index(services, wait).await;
}

/// The loop of the worker with the task that makes the index again given.
fn supervise<S, R, F>(services: Arc<AuthServices<S>>, every: Duration, rebuild: R) -> JoinHandle<()>
where
    S: AuthStore,
    R: Fn(Arc<AuthServices<S>>, Duration) -> F + Clone + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        // The set goes away with this task, and takes the rebuild with it when the worker stops.
        let mut beside = JoinSet::new();
        let mut retry_in = every;
        beside.spawn(rebuild(Arc::clone(&services), every));
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
                    let (services, wait, rebuild) =
                        (Arc::clone(&services), retry_in, rebuild.clone());
                    beside.spawn(async move {
                        tokio::time::sleep(wait).await;
                        rebuild(services, every).await;
                    });
                    retry_in = next_wait(retry_in);
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

/// Makes the text index again if a migration asked for it, until it has gone through once; a failed
/// try waits `wait`, then twice as long each time.
async fn rebuild_text_index<S: AuthStore>(services: Arc<AuthServices<S>>, mut wait: Duration) {
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
                wait = next_wait(wait);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wait_doubles_up_to_the_longest() {
        let mut wait = POLL_EVERY;
        let mut seen = Vec::new();
        for _ in 0..7 {
            seen.push(wait.as_secs());
            wait = next_wait(wait);
        }
        assert_eq!(seen, [2, 4, 8, 16, 32, 60, 60]);
    }
}
