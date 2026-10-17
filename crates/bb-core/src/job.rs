//! Background jobs (ADR-0017): the record of one piece of work in the queue.

use jiff::Timestamp;

use crate::id::JobId;
use crate::personal::Entity;
use crate::technical::TechnicalName;

/// How long the payload of a job that died is kept, in days (M1-F21, PRIV-05): it can hold the
/// address and the text of a person, so it is emptied when this has passed. The job itself stays on
/// the dead-letter list for an administrator to look at.
pub const DEAD_PAYLOAD_KEEP_DAYS: i64 = 30;

/// The moment before which the payload of a dead job is too old to keep, at `now`.
#[must_use]
pub fn dead_payload_cutoff(now: Timestamp) -> Option<Timestamp> {
    now.checked_sub(jiff::SignedDuration::from_secs(
        DEAD_PAYLOAD_KEEP_DAYS * crate::time::SECONDS_PER_DAY,
    ))
    .ok()
}

words! {
    /// Where a job is in its life.
    pub enum JobState {
        /// Waiting for its time, or being worked on while it holds a lease.
        Queued => "queued",
        /// Failed too often. It stays here for an administrator to look at (the dead-letter list).
        Dead => "dead",
    }
}

words! {
    /// Whether another start of a job that failed can help.
    pub enum FailureKind {
        /// The work may succeed later (the store broke, a lease ran out): it waits and runs again,
        /// until it has had all its starts.
        Temporary => "temporary",
        /// The work can never succeed as it was asked for (the payload names nothing, the request
        /// was refused): it goes to the dead-letter list at once.
        Permanent => "permanent",
    }
}

/// What happened to a job that a worker reported as failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureOutcome {
    /// It will run again at this moment.
    Retry(Timestamp),
    /// It failed too often, or for good, and is on the dead-letter list.
    Dead,
}

/// One job in the queue.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Job {
    /// The ID of the job.
    #[not_personal]
    pub id: JobId,
    /// What kind of work it is, like `mail.send`. The runner picks the handler by this name.
    #[not_personal]
    pub kind: TechnicalName,
    /// What the work needs to know. It can name people or hold their text (for example the
    /// recipient of an e-mail), so it is deleted with the job.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub payload: serde_json::Value,
    /// The earliest moment the job may run.
    #[not_personal]
    pub run_at: Timestamp,
    /// How many times a worker has started it.
    #[not_personal]
    pub attempts: u32,
    /// After how many starts it goes to the dead-letter list.
    #[not_personal]
    pub max_attempts: u32,
    /// Waiting or dead.
    #[not_personal]
    pub state: JobState,
    /// The worker that holds the lease, if any.
    #[not_personal]
    pub lease_owner: Option<TechnicalName>,
    /// Until when the lease holds. After this moment another worker may take the job.
    #[not_personal]
    pub lease_until: Option<Timestamp>,
    /// The code of the last failure (a technical name, never the message of an error).
    #[not_personal]
    pub last_failure: Option<TechnicalName>,
    /// When the job was queued.
    #[not_personal]
    pub created_at: Timestamp,
}
