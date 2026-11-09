//! The hand-overs, standing rules and proposals of the in-memory store (AI-31 to AI-35).

use bb_core::agent_records::{
    GivingState, HandOver, Proposal, ProposalState, RuleMatch, StandingRule,
};
use bb_core::id::{AgentId, AgentNoteId, HandOverId, IssueId, ProjectId, StandingRuleId, UserId};
use jiff::Timestamp;

use super::fake::{FakeError, Tx};
use super::fake_agent_work::page;
use crate::agent_work::{GrantStore, RuleCursor};

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

    async fn open_rules_of_agents_in(
        tx: &mut Tx,
        agents: Vec<AgentId>,
        limit: u64,
    ) -> Result<Vec<StandingRule>, FakeError> {
        // Newest first, the ID breaking a tie: as the store reads them.
        let mut found: Vec<StandingRule> = tx
            .state
            .work
            .rules
            .iter()
            .filter(|r| agents.contains(&r.agent) && r.state == GivingState::Open)
            .cloned()
            .collect();
        found.sort_by_key(|r| (std::cmp::Reverse(r.made_at), r.id));
        found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
        Ok(found)
    }

    async fn hand_over_rule_projects_in(
        tx: &mut Tx,
        agent: AgentId,
        now: Timestamp,
    ) -> Result<Vec<ProjectId>, FakeError> {
        let mut projects: Vec<ProjectId> = tx
            .state
            .work
            .rules
            .iter()
            .filter(|r| {
                r.agent == agent
                    && r.way == bb_core::agent_work::RuleWay::HandOver
                    && r.works_at(now)
            })
            .map(|r| r.project)
            .collect();
        projects.sort();
        projects.dedup();
        Ok(projects)
    }

    async fn rules_of_agent_in(
        tx: &mut Tx,
        agent: AgentId,
        after: Option<RuleCursor>,
        limit: u64,
    ) -> Result<Vec<StandingRule>, FakeError> {
        let mut found: Vec<StandingRule> = tx
            .state
            .work
            .rules
            .iter()
            .filter(|r| r.agent == agent)
            .cloned()
            .collect();
        // Those that work first, newest first in each.
        let key = |r: &StandingRule| {
            (
                r.state != GivingState::Open,
                std::cmp::Reverse(r.made_at),
                std::cmp::Reverse(r.id),
            )
        };
        found.sort_by_key(key);
        let rest = found.into_iter().skip_while(|r| {
            after.is_some_and(|cursor| {
                key(r)
                    <= (
                        !cursor.open,
                        std::cmp::Reverse(cursor.made_at),
                        std::cmp::Reverse(cursor.id),
                    )
            })
        });
        Ok(rest
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .collect())
    }

    async fn every_open_rule_in(tx: &mut Tx) -> Result<Vec<StandingRule>, FakeError> {
        Ok(tx
            .state
            .work
            .rules
            .iter()
            .filter(|r| r.state == GivingState::Open)
            .cloned()
            .collect())
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

    async fn known_matches_in(
        tx: &mut Tx,
        rule: StandingRuleId,
        issues: Vec<IssueId>,
    ) -> Result<Vec<IssueId>, FakeError> {
        Ok(tx
            .state
            .work
            .matches
            .iter()
            .filter(|found| found.rule == rule && issues.contains(&found.issue))
            .map(|found| found.issue)
            .collect())
    }

    async fn proposals_on_in(
        tx: &mut Tx,
        issue: IssueId,
        state: Option<ProposalState>,
        agent: Option<AgentId>,
        limit: u64,
    ) -> Result<Vec<Proposal>, FakeError> {
        let mut found: Vec<Proposal> = tx
            .state
            .work
            .proposals
            .iter()
            .filter(|p| {
                p.issue == issue
                    && state.is_none_or(|s| p.state == s)
                    && agent.is_none_or(|agent| p.agent == agent)
            })
            .cloned()
            .collect();
        found.reverse();
        found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
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
        if tx.state.hand_overs_cannot_end {
            return Ok(false);
        }
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

    async fn touch_hand_over(tx: &mut Tx, id: HandOverId, version: i64) -> Result<bool, FakeError> {
        let failing = tx
            .state
            .touches_fail
            .fetch_update(
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
                |left| left.checked_sub(1),
            )
            .is_ok();
        if failing {
            return Ok(false);
        }
        Ok(tx
            .state
            .work
            .hand_overs
            .iter_mut()
            .find(|h| h.id == id && h.version == version && h.state == GivingState::Open)
            .map(|h| h.version += 1)
            .is_some())
    }

    async fn insert_standing_rule(tx: &mut Tx, rule: &StandingRule) -> Result<(), FakeError> {
        tx.state.work.rules.push(rule.clone());
        Ok(())
    }

    async fn set_rule_end(
        tx: &mut Tx,
        id: StandingRuleId,
        ends_at: Option<Timestamp>,
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

    async fn reopen_proposal(tx: &mut Tx, id: AgentNoteId, by: UserId) -> Result<bool, FakeError> {
        Ok(tx
            .state
            .work
            .proposals
            .iter_mut()
            .find(|p| p.id == id && p.state != ProposalState::Open && p.decided_by == Some(by))
            .map(|p| {
                p.state = ProposalState::Open;
                p.decided_by = None;
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
            .filter(|h| h.agent == agent && h.state == GivingState::Open)
        {
            h.state = GivingState::Ended;
            h.version += 1;
        }
        for r in tx
            .state
            .work
            .rules
            .iter_mut()
            .filter(|r| r.agent == agent && r.state == GivingState::Open)
        {
            r.state = GivingState::Ended;
            r.version += 1;
        }
        Ok(())
    }
}
