//! Hand-overs, standing rules, their proposals and matches.

use bb_app::agent_work::{GrantStore, RuleCursor};

use super::*;

impl GrantStore for AuthStorage {
    async fn hand_over_in(
        transaction: &mut Transaction,
        id: HandOverId,
    ) -> Result<Option<HandOver>, StoreError> {
        let mut select = hand_over_select();
        select.and_where(Expr::col(HandOverRow::Id).eq(id.stored()));
        first(transaction, &select, read_hand_over).await
    }

    async fn standing_rule_in(
        transaction: &mut Transaction,
        id: StandingRuleId,
    ) -> Result<Option<StandingRule>, StoreError> {
        let mut select = rule_select();
        select.and_where(Expr::col(StandingRuleRow::Id).eq(id.stored()));
        first(transaction, &select, read_rule).await
    }

    async fn proposal_in(
        transaction: &mut Transaction,
        id: AgentNoteId,
    ) -> Result<Option<Proposal>, StoreError> {
        let mut select = proposal_select();
        select.and_where(Expr::col(ProposalRow::Id).eq(id.stored()));
        first(transaction, &select, read_proposal).await
    }

    async fn rules_of_agent_in(
        transaction: &mut Transaction,
        agent: AgentId,
        cursor: Option<RuleCursor>,
        limit: u64,
    ) -> Result<Vec<StandingRule>, StoreError> {
        let works = Expr::col(StandingRuleRow::State).eq(GivingState::Open.code());
        let mut select = rule_select();
        select.and_where(Expr::col(StandingRuleRow::Agent).eq(agent.stored()));
        if let Some(cursor) = cursor {
            // Those that work come first, newest first; then those that ended, newest first.
            let made = to_micros(cursor.made_at);
            let older = sea_query::Cond::any()
                .add(Expr::col(StandingRuleRow::MadeAt).lt(made))
                .add(
                    sea_query::Cond::all()
                        .add(Expr::col(StandingRuleRow::MadeAt).eq(made))
                        .add(Expr::col(StandingRuleRow::Id).lt(cursor.id.stored())),
                );
            if cursor.open {
                select.and_where(
                    sea_query::Cond::any()
                        .add(Expr::col(StandingRuleRow::State).ne(GivingState::Open.code()))
                        .add(older)
                        .into(),
                );
            } else {
                select.and_where(works.clone().not());
                select.and_where(older.into());
            }
        }
        select
            .order_by_expr(works, Order::Desc)
            .order_by(StandingRuleRow::MadeAt, Order::Desc)
            .order_by(StandingRuleRow::Id, Order::Desc)
            .limit(limit);
        all(transaction, &select, read_rule).await
    }

    async fn open_rules_of_agents_in(
        transaction: &mut Transaction,
        agents: Vec<AgentId>,
        limit: u64,
    ) -> Result<Vec<StandingRule>, StoreError> {
        if agents.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = rule_select();
        select
            .and_where(
                Expr::col(StandingRuleRow::Agent).is_in(
                    agents
                        .iter()
                        .map(|agent| agent.stored())
                        .collect::<Vec<_>>(),
                ),
            )
            .and_where(Expr::col(StandingRuleRow::State).eq(GivingState::Open.code()))
            .order_by(StandingRuleRow::Id, Order::Asc)
            .limit(limit);
        all(transaction, &select, read_rule).await
    }

    async fn every_open_rule_in(
        transaction: &mut Transaction,
    ) -> Result<Vec<StandingRule>, StoreError> {
        let mut select = rule_select();
        select
            .and_where(Expr::col(StandingRuleRow::State).eq(GivingState::Open.code()))
            .order_by(StandingRuleRow::Id, Order::Asc);
        all(transaction, &select, read_rule).await
    }

    async fn open_rules_in_in(
        transaction: &mut Transaction,
        project: ProjectId,
    ) -> Result<Vec<StandingRule>, StoreError> {
        let mut select = rule_select();
        select
            .and_where(Expr::col(StandingRuleRow::Project).eq(project.stored()))
            .and_where(Expr::col(StandingRuleRow::State).eq(GivingState::Open.code()))
            .order_by(StandingRuleRow::Id, Order::Asc);
        all(transaction, &select, read_rule).await
    }

    async fn known_matches_in(
        transaction: &mut Transaction,
        rule: StandingRuleId,
        issues: Vec<IssueId>,
    ) -> Result<Vec<IssueId>, StoreError> {
        if issues.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = Query::select();
        select
            .column(RuleMatchRow::Issue)
            .from(RuleMatchRow::Table)
            .and_where(Expr::col(RuleMatchRow::Rule).eq(rule.stored()))
            .and_where(
                Expr::col(RuleMatchRow::Issue).is_in(
                    issues
                        .iter()
                        .map(|issue| issue.stored())
                        .collect::<Vec<_>>(),
                ),
            );
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| row.id(RuleMatchRow::Issue))
            .collect()
    }

    async fn proposals_on_in(
        transaction: &mut Transaction,
        issue: IssueId,
        state: Option<ProposalState>,
        agent: Option<AgentId>,
        limit: u64,
    ) -> Result<Vec<Proposal>, StoreError> {
        let mut select = proposal_select();
        select
            .and_where(Expr::col(ProposalRow::Issue).eq(issue.stored()))
            .order_by(ProposalRow::Id, Order::Desc)
            .limit(limit);
        if let Some(state) = state {
            select.and_where(Expr::col(ProposalRow::State).eq(state.code()));
        }
        if let Some(agent) = agent {
            select.and_where(Expr::col(ProposalRow::Agent).eq(agent.stored()));
        }
        all(transaction, &select, read_proposal).await
    }

    async fn matches_of_in(
        transaction: &mut Transaction,
        rule: StandingRuleId,
        cursor: Option<IssueId>,
        limit: u64,
    ) -> Result<Vec<RuleMatch>, StoreError> {
        let mut select = Query::select();
        select
            .columns([RuleMatchRow::Rule, RuleMatchRow::Issue, RuleMatchRow::At])
            .from(RuleMatchRow::Table)
            .and_where(Expr::col(RuleMatchRow::Rule).eq(rule.stored()));
        all(
            transaction,
            &after(select, RuleMatchRow::Issue, cursor, limit),
            |row| {
                Ok(RuleMatch {
                    rule: row.id(RuleMatchRow::Rule)?,
                    issue: row.id(RuleMatchRow::Issue)?,
                    at: row.time(RuleMatchRow::At)?,
                })
            },
        )
        .await
    }

    async fn insert_hand_over(
        transaction: &mut Transaction,
        hand_over: &HandOver,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(HandOverRow::Table)
            .columns([
                HandOverRow::Id,
                HandOverRow::Agent,
                HandOverRow::Person,
                HandOverRow::Note,
                HandOverRow::Powers,
                HandOverRow::Fields,
                HandOverRow::BudgetAttempts,
                HandOverRow::BudgetMinutes,
                HandOverRow::BudgetCost,
                HandOverRow::PlanNeedsApproval,
                HandOverRow::AcceptByItself,
                HandOverRow::State,
                HandOverRow::EndsAt,
                HandOverRow::MadeAt,
                HandOverRow::Version,
            ])
            .values([
                Expr::val(hand_over.id.stored()),
                Expr::val(hand_over.agent.stored()),
                Expr::val(hand_over.person.stored()),
                Expr::val(hand_over.note.as_str()),
                Expr::val(serde_json::to_string(&hand_over.powers)?),
                Expr::val(serde_json::to_string(&hand_over.fields)?),
                Expr::val(hand_over.budget_attempts),
                Expr::val(hand_over.budget_minutes),
                Expr::val(hand_over.budget_cost),
                Expr::val(hand_over.plan_needs_approval),
                Expr::val(hand_over.accept_by_itself),
                Expr::val(hand_over.state.code()),
                Expr::val(to_micros(hand_over.ends_at)),
                Expr::val(to_micros(hand_over.made_at)),
                Expr::val(hand_over.version),
            ])
            .map_err(|_| StoreError::Corrupt("a hand-over"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn end_hand_over(
        transaction: &mut Transaction,
        id: HandOverId,
        version: i64,
    ) -> Result<bool, StoreError> {
        update_at(
            transaction,
            HandOverRow::Table,
            HandOverRow::Id,
            id.stored(),
            HandOverRow::Version,
            version,
            |update| {
                update.value(HandOverRow::State, GivingState::Ended.code());
            },
        )
        .await
    }

    async fn touch_hand_over(
        transaction: &mut Transaction,
        id: HandOverId,
        version: i64,
    ) -> Result<bool, StoreError> {
        update_at(
            transaction,
            HandOverRow::Table,
            HandOverRow::Id,
            id.stored(),
            HandOverRow::Version,
            version,
            |update| {
                update.and_where(Expr::col(HandOverRow::State).eq(GivingState::Open.code()));
            },
        )
        .await
    }

    async fn insert_standing_rule(
        transaction: &mut Transaction,
        rule: &StandingRule,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(StandingRuleRow::Table)
            .columns([
                StandingRuleRow::Id,
                StandingRuleRow::Agent,
                StandingRuleRow::Person,
                StandingRuleRow::Search,
                StandingRuleRow::Project,
                StandingRuleRow::Fields,
                StandingRuleRow::CreatesIn,
                StandingRuleRow::State,
                StandingRuleRow::EndsAt,
                StandingRuleRow::MadeAt,
                StandingRuleRow::Version,
            ])
            .values([
                Expr::val(rule.id.stored()),
                Expr::val(rule.agent.stored()),
                Expr::val(rule.person.stored()),
                Expr::val(rule.search.stored()),
                Expr::val(rule.project.stored()),
                Expr::val(serde_json::to_string(&rule.fields)?),
                Expr::val(rule.creates_in.map(StoredId::stored)),
                Expr::val(rule.state.code()),
                Expr::val(to_micros(rule.ends_at)),
                Expr::val(to_micros(rule.made_at)),
                Expr::val(rule.version),
            ])
            .map_err(|_| StoreError::Corrupt("a standing rule"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn set_rule_end(
        transaction: &mut Transaction,
        id: StandingRuleId,
        ends_at: Timestamp,
        ended: bool,
        version: i64,
    ) -> Result<bool, StoreError> {
        let state = if ended {
            GivingState::Ended
        } else {
            GivingState::Open
        };
        update_at(
            transaction,
            StandingRuleRow::Table,
            StandingRuleRow::Id,
            id.stored(),
            StandingRuleRow::Version,
            version,
            |update| {
                update
                    .value(StandingRuleRow::State, state.code())
                    .value(StandingRuleRow::EndsAt, to_micros(ends_at));
            },
        )
        .await
    }

    async fn insert_proposal(
        transaction: &mut Transaction,
        proposal: &Proposal,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(ProposalRow::Table)
            .columns([
                ProposalRow::Id,
                ProposalRow::Rule,
                ProposalRow::Agent,
                ProposalRow::Issue,
                ProposalRow::Field,
                ProposalRow::Before,
                ProposalRow::After,
                ProposalRow::State,
                ProposalRow::DecidedBy,
                ProposalRow::MadeAt,
            ])
            .values([
                Expr::val(proposal.id.stored()),
                Expr::val(proposal.rule.stored()),
                Expr::val(proposal.agent.stored()),
                Expr::val(proposal.issue.stored()),
                Expr::val(proposal.field.code()),
                Expr::val(serde_json::to_string(&proposal.before)?),
                Expr::val(serde_json::to_string(&proposal.after)?),
                Expr::val(proposal.state.code()),
                Expr::val(proposal.decided_by.map(StoredId::stored)),
                Expr::val(to_micros(proposal.made_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a proposal"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn set_proposal_state(
        transaction: &mut Transaction,
        id: AgentNoteId,
        state: ProposalState,
        by: UserId,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(ProposalRow::Table)
            .value(ProposalRow::State, state.code())
            .value(ProposalRow::DecidedBy, by.stored())
            .and_where(Expr::col(ProposalRow::Id).eq(id.stored()))
            .and_where(Expr::col(ProposalRow::State).eq(ProposalState::Open.code()));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn reopen_proposal(
        transaction: &mut Transaction,
        id: AgentNoteId,
        by: UserId,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(ProposalRow::Table)
            .value(ProposalRow::State, ProposalState::Open.code())
            .value(ProposalRow::DecidedBy, Expr::val(None::<Vec<u8>>))
            .and_where(Expr::col(ProposalRow::Id).eq(id.stored()))
            .and_where(Expr::col(ProposalRow::DecidedBy).eq(by.stored()))
            .and_where(Expr::col(ProposalRow::State).ne(ProposalState::Open.code()));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn insert_match(
        transaction: &mut Transaction,
        rule_match: &RuleMatch,
    ) -> Result<bool, StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(RuleMatchRow::Table)
            .columns([RuleMatchRow::Rule, RuleMatchRow::Issue, RuleMatchRow::At])
            .values([
                Expr::val(rule_match.rule.stored()),
                Expr::val(rule_match.issue.stored()),
                Expr::val(to_micros(rule_match.at)),
            ])
            .map_err(|_| StoreError::Corrupt("a match"))?;
        insert.on_conflict(
            sea_query::OnConflict::columns([RuleMatchRow::Rule, RuleMatchRow::Issue])
                .do_nothing()
                .to_owned(),
        );
        Ok(transaction.execute(&insert).await? > 0)
    }

    async fn end_grants_of_agent(
        transaction: &mut Transaction,
        agent: AgentId,
    ) -> Result<(), StoreError> {
        let mut hand_overs = Query::update();
        hand_overs
            .table(HandOverRow::Table)
            .value(HandOverRow::State, GivingState::Ended.code())
            .value(HandOverRow::Version, Expr::col(HandOverRow::Version).add(1))
            .and_where(Expr::col(HandOverRow::Agent).eq(agent.stored()))
            .and_where(Expr::col(HandOverRow::State).eq(GivingState::Open.code()));
        transaction.execute(&hand_overs).await?;
        let mut rules = Query::update();
        rules
            .table(StandingRuleRow::Table)
            .value(StandingRuleRow::State, GivingState::Ended.code())
            .value(
                StandingRuleRow::Version,
                Expr::col(StandingRuleRow::Version).add(1),
            )
            .and_where(Expr::col(StandingRuleRow::Agent).eq(agent.stored()))
            .and_where(Expr::col(StandingRuleRow::State).eq(GivingState::Open.code()));
        transaction.execute(&rules).await?;
        Ok(())
    }
}
