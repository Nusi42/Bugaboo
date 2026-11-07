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

/// The open runs of the sessions of apps, by what the session is counted under.
pub(crate) struct Sessions {
    api: Arc<dyn AuthApi>,
    open: Mutex<HashMap<String, Open>>,
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
        let now = Instant::now();
        let mut to_close = Vec::new();
        {
            let mut open = locked(&self.open);
            if let Some(found) = open.get_mut(&who.credential) {
                if now.duration_since(found.seen) < IDLE {
                    found.seen = now;
                    return Ok(Some(found.run));
                }
                to_close.push(found.run);
                open.remove(&who.credential);
            }
        }
        self.close(info, to_close, RunEnd::Done).await;
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
            locked(&self.open).insert(who.credential.clone(), Open { run, seen: now });
        }
        Ok(started.run)
    }

    /// Ends the session of this grant or key now (the app said goodbye).
    pub(crate) async fn end(&self, info: &RequestInfo, who: &McpIdentity) {
        let found = locked(&self.open).remove(&who.credential);
        self.close(
            info,
            found.into_iter().map(|each| each.run).collect(),
            RunEnd::Done,
        )
        .await;
    }

    /// Closes the runs of the sessions that have been idle for [`IDLE`].
    pub(crate) async fn close_idle(&self) {
        let now = Instant::now();
        let idle: Vec<RunId> = {
            let mut open = locked(&self.open);
            let gone: Vec<String> = open
                .iter()
                .filter(|(_, each)| now.duration_since(each.seen) >= IDLE)
                .map(|(credential, _)| credential.clone())
                .collect();
            gone.iter()
                .filter_map(|credential| open.remove(credential))
                .map(|each| each.run)
                .collect()
        };
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
