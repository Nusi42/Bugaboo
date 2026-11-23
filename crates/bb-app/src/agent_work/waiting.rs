//! *Wait for work* (AI-36, ADR-0029, amendment M9): an agent asks to be answered when work waits for
//! it, or after some seconds, in one call. The answer is what *changes since* answers for the same
//! mark, from the same use case, so there is no second query of what changed.
//!
//! The call is decided on what is stored, never on whether a signal was seen: work *Waiting for the
//! agent* (a hand-over it has not taken, or work sent back) answers at once. Otherwise the call waits
//! on the agent's signal, which every change that gives the agent such work rings after its commit,
//! and reads again. While it waits it holds no connection of the database and no lock: only an
//! entry of the agent in [`WorkSignal`]. Bugaboo runs as one process, so the signal reaches every
//! waiting call.
//!
//! A waiting call holds one connection of the listener, so the waits are kept fair: one agent waits
//! once at a time, and at most a share of the listener's connections wait at once. A call that may
//! not wait answers at once and says so.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use bb_core::ErrorCode;
use bb_core::agent_work::{AgentWorkState, WAIT_SECONDS_DEFAULT, WAIT_SECONDS_MOST};
use bb_core::id::AgentId;
use bb_policy::Authorized;
use serde::Serialize;

use super::changes::{ChangesPage, ChangesSinceQuery, MOST_WORK_SEEN, changes_since};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::ports::BoxFuture;

/// The part of the listener's connections that may wait at once: a quarter, so waiting agents never
/// take the connections people need.
pub const WAITING_SHARE_OF_CONNECTIONS: usize = 4;

/// The port that lets time pass: the server gives the one of its runtime. `bb-app` runs on no
/// runtime of its own.
pub trait Pause: Send + Sync {
    /// A future that is ready after `length`.
    fn pause(&self, length: Duration) -> BoxFuture<'static, ()>;
}

/// The bell of one waiting call: rung once, heard once.
#[derive(Default)]
struct Bell {
    state: Mutex<(bool, Option<Waker>)>,
}

impl Bell {
    fn ring(&self) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.0 = true;
        if let Some(waker) = state.1.take() {
            waker.wake();
        }
    }
}

/// Ready when the bell rang or the pause is over, whichever comes first; says whether the bell rang.
struct RungOrOver<'a> {
    bell: &'a Bell,
    over: BoxFuture<'static, ()>,
}

impl Future for RungOrOver<'_> {
    type Output = bool;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<bool> {
        {
            let mut state = self
                .bell
                .state
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if state.0 {
                return Poll::Ready(true);
            }
            state.1 = Some(context.waker().clone());
        }
        self.over.as_mut().poll(context).map(|()| false)
    }
}

/// The agents that wait for work now, each with the bell its call waits on.
pub struct WorkSignal {
    waiting: Mutex<HashMap<AgentId, Arc<Bell>>>,
    /// How many calls may wait at once.
    most: usize,
    /// How time passes; without it no call waits.
    pause: Option<Arc<dyn Pause>>,
    /// Whether the server stops: no call waits any more.
    closed: AtomicBool,
}

impl Default for WorkSignal {
    /// No call waits: a server gives the pause of its runtime ([`WorkSignal::for_connections`]).
    fn default() -> Self {
        Self {
            waiting: Mutex::default(),
            most: 0,
            pause: None,
            closed: AtomicBool::new(false),
        }
    }
}

impl WorkSignal {
    /// The signal of a server whose listener holds at most `connections` at once, and whose
    /// runtime lets time pass with `pause`.
    #[must_use]
    pub fn for_connections(connections: usize, pause: Arc<dyn Pause>) -> Self {
        Self {
            waiting: Mutex::default(),
            most: (connections / WAITING_SHARE_OF_CONNECTIONS).max(1),
            pause: Some(pause),
            closed: AtomicBool::new(false),
        }
    }

    /// Tells the waiting call of `agent`, if there is one, that work came. A call that has not
    /// started to wait yet keeps it and does not wait.
    pub fn ring(&self, agent: AgentId) {
        let waiting = self.waiting.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(bell) = waiting.get(&agent) {
            bell.ring();
        }
    }

    /// Ends every wait, because the server stops: each waiting call answers now, and no call
    /// waits after, so the stop does not wait for the seconds of the waits.
    pub fn ring_all(&self) {
        let waiting = self.waiting.lock().unwrap_or_else(PoisonError::into_inner);
        self.closed.store(true, Ordering::SeqCst);
        for bell in waiting.values() {
            bell.ring();
        }
    }

    /// The entry of a call of `agent` that may wait: none when the agent waits already, as many
    /// calls wait as may, time cannot pass, or the server stops.
    fn enter(&self, agent: AgentId) -> Option<Waiting<'_>> {
        let pause = Arc::clone(self.pause.as_ref()?);
        let mut waiting = self.waiting.lock().unwrap_or_else(PoisonError::into_inner);
        if self.closed.load(Ordering::SeqCst)
            || waiting.contains_key(&agent)
            || waiting.len() >= self.most
        {
            return None;
        }
        let bell = Arc::new(Bell::default());
        waiting.insert(agent, Arc::clone(&bell));
        Some(Waiting {
            signals: self,
            agent,
            bell,
            pause,
        })
    }

    /// How time passes on the server's runtime; `None` when the server gave none.
    pub(crate) fn pause(&self) -> Option<&dyn Pause> {
        self.pause.as_deref()
    }

    /// How many calls wait now.
    #[must_use]
    pub fn waiting_now(&self) -> usize {
        self.waiting
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

/// The entry of one waiting call; it goes when the call ends, however it ends.
struct Waiting<'a> {
    signals: &'a WorkSignal,
    agent: AgentId,
    bell: Arc<Bell>,
    pause: Arc<dyn Pause>,
}

impl Waiting<'_> {
    /// Waits until the bell rings or `length` is over; says whether it rang.
    async fn rung_within(&self, length: Duration) -> bool {
        RungOrOver {
            bell: &self.bell,
            over: self.pause.pause(length),
        }
        .await
    }
}

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        self.signals
            .waiting
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.agent);
    }
}

/// Wait for work, then ask what changed since a mark.
pub struct WaitForWorkQuery {
    /// The secret of the key of the agent.
    pub session_token: String,
    /// The mark the agent got last time, as for *changes since*.
    pub since: Option<String>,
    /// How many changes a page holds at most.
    pub limit: Option<u32>,
    /// How many seconds to wait at most: 1 to [`WAIT_SECONDS_MOST`], [`WAIT_SECONDS_DEFAULT`] when
    /// not given.
    pub seconds: Option<u64>,
}

/// What *Wait for work* answers: what changed since the mark, and whether work waits now.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WaitedPage {
    /// What *changes since* answers for the same mark.
    pub page: ChangesPage,
    /// Whether work waits for the agent now: a hand-over it has not taken, or work sent back.
    pub work_waiting: bool,
    /// Whether the call waited: it does not when work waited already, when the agent waits in
    /// another call, or when as many calls wait as may.
    pub waited: bool,
}

impl bb_policy::Masked for WaitedPage {}

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether work waits for the agent that it can take: one read of its open work, and for work
    /// *Waiting for the agent* whether another agent has an attempt running on its issue, which
    /// *take* refuses. Such work does not count, so a call does not answer again and again while
    /// the other agent holds the issue.
    async fn work_waits_for(&self, agent: AgentId) -> Result<bool, AppError> {
        let failed = |error| internal::<S>(&error);
        let open = self
            .store()
            .current_work_of_agent(agent, self.now(), MOST_WORK_SEEN)
            .await
            .map_err(failed)?;
        for work in open
            .iter()
            .filter(|work| work.state == AgentWorkState::WaitingForTheAgent)
        {
            let held = self
                .store()
                .another_in_attempt(work.issue, work.id)
                .await
                .map_err(failed)?;
            if !held {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Rings the signal of the agent: work came that waits for it. Called after the commit.
    pub(crate) fn work_came_for(&self, agent: AgentId) {
        self.services.work_signal.ring(agent);
    }

    /// Ends the wait of the agent, which was paused or stopped: its waiting call answers now
    /// instead of after its seconds. Called after the commit.
    pub(crate) fn wait_ends_for(&self, agent: AgentId) {
        self.services.work_signal.ring(agent);
    }
}

/// Waits until work waits for the agent or the seconds are over, then answers what *changes since*
/// answers for the same mark, and whether work waits now.
///
/// # Errors
/// `access.forbidden` for a request that does not come with the key of an agent,
/// `request.invalid` for seconds out of range or a mark that is not one.
pub async fn wait_for_work<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: WaitForWorkQuery,
) -> Result<Authorized<WaitedPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let Some(agent) = actor.agent() else {
        return Err(ErrorCode::Forbidden.into());
    };
    let seconds = input.seconds.unwrap_or(WAIT_SECONDS_DEFAULT);
    if !(1..=WAIT_SECONDS_MOST).contains(&seconds) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    // The entry comes before the read: work that comes between the two rings a signal that is
    // kept, so nothing is missed.
    let entry = ctx.services.work_signal.enter(agent);
    let mut work_waiting = ctx.work_waits_for(agent).await?;
    let mut waited = false;
    if let Some(entry) = &entry
        && !work_waiting
    {
        waited = true;
        if entry.rung_within(Duration::from_secs(seconds)).await {
            work_waiting = ctx.work_waits_for(agent).await?;
        }
    }
    drop(entry);
    let page = changes_since(
        ctx,
        ChangesSinceQuery {
            session_token: input.session_token,
            since: input.since,
            limit: input.limit,
        },
    )
    .await?;
    Ok(page.map(|page| WaitedPage {
        page,
        work_waiting,
        waited,
    }))
}
