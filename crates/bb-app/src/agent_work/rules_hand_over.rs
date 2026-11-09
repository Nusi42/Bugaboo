//! Standing rules that hand over what they find (AI-32, ADR-0029, amendment "Rules that hand
//! over"): each issue the saved search of the rule finds becomes one hand-over to the agent of the
//! rule, once, through the one `make_hand_over` that a person's hand-over uses too.

use bb_core::ErrorCode;
use bb_core::agent::Agent;
use bb_core::agent_records::StandingRule;
use bb_core::id::IssueId;

use super::hand_over::{ByRule, Giving, Reach};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};

impl<S: AuthStore> Ctx<'_, S> {
    /// Hands over one issue a rule found, written down as found in the same change.
    async fn hand_over_one(
        &self,
        actor: &Actor,
        agent: &Agent,
        reach: &Reach,
        giving: Giving,
        issue: IssueId,
        by: ByRule,
    ) -> Result<(), AppError> {
        self.plan_hand_over(actor, agent, reach, giving, &[issue.to_string()], Some(by))
            .await?
            .record_change(self.store())
            .await
            .map(drop)
    }

    /// Hands over the issues a rule found that it did not know yet, one hand-over for each, as the
    /// person of the rule, with the powers of the rule. Each is judged as a hand-over by that
    /// person is, now:
    ///
    /// - an issue the person may not hand over (any more) is left out and not written down: it is
    ///   looked at again at the next change in the project;
    /// - an issue the agent has open work on already (handed over by a person, or by another rule)
    ///   is written down as found and not handed over twice;
    /// - when the agent has as many open pieces of work as it may work on at once, this issue and
    ///   the rest are neither handed over nor written down: they are looked at again at the next
    ///   change in the project, and the agent's own work makes changes (ADR-0029, decision 9).
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(super) async fn hand_over_found(
        &self,
        rule: &StandingRule,
        actor: &Actor,
        agent: &Agent,
        fresh: Vec<IssueId>,
    ) -> Result<(), AppError> {
        let most_open = usize::try_from(agent.issues_at_once).unwrap_or(0);
        let mut already: Vec<IssueId> = Vec::new();
        // The fields of the rule are checked once for the look: a rule that names one no agent can
        // be given (an own field that was retired) is left alone, and the look goes on with the
        // other rules of the project.
        let reach = match self.hand_over_reach(rule.person, rule.fields.clone()).await {
            Ok(reach) => reach,
            Err(AppError::Refused(ErrorCode::InvalidRequest)) => return Ok(()),
            Err(other) => return Err(other),
        };
        for issue in fresh {
            let giving = Giving {
                note: String::new(),
                powers: rule.powers.clone(),
                budget_attempts: None,
                budget_minutes: None,
                budget_cost: None,
                plan_needs_approval: false,
                accept_by_itself: rule.accept_by_itself,
                days: None,
            };
            let by = ByRule {
                rule: rule.id,
                most_open,
            };
            match self
                .hand_over_one(actor, agent, &reach, giving, issue, by)
                .await
            {
                // Handed over, or left out for the person's rights of now (looked at again later).
                Ok(()) | Err(AppError::Refused(ErrorCode::Forbidden | ErrorCode::NotFound)) => {}
                Err(AppError::Refused(ErrorCode::AgentLimitReached)) => break,
                Err(AppError::Refused(ErrorCode::AgentWorkOpen)) => already.push(issue),
                Err(other) => return Err(other),
            }
        }
        if already.is_empty() {
            return Ok(());
        }
        self.write_down_found(rule, &already).await
    }
}
