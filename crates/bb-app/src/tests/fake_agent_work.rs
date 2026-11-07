//! The agent work part of the in-memory store (AI-31 to AI-35).

use bb_core::agent_records::{
    Attempt, AttemptResult, GivingState, HandOver, IssueWork, Proposal, ProposalState, Question,
    Review, RuleMatch, StandingRule,
};
use bb_core::agent_work::AgentWorkState;
use bb_core::id::{
    AgentId, AgentNoteId, AgentWorkId, AttemptId, HandOverId, IssueId, ProjectId, StandingRuleId,
    UserId,
};
use jiff::Timestamp;

use super::fake::{FakeError, Tx};
use crate::agent_work::{AgentWorkRecords, GrantStore};

/// Everything stored about agent work.
#[derive(Clone, Default)]
pub(crate) struct FakeAgentWork {
    pub(crate) hand_overs: Vec<HandOver>,
    pub(crate) rules: Vec<StandingRule>,
    pub(crate) works: Vec<IssueWork>,
    pub(crate) attempts: Vec<Attempt>,
    pub(crate) results: Vec<AttemptResult>,
    pub(crate) questions: Vec<Question>,
    pub(crate) reviews: Vec<Review>,
    pub(crate) proposals: Vec<Proposal>,
    pub(crate) matches: Vec<RuleMatch>,
}

impl FakeAgentWork {
    /// Whether the work is open: not an end, and its hand-over still gives work.
    pub(crate) fn is_open(&self, work: &IssueWork) -> bool {
        work.state.is_open()
            && !work.closed
            && self
                .hand_overs
                .iter()
                .any(|h| h.id == work.hand_over && h.state == GivingState::Open)
    }
}

fn page<T: Clone, I: Ord + Copy>(
    items: impl Iterator<Item = T>,
    id: impl Fn(&T) -> I,
    after: Option<I>,
    limit: u64,
) -> Vec<T> {
    let mut items: Vec<T> = items
        .filter(|item| after.is_none_or(|a| id(item) > a))
        .collect();
    items.sort_by_key(|item| id(item));
    items.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    items
}

impl GrantStore for super::fake::FakeStore {
    async fn hand_over_in(tx: &mut Tx, id: HandOverId) -> Result<Option<HandOver>, FakeError> {
        Ok(tx
            .state
            .work
            .hand_overs
            .iter()
            .find(|h| h.id == id)
            .cloned())
    }

    async fn standing_rule_in(
        tx: &mut Tx,
        id: StandingRuleId,
    ) -> Result<Option<StandingRule>, FakeError> {
        Ok(tx.state.work.rules.iter().find(|r| r.id == id).cloned())
    }

    async fn proposal_in(tx: &mut Tx, id: AgentNoteId) -> Result<Option<Proposal>, FakeError> {
        Ok(tx.state.work.proposals.iter().find(|p| p.id == id).cloned())
    }

    async fn rules_of_agent_in(
        tx: &mut Tx,
        agent: AgentId,
        after: Option<StandingRuleId>,
        limit: u64,
    ) -> Result<Vec<StandingRule>, FakeError> {
        Ok(page(
            tx.state
                .work
                .rules
                .iter()
                .filter(|r| r.agent == agent)
                .cloned(),
            |r| r.id,
            after,
            limit,
        ))
    }

    async fn open_rules_in_in(
        tx: &mut Tx,
        project: ProjectId,
    ) -> Result<Vec<StandingRule>, FakeError> {
        Ok(tx
            .state
            .work
            .rules
            .iter()
            .filter(|r| r.project == project && r.state == GivingState::Open)
            .cloned()
            .collect())
    }

    async fn proposals_on_in(
        tx: &mut Tx,
        issue: IssueId,
        state: Option<ProposalState>,
    ) -> Result<Vec<Proposal>, FakeError> {
        let mut found: Vec<Proposal> = tx
            .state
            .work
            .proposals
            .iter()
            .filter(|p| p.issue == issue && state.is_none_or(|s| p.state == s))
            .cloned()
            .collect();
        found.reverse();
        Ok(found)
    }

    async fn matches_of_in(
        tx: &mut Tx,
        rule: StandingRuleId,
        after: Option<IssueId>,
        limit: u64,
    ) -> Result<Vec<RuleMatch>, FakeError> {
        Ok(page(
            tx.state
                .work
                .matches
                .iter()
                .filter(|m| m.rule == rule)
                .cloned(),
            |m| m.issue,
            after,
            limit,
        ))
    }

    async fn insert_hand_over(tx: &mut Tx, hand_over: &HandOver) -> Result<(), FakeError> {
        tx.state.work.hand_overs.push(hand_over.clone());
        Ok(())
    }

    async fn end_hand_over(tx: &mut Tx, id: HandOverId, version: i64) -> Result<bool, FakeError> {
        Ok(tx
            .state
            .work
            .hand_overs
            .iter_mut()
            .find(|h| h.id == id && h.version == version)
            .map(|h| {
                h.state = GivingState::Ended;
                h.version += 1;
            })
            .is_some())
    }

    async fn insert_standing_rule(tx: &mut Tx, rule: &StandingRule) -> Result<(), FakeError> {
        tx.state.work.rules.push(rule.clone());
        Ok(())
    }

    async fn set_rule_end(
        tx: &mut Tx,
        id: StandingRuleId,
        ends_at: Timestamp,
        ended: bool,
        version: i64,
    ) -> Result<bool, FakeError> {
        Ok(tx
            .state
            .work
            .rules
            .iter_mut()
            .find(|r| r.id == id && r.version == version)
            .map(|r| {
                r.ends_at = ends_at;
                r.state = if ended {
                    GivingState::Ended
                } else {
                    GivingState::Open
                };
                r.version += 1;
            })
            .is_some())
    }

    async fn insert_proposal(tx: &mut Tx, proposal: &Proposal) -> Result<(), FakeError> {
        tx.state.work.proposals.push(proposal.clone());
        Ok(())
    }

    async fn set_proposal_state(
        tx: &mut Tx,
        id: AgentNoteId,
        state: ProposalState,
        by: UserId,
    ) -> Result<bool, FakeError> {
        Ok(tx
            .state
            .work
            .proposals
            .iter_mut()
            .find(|p| p.id == id && p.state == ProposalState::Open)
            .map(|p| {
                p.state = state;
                p.decided_by = Some(by);
            })
            .is_some())
    }

    async fn insert_match(tx: &mut Tx, rule_match: &RuleMatch) -> Result<bool, FakeError> {
        let known = tx
            .state
            .work
            .matches
            .iter()
            .any(|m| m.rule == rule_match.rule && m.issue == rule_match.issue);
        if !known {
            tx.state.work.matches.push(rule_match.clone());
        }
        Ok(!known)
    }

    async fn end_grants_of_agent(tx: &mut Tx, agent: AgentId) -> Result<(), FakeError> {
        for h in tx
            .state
            .work
            .hand_overs
            .iter_mut()
            .filter(|h| h.agent == agent)
        {
            h.state = GivingState::Ended;
        }
        for r in tx.state.work.rules.iter_mut().filter(|r| r.agent == agent) {
            r.state = GivingState::Ended;
        }
        Ok(())
    }
}

impl AgentWorkRecords for super::fake::FakeStore {
    async fn issue_work_in(tx: &mut Tx, id: AgentWorkId) -> Result<Option<IssueWork>, FakeError> {
        Ok(tx.state.work.works.iter().find(|w| w.id == id).cloned())
    }

    async fn open_work_of_in(
        tx: &mut Tx,
        agent: AgentId,
        issue: IssueId,
    ) -> Result<Option<IssueWork>, FakeError> {
        let work = &tx.state.work;
        Ok(work
            .works
            .iter()
            .find(|w| w.agent == agent && w.issue == issue && work.is_open(w))
            .cloned())
    }

    async fn attempts_of_in(tx: &mut Tx, work: AgentWorkId) -> Result<Vec<Attempt>, FakeError> {
        let mut found: Vec<Attempt> = tx
            .state
            .work
            .attempts
            .iter()
            .filter(|a| a.work == work)
            .cloned()
            .collect();
        found.sort_by_key(|a| a.number);
        Ok(found)
    }

    async fn attempt_in(tx: &mut Tx, id: AttemptId) -> Result<Option<Attempt>, FakeError> {
        Ok(tx.state.work.attempts.iter().find(|a| a.id == id).cloned())
    }

    async fn questions_of_in(tx: &mut Tx, work: AgentWorkId) -> Result<Vec<Question>, FakeError> {
        Ok(tx
            .state
            .work
            .questions
            .iter()
            .filter(|q| q.work == work)
            .cloned()
            .collect())
    }

    async fn question_in(tx: &mut Tx, id: AgentNoteId) -> Result<Option<Question>, FakeError> {
        Ok(tx.state.work.questions.iter().find(|q| q.id == id).cloned())
    }

    async fn results_of_in(
        tx: &mut Tx,
        attempt: AttemptId,
    ) -> Result<Vec<AttemptResult>, FakeError> {
        Ok(tx
            .state
            .work
            .results
            .iter()
            .filter(|r| r.attempt == attempt)
            .cloned()
            .collect())
    }

    async fn works_of_hand_over_in(
        tx: &mut Tx,
        hand_over: HandOverId,
    ) -> Result<Vec<IssueWork>, FakeError> {
        Ok(tx
            .state
            .work
            .works
            .iter()
            .filter(|w| w.hand_over == hand_over)
            .cloned()
            .collect())
    }

    async fn work_on_issue_in(tx: &mut Tx, issue: IssueId) -> Result<Vec<IssueWork>, FakeError> {
        let mut found: Vec<IssueWork> = tx
            .state
            .work
            .works
            .iter()
            .filter(|w| w.issue == issue)
            .cloned()
            .collect();
        found.reverse();
        Ok(found)
    }

    async fn work_of_agent_in(
        tx: &mut Tx,
        agent: AgentId,
        after: Option<AgentWorkId>,
        limit: u64,
    ) -> Result<Vec<IssueWork>, FakeError> {
        Ok(page(
            tx.state
                .work
                .works
                .iter()
                .filter(|w| w.agent == agent)
                .cloned(),
            |w| w.id,
            after,
            limit,
        ))
    }

    async fn reviews_of_in(tx: &mut Tx, work: AgentWorkId) -> Result<Vec<Review>, FakeError> {
        Ok(tx
            .state
            .work
            .reviews
            .iter()
            .filter(|r| r.work == work)
            .cloned()
            .collect())
    }

    async fn people_of_work_in(tx: &mut Tx, work: AgentWorkId) -> Result<Vec<UserId>, FakeError> {
        let state = &tx.state.work;
        let mut people = Vec::new();
        if let Some(found) = state.works.iter().find(|w| w.id == work) {
            people.push(found.person);
            if let Some(hand_over) = state.hand_overs.iter().find(|h| h.id == found.hand_over)
                && !people.contains(&hand_over.person)
            {
                people.push(hand_over.person);
            }
        }
        Ok(people)
    }

    async fn insert_work(tx: &mut Tx, work: &IssueWork) -> Result<(), FakeError> {
        tx.state.work.works.push(work.clone());
        Ok(())
    }

    async fn set_work_state(
        tx: &mut Tx,
        id: AgentWorkId,
        state: AgentWorkState,
        gave_up: Option<&str>,
        accepted_by_itself: bool,
        version: i64,
    ) -> Result<bool, FakeError> {
        Ok(tx
            .state
            .work
            .works
            .iter_mut()
            .find(|w| w.id == id && w.version == version)
            .map(|w| {
                w.state = state;
                w.gave_up = gave_up.map(str::to_owned);
                w.accepted_by_itself = accepted_by_itself;
                w.version += 1;
            })
            .is_some())
    }

    async fn close_work(
        tx: &mut Tx,
        id: AgentWorkId,
        accepted_by_itself: bool,
    ) -> Result<(), FakeError> {
        if let Some(w) = tx.state.work.works.iter_mut().find(|w| w.id == id) {
            w.closed = true;
            w.accepted_by_itself = accepted_by_itself;
        }
        Ok(())
    }

    async fn mark_work_seen(tx: &mut Tx, id: AgentWorkId) -> Result<(), FakeError> {
        if let Some(w) = tx.state.work.works.iter_mut().find(|w| w.id == id) {
            w.accepted_by_itself = false;
        }
        Ok(())
    }

    async fn insert_attempt(tx: &mut Tx, attempt: &Attempt) -> Result<(), FakeError> {
        tx.state.work.attempts.push(attempt.clone());
        Ok(())
    }

    async fn update_attempt(tx: &mut Tx, attempt: &Attempt) -> Result<(), FakeError> {
        if let Some(held) = tx
            .state
            .work
            .attempts
            .iter_mut()
            .find(|a| a.id == attempt.id)
        {
            *held = attempt.clone();
        }
        Ok(())
    }

    async fn insert_result(tx: &mut Tx, result: &AttemptResult) -> Result<(), FakeError> {
        tx.state.work.results.push(result.clone());
        Ok(())
    }

    async fn insert_question(tx: &mut Tx, question: &Question) -> Result<(), FakeError> {
        tx.state.work.questions.push(question.clone());
        Ok(())
    }

    async fn set_answer(
        tx: &mut Tx,
        id: AgentNoteId,
        answer: &str,
        by: UserId,
    ) -> Result<(), FakeError> {
        if let Some(q) = tx.state.work.questions.iter_mut().find(|q| q.id == id) {
            q.answer = Some(answer.to_owned());
            q.answered_by = Some(by);
        }
        Ok(())
    }

    async fn insert_review(tx: &mut Tx, review: &Review) -> Result<(), FakeError> {
        tx.state.work.reviews.push(review.clone());
        Ok(())
    }

    async fn stop_work_of_agent(
        tx: &mut Tx,
        agent: AgentId,
    ) -> Result<Vec<AgentWorkId>, FakeError> {
        let open: Vec<AgentWorkId> = {
            let work = &tx.state.work;
            work.works
                .iter()
                .filter(|w| w.agent == agent && work.is_open(w))
                .map(|w| w.id)
                .collect()
        };
        for w in &mut tx.state.work.works {
            if open.contains(&w.id) {
                w.state = AgentWorkState::Stopped;
                w.version += 1;
            }
        }
        Ok(open)
    }
}
