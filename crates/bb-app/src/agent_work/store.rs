//! The port of agent work: hand-overs, standing rules, the work on an issue and what belongs to
//! it (AI-31 to AI-35, ADR-0029). It also fills `AgentWorkStore`, which narrows the agent actor.
//!
//! Like the other ports, reads take `&self` and writes are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::agent_records::{
    Attempt, AttemptResult, HandOver, IssueWork, Proposal, ProposalState, Question, Review,
    RuleMatch, StandingRule,
};
use bb_core::agent_work::AgentWorkState;
use bb_core::id::{
    AgentId, AgentNoteId, AgentWorkId, AttemptId, HandOverId, IssueId, ProjectId, StandingRuleId,
    UserId,
};
use jiff::Timestamp;

use crate::agents::AgentWorkStore;
use crate::store_macros::{queries, writes};

/// What hand-overs, standing rules and what they propose need from storage.
pub trait GrantStore: AgentWorkStore + 'static {
    queries! {
        /// The hand-over with this ID.
        fn hand_over(id: HandOverId) -> Option<HandOver> = hand_over_in;
        /// The standing rule with this ID.
        fn standing_rule(id: StandingRuleId) -> Option<StandingRule> = standing_rule_in;
        /// The proposal with this ID.
        fn proposal(id: AgentNoteId) -> Option<Proposal> = proposal_in;
        /// The standing rules of one agent, by ID, after `after`, at most `limit`.
        fn rules_of_agent(agent: AgentId, after: Option<StandingRuleId>, limit: u64) -> Vec<StandingRule> = rules_of_agent_in;
        /// Every standing rule that has not ended.
        fn open_rules() -> Vec<StandingRule> = every_open_rule_in;
        /// The standing rules that work in a project and have not ended.
        fn open_rules_in(project: ProjectId) -> Vec<StandingRule> = open_rules_in_in;
        /// The proposals on an issue, newest first.
        fn proposals_on(issue: IssueId, state: Option<ProposalState>) -> Vec<Proposal> = proposals_on_in;
        /// The issues that came under a rule, by issue, after `after`, at most `limit`.
        fn matches_of(rule: StandingRuleId, after: Option<IssueId>, limit: u64) -> Vec<RuleMatch> = matches_of_in;
    }

    writes! {
        /// Adds a hand-over.
        fn insert_hand_over(hand_over: &HandOver) -> ();
        /// Ends a hand-over that is at `version`; whether it was.
        fn end_hand_over(id: HandOverId, version: i64) -> bool;
        /// Adds a standing rule.
        fn insert_standing_rule(rule: &StandingRule) -> ();
        /// Ends a rule, or renews it to `ends_at`, counting its version up; whether it was at
        /// `version`.
        fn set_rule_end(id: StandingRuleId, ends_at: Timestamp, ended: bool, version: i64) -> bool;
        /// Adds a proposal.
        fn insert_proposal(proposal: &Proposal) -> ();
        /// Writes what became of a proposal that was open; whether it was open.
        fn set_proposal_state(id: AgentNoteId, state: ProposalState, by: UserId) -> bool;
        /// Writes down that an issue came under a rule; whether it was new.
        fn insert_match(rule_match: &RuleMatch) -> bool;
        /// Ends every open hand-over and standing rule of an agent.
        fn end_grants_of_agent(agent: AgentId) -> ();
    }
}

/// What the work on an issue needs from storage.
pub trait AgentWorkRecords: GrantStore + 'static {
    queries! {
        /// The work with this ID.
        fn issue_work(id: AgentWorkId) -> Option<IssueWork> = issue_work_in;
        /// The work of an agent on an issue that has not ended, if there is one: a hand-over is
        /// open once for an issue and an agent.
        fn open_work_of(agent: AgentId, issue: IssueId) -> Option<IssueWork> = open_work_of_in;
        /// The attempts of a piece of work, oldest first.
        fn attempts_of(work: AgentWorkId) -> Vec<Attempt> = attempts_of_in;
        /// The attempt with this ID.
        fn attempt(id: AttemptId) -> Option<Attempt> = attempt_in;
        /// The questions of a piece of work, oldest first.
        fn questions_of(work: AgentWorkId) -> Vec<Question> = questions_of_in;
        /// What the attempt attached.
        fn results_of(attempt: AttemptId) -> Vec<AttemptResult> = results_of_in;
        /// The question with this ID.
        fn question(id: AgentNoteId) -> Option<Question> = question_in;
        /// The work of a hand-over, one piece for each issue.
        fn works_of_hand_over(hand_over: HandOverId) -> Vec<IssueWork> = works_of_hand_over_in;
        /// All the work on an issue, newest first.
        fn work_on_issue(issue: IssueId) -> Vec<IssueWork> = work_on_issue_in;
        /// The work of one agent, by ID, after `after`, at most `limit`.
        fn work_of_agent(agent: AgentId, after: Option<AgentWorkId>, limit: u64) -> Vec<IssueWork> = work_of_agent_in;
        /// The reviews of a piece of work, oldest first.
        fn reviews_of(work: AgentWorkId) -> Vec<Review> = reviews_of_in;
        /// The people who must hear about work: the person responsible for the agent and the person
        /// who handed over, without repeating one.
        fn people_of_work(work: AgentWorkId) -> Vec<UserId> = people_of_work_in;
    }

    writes! {
        /// Adds the work on an issue.
        fn insert_work(work: &IssueWork) -> ();
        /// Writes the state of the work, counting its version up; whether it was at `version`.
        fn set_work_state(
            id: AgentWorkId,
            state: AgentWorkState,
            gave_up: Option<&str>,
            accepted_by_itself: bool,
            version: i64
        ) -> bool;
        /// Ends the work with a review; whether it was accepted by itself and still waits for a
        /// person to look at it.
        fn close_work(id: AgentWorkId, accepted_by_itself: bool, status_pending: bool) -> ();
        /// Marks that a person looked at work that was accepted by itself.
        fn mark_work_seen(id: AgentWorkId) -> ();
        /// Adds an attempt.
        fn insert_attempt(attempt: &Attempt) -> ();
        /// Writes what the attempt now holds: its plan, the approval, what the agent reported and
        /// the end.
        fn update_attempt(attempt: &Attempt) -> ();
        /// Adds what an attempt attached.
        fn insert_result(result: &AttemptResult) -> ();
        /// Adds a question.
        fn insert_question(question: &Question) -> ();
        /// Writes the answer to a question.
        fn set_answer(id: AgentNoteId, answer: &str, by: UserId) -> ();
        /// Adds a review.
        fn insert_review(review: &Review) -> ();
        /// Ends the open work of an agent as *Stopped*; the work that was open.
        fn stop_work_of_agent(agent: AgentId) -> Vec<AgentWorkId>;
    }
}
