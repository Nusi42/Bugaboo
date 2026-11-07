//! The names of the members of the events and job payloads of agent work, written once.

/// The members of the events.
pub(crate) mod member {
    /// What the work waits for when it is *Waiting for you*: a question or a plan.
    pub(crate) const WAITING_FOR: &str = "waiting_for";
    /// The agent.
    pub(crate) const AGENT: &str = "agent";
    /// The issues of a hand-over.
    pub(crate) const ISSUES: &str = "issues";
    /// The issue of a piece of work.
    pub(crate) const ISSUE: &str = "issue";
    /// The hand-over a piece of work belongs to.
    pub(crate) const HAND_OVER: &str = "hand_over";
    /// When something ends.
    pub(crate) const ENDS_AT: &str = "ends_at";
    /// The state of a hand-over, a rule or a piece of work.
    pub(crate) const STATE: &str = "state";
    /// The state of the agent work (the member the notifications read).
    pub(crate) const WORK_STATE: &str = "work_state";
    /// The answer of a review.
    pub(crate) const REVIEW: &str = "review";
    /// The saved search of a rule.
    pub(crate) const SEARCH: &str = "search";
    /// The project of a rule.
    pub(crate) const PROJECT: &str = "project";
    /// The fields a rule may change.
    pub(crate) const FIELDS: &str = "fields";
    /// The proposal that was applied or discarded.
    pub(crate) const PROPOSAL: &str = "proposal";
}

/// The members of the payloads of the jobs.
pub(crate) mod payload {
    /// The hand-over a job is about.
    pub(crate) const HAND_OVER: &str = "hand_over";
    /// The standing rule a job is about.
    pub(crate) const RULE: &str = "rule";
    /// The end of the rule the job was queued for.
    pub(crate) const ENDS_AT: &str = "ends_at";
    /// The project a rule job looks at.
    pub(crate) const PROJECT: &str = "project";
}

/// What an event of agent work says for the reader of notifications.
pub(crate) mod event_member {
    pub(crate) use super::member::{ISSUE, STATE, WAITING_FOR, WORK_STATE};

    /// The state of a standing rule three days before its end.
    pub(crate) const ABOUT_TO_RUN_OUT: &str = "about-to-run-out";
}

/// The values of the member `waiting_for`.
pub(crate) mod waiting_for {
    /// The work waits for the answer to a question.
    pub(crate) const QUESTION: &str = "question";
    /// The work waits for *Approve plan*.
    pub(crate) const PLAN: &str = "plan";
}
