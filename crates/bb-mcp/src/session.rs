//! The run of a session of an AI app (AI-38, ADR-0042 decision 5): one run for as long as the app
//! keeps calling, a new one after [`IDLE`] without a call, and none for an agent, whose calls belong
//! to the run of its attempt.
//!
//! The newer protocol has no session of its own, so the run is kept per grant (or access key), not
//! per transport session: the first call opens a run, the calls that follow within the idle time
//! (`initialize` too) go on in it, a call after the idle time starts a new one, and the sweeper
//! closes the runs nobody calls any more. The runs are opened and closed through the use cases of
//! `bb-app`, which keep them with the port of runs.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bb_app::auth::{
    AppError, AuthApi, EndAppRuns, McpIdentity, RequestInfo, StartAppSession, locked,
};
use bb_core::id::RunId;
use bb_core::oauth::SESSION_IDLE_MINUTES;
use bb_core::run::RunEnd;
use tokio::time::Instant;

/// Minutes without a call after which a session of an app ends: the one named constant of the
/// idle time (ADR-0042).
pub(crate) const IDLE: Duration = Duration::from_secs(SESSION_IDLE_MINUTES * 60);
/// How often the sweeper looks for sessions that have been idle too long.
const SWEEP_EVERY: Duration = Duration::from_mins(1);

struct Open {
    run: RunId,
    seen: Instant,
}

/// What one grant (or key) has open: the slot is locked while a call finds or starts its run, so
/// two first calls at once start one run and not two.
type Slot = Arc<tokio::sync::Mutex<Option<Open>>>;

/// The open runs of the sessions of apps, by what the session is counted under.
pub(crate) struct Sessions {
    api: Arc<dyn AuthApi>,
    open: Mutex<HashMap<String, Slot>>,
}

impl Sessions {
    pub(crate) fn new(api: Arc<dyn AuthApi>) -> Arc<Self> {
        let sessions = Arc::new(Self {
            api,
            open: Mutex::default(),
        });
        // A server that was not started inside a runtime (a test of the routes) has no sweeper;
        // the next call after the idle time still starts a new run.
        if tokio::runtime::Handle::try_current().is_ok() {
            let watched = Arc::downgrade(&sessions);
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(SWEEP_EVERY).await;
                    let Some(sessions) = watched.upgrade() else {
                        break;
                    };
                    sessions.close_idle().await;
                }
            });
        }
        sessions
    }

    /// The slot of a grant or key, made when there is none.
    fn slot_of(&self, credential: &str) -> Slot {
        Arc::clone(locked(&self.open).entry(credential.to_owned()).or_default())
    }

    /// The run the calls of this session belong to: the open one when it was called within
    /// [`IDLE`], else a new one.
    ///
    /// # Errors
    /// The error of the use case when the token does not work.
    pub(crate) async fn run_of(
        &self,
        info: &RequestInfo,
        token: &str,
        who: &McpIdentity,
    ) -> Result<Option<RunId>, AppError> {
        if who.agent.is_some() {
            return Ok(None);
        }
        let slot = self.slot_of(&who.credential);
        let mut held = slot.lock().await;
        let now = Instant::now();
        if let Some(found) = held.as_mut() {
            if now.duration_since(found.seen) < IDLE {
                found.seen = now;
                return Ok(Some(found.run));
            }
            let old = found.run;
            *held = None;
            self.close(info, vec![old], RunEnd::Done).await;
        }
        let started = self
            .api
            .start_app_session(
                info.clone(),
                StartAppSession {
                    token: token.to_owned(),
                },
            )
            .await?;
        if let Some(run) = started.run {
            *held = Some(Open { run, seen: now });
        }
        Ok(started.run)
    }

    /// Ends the session of this grant or key now (the app said goodbye).
    pub(crate) async fn end(&self, info: &RequestInfo, who: &McpIdentity) {
        let slot = self.slot_of(&who.credential);
        let found = slot.lock().await.take();
        self.close(
            info,
            found.into_iter().map(|each| each.run).collect(),
            RunEnd::Done,
        )
        .await;
    }

    /// Closes the runs of the sessions that have been idle for [`IDLE`]. A slot that is being
    /// used right now is not idle.
    pub(crate) async fn close_idle(&self) {
        let now = Instant::now();
        let slots: Vec<Slot> = locked(&self.open).values().cloned().collect();
        let mut idle = Vec::new();
        for slot in slots {
            let Ok(mut held) = slot.try_lock() else {
                continue;
            };
            if held
                .as_ref()
                .is_some_and(|open| now.duration_since(open.seen) >= IDLE)
                && let Some(open) = held.take()
            {
                idle.push(open.run);
            }
        }
        // A slot nobody holds (only the map has it) and that has no run is left out, so the map
        // does not grow with every grant that ever called. The test is made under the lock of the
        // map: nobody can take the slot out of it while it is judged.
        locked(&self.open).retain(|_, slot| {
            Arc::strong_count(slot) > 1 || slot.try_lock().is_ok_and(|held| held.is_some())
        });
        self.close(&RequestInfo::default(), idle, RunEnd::Done)
            .await;
    }

    async fn close(&self, info: &RequestInfo, runs: Vec<RunId>, how: RunEnd) {
        if runs.is_empty() {
            return;
        }
        if let Err(error) = self
            .api
            .end_app_runs(info.clone(), EndAppRuns { runs, how })
            .await
        {
            tracing::warn!(cause = %error, "the runs of sessions of AI apps could not be closed");
        }
    }
}
