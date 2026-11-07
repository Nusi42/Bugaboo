//! The names of the members of the events and job payloads of agent work, written once.

/// The members of the events.
pub(crate) mod member {
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
