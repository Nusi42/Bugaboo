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
use bb_core::agent_work::{AgentWorkState, StopCause};
use bb_core::event::DomainEvent;
use bb_core::id::{
    AgentId, AgentNoteId, AgentWorkId, AttemptId, HandOverId, IssueId, ProjectId, StandingRuleId,
    UserId,
};
use jiff::Timestamp;

use crate::agents::AgentWorkStore;
use crate::store_macros::{queries, reads, writes};

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
        /// The proposals on an issue, newest first, at most `limit`; only those of `agent` when it
        /// is given.
        fn proposals_on(issue: IssueId, state: Option<ProposalState>, agent: Option<AgentId>, limit: u64) -> Vec<Proposal> = proposals_on_in;
        /// Which of these issues came under the rule already.
        fn known_matches(rule: StandingRuleId, issues: Vec<IssueId>) -> Vec<IssueId> = known_matches_in;
        /// The issues that came under a rule, by issue, after `after`, at most `limit`.
        fn matches_of(rule: StandingRuleId, after: Option<IssueId>, limit: u64) -> Vec<RuleMatch> = matches_of_in;
    }

    writes! {
        /// Adds a hand-over.
        fn insert_hand_over(hand_over: &HandOver) -> ();
        /// Ends a hand-over that is at `version`; whether it was.
        fn end_hand_over(id: HandOverId, version: i64) -> bool;
        /// Counts the version of an open hand-over up if it is at `version`, so that two changes to
        /// the work of one hand-over at the same moment cannot both pass (the second finds the
        /// version changed, also where a read does not wait for a write); whether it was.
        fn touch_hand_over(id: HandOverId, version: i64) -> bool;
        /// Adds a standing rule.
        fn insert_standing_rule(rule: &StandingRule) -> ();
        /// Ends a rule, or renews it to `ends_at`, counting its version up; whether it was at
        /// `version`.
        fn set_rule_end(id: StandingRuleId, ends_at: Timestamp, ended: bool, version: i64) -> bool;
        /// Adds a proposal.
        fn insert_proposal(proposal: &Proposal) -> ();
        /// Writes what became of a proposal that was open; whether it was open.
        fn set_proposal_state(id: AgentNoteId, state: ProposalState, by: UserId) -> bool;
        /// Opens a proposal again that `by` applied or discarded and that could not be carried
        /// out; whether it was.
        fn reopen_proposal(id: AgentNoteId, by: UserId) -> bool;
        /// Writes down that an issue came under a rule; whether it was new.
        fn insert_match(rule_match: &RuleMatch) -> bool;
        /// Ends every open hand-over and standing rule of an agent.
        fn end_grants_of_agent(agent: AgentId) -> ();
    }
}

/// Everything that belongs to a number of pieces of work, read together: a page of work costs the
/// same few statements whatever its length (PERF-N1).
#[derive(Clone, Debug, Default)]
pub struct WorkDetails {
    /// The hand-overs the work was given by.
    pub hand_overs: Vec<HandOver>,
    /// The questions, oldest first.
    pub questions: Vec<Question>,
    /// The attempts, oldest first.
    pub attempts: Vec<Attempt>,
    /// What the attempts attached.
    pub results: Vec<AttemptResult>,
    /// The reviews, oldest first.
    pub reviews: Vec<Review>,
}

/// What the work on an issue needs from storage.
pub trait AgentWorkRecords: GrantStore + 'static {
    reads! {
        /// The issues that came under the open rules of an agent after `since`, oldest first, at
        /// most `limit`: what *changes since* tells of the matches of its rules.
        fn matches_of_agent_since(agent: AgentId, now: Timestamp, since: (Timestamp, Option<(IssueId, StandingRuleId)>), limit: u64) -> Vec<RuleMatch>;
        /// The events about issues that were committed after the place `after` in the order they
        /// were committed, with their places, at most `limit`.
        fn issue_events_after(after: i64, limit: u64) -> Vec<(i64, DomainEvent)>;
        /// The place of the newest event in the order they were committed; 0 when there is none.
        fn last_event_place() -> i64;
    }

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
        /// The work on an issue, newest first, at most `limit`; only the work of `agent` when it is
        /// given.
        fn work_on_issue(issue: IssueId, agent: Option<AgentId>, limit: u64) -> Vec<IssueWork> = work_on_issue_in;
        /// The work on these issues that is open at `now`, newest first: not ended or closed, and its
        /// hand-over has neither ended nor run out. The one definition of *open at `now`* that the
        /// work of an agent uses too, so a list row and the issue agree (PERF-N1).
        fn open_work_on_issues(issues: Vec<IssueId>, now: Timestamp) -> Vec<IssueWork> = open_work_on_issues_in;
        /// What belongs to these pieces of work, read together.
        fn details_of_works(works: Vec<IssueWork>) -> WorkDetails = details_of_works_in;
        /// Whether an agent has anything left to end: open work, an open hand-over or an open rule.
        fn agent_has_open_giving(agent: AgentId) -> bool = agent_has_open_giving_in;
        /// The work of one agent, by ID, after `after`, at most `limit`.
        fn work_of_agent(agent: AgentId, after: Option<AgentWorkId>, limit: u64) -> Vec<IssueWork> = work_of_agent_in;
        /// How many issues the agent has an attempt running on now (the limit of issues at once).
        fn working_issues_of(agent: AgentId) -> u64 = working_issues_of_in;
        /// Whether another piece of work on the issue has an attempt running (all the work on the
    /// issue is looked at, however much there is).
    fn another_in_attempt(issue: IssueId, except: AgentWorkId) -> bool = another_in_attempt_in;
        /// The work that is current for these agents, newest first, at most `limit`: work that has
        /// not ended and whose hand-over still gives work, and work that was accepted by itself
        /// and that nobody has looked at yet (*My agents*).
        fn current_work_of_agents(agents: Vec<AgentId>, limit: u64) -> Vec<IssueWork> = current_work_of_agents_in;
        /// The reviews of a piece of work, oldest first.
        fn reviews_of(work: AgentWorkId) -> Vec<Review> = reviews_of_in;
        /// The people who must hear about work: the person responsible for the agent and the person
        /// who handed over, without repeating one.
        fn people_of_work(work: AgentWorkId) -> Vec<UserId> = people_of_work_in;
    }

    writes! {
        /// Takes the lock of the taking of one issue, so that two agents who take it at the same
        /// moment are judged one after the other; does nothing where writes cannot overlap.
        fn lock_taking_of(issue: IssueId) -> ();
        /// Takes the lock of the attempts of one agent, so that two takes of it at the same moment
    /// are judged one after the other; does nothing where writes cannot overlap. Taken after
    /// `lock_taking_of`.
        fn lock_working_of(agent: AgentId) -> ();
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
        /// Ends the work with a review, counting its version up; whether it was at `version`.
        /// `accepted_by_itself` and `status_pending` say whether it still waits for a person to
        /// look at it.
        fn close_work(id: AgentWorkId, accepted_by_itself: bool, status_pending: bool, version: i64) -> bool;
        /// Marks that a person looked at work that was accepted by itself, counting its version
        /// up; whether it was at `version`.
        fn mark_work_seen(id: AgentWorkId, version: i64) -> bool;
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
        fn set_answer(id: AgentNoteId, answer: &str, by: UserId, at: Timestamp) -> ();
        /// Notes why a piece of work stopped.
        fn set_stop_cause(id: AgentWorkId, cause: StopCause) -> ();
        /// Adds a review.
        fn insert_review(review: &Review) -> ();
        /// Ends the open work of an agent as *Stopped*; the work that was open.
        fn stop_work_of_agent(agent: AgentId, cause: StopCause) -> Vec<AgentWorkId>;
    }
}
