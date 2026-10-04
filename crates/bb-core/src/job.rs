//! Background jobs (ADR-0017): the record of one piece of work in the queue.

use jiff::Timestamp;

use crate::id::JobId;
use crate::personal::Entity;
use crate::technical::TechnicalName;

words! {
    /// Where a job is in its life.
    pub enum JobState {
        /// Waiting for its time, or being worked on while it holds a lease.
        Queued => "queued",
        /// Failed too often. It stays here for an administrator to look at (the dead-letter list).
        Dead => "dead",
    }
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
