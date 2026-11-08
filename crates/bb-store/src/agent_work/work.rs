//! The work on an issue: attempts, results, questions and reviews.

use bb_app::agent_work::{AgentWorkRecords, WorkDetails};

use super::*;

fn results_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            AttemptResultRow::Id,
            AttemptResultRow::Attempt,
            AttemptResultRow::Kind,
            AttemptResultRow::Label,
            AttemptResultRow::Link,
            AttemptResultRow::Outcome,
        ])
        .from(AttemptResultRow::Table);
    select
}

fn read_result(row: &Row) -> Result<AttemptResult, StoreError> {
    let outcome = row
        .get::<Option<String>>(AttemptResultRow::Outcome)?
        .map(|code| CheckResult::from_code(&code).ok_or(StoreError::Corrupt("a check result")))
        .transpose()?;
    Ok(AttemptResult {
        id: row.id(AttemptResultRow::Id)?,
        attempt: row.id(AttemptResultRow::Attempt)?,
        kind: word::<ResultKind>(row, AttemptResultRow::Kind)?,
        label: row.get(AttemptResultRow::Label)?,
        link: row.get(AttemptResultRow::Link)?,
        outcome,
    })
}

fn reviews_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            ReviewRow::Id,
            ReviewRow::Work,
            ReviewRow::Answer,
            ReviewRow::Note,
            ReviewRow::By,
            ReviewRow::At,
        ])
        .from(ReviewRow::Table);
    select
}

fn read_review(row: &Row) -> Result<Review, StoreError> {
    Ok(Review {
        id: row.id(ReviewRow::Id)?,
        work: row.id(ReviewRow::Work)?,
        answer: word::<ReviewAnswer>(row, ReviewRow::Answer)?,
        note: row.get(ReviewRow::Note)?,
        by: row.optional_id(ReviewRow::By)?,
        at: row.time(ReviewRow::At)?,
    })
}

impl AgentWorkRecords for AuthStorage {
    async fn matches_of_agent_since(
        &self,
        agent: AgentId,
        now: Timestamp,
        since: (Timestamp, Option<(IssueId, StandingRuleId)>),
        limit: u64,
    ) -> Result<Vec<RuleMatch>, StoreError> {
        let (since, after) = since;
        let col = |column: RuleMatchRow| Expr::col((RuleMatchRow::Table, column));
        // After the match the last page ended with: later in time, or at that very moment with a
        // later issue, or with the same issue and a later rule.
        let later = match after {
            Some((issue, rule)) => {
                col(RuleMatchRow::At)
                    .gt(to_micros(since))
                    .or(col(RuleMatchRow::At).eq(to_micros(since)).and(
                        col(RuleMatchRow::Issue)
                            .gt(issue.stored())
                            .or(col(RuleMatchRow::Issue)
                                .eq(issue.stored())
                                .and(col(RuleMatchRow::Rule).gt(rule.stored()))),
                    ))
            }
            None => col(RuleMatchRow::At).gt(to_micros(since)),
        };
        let mut select = Query::select();
        select
            .columns([
                (RuleMatchRow::Table, RuleMatchRow::Rule),
                (RuleMatchRow::Table, RuleMatchRow::Issue),
                (RuleMatchRow::Table, RuleMatchRow::At),
            ])
            .from(RuleMatchRow::Table)
            .inner_join(
                StandingRuleRow::Table,
                Expr::col((StandingRuleRow::Table, StandingRuleRow::Id))
                    .equals((RuleMatchRow::Table, RuleMatchRow::Rule)),
            )
            .cond_where(working_rule(agent, now))
            .and_where(later)
            .order_by((RuleMatchRow::Table, RuleMatchRow::At), Order::Asc)
            .order_by((RuleMatchRow::Table, RuleMatchRow::Issue), Order::Asc)
            .order_by((RuleMatchRow::Table, RuleMatchRow::Rule), Order::Asc)
            .limit(limit);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                Ok(RuleMatch {
                    rule: row.id(RuleMatchRow::Rule)?,
                    issue: row.id(RuleMatchRow::Issue)?,
                    at: row.time(RuleMatchRow::At)?,
                })
            })
            .collect()
    }

    async fn issue_events_after(
        &self,
        after: i64,
        limit: u64,
    ) -> Result<Vec<(i64, bb_core::event::DomainEvent)>, StoreError> {
        self.database().issue_events_after(after, limit).await
    }

    async fn last_event_place(&self) -> Result<i64, StoreError> {
        self.database().last_event_place().await
    }

    async fn issue_work_in(
        transaction: &mut Transaction,
        id: AgentWorkId,
    ) -> Result<Option<IssueWork>, StoreError> {
        let mut select = work_select();
        select.and_where(Expr::col(IssueWorkRow::Id).eq(id.stored()));
        first(transaction, &select, read_work).await
    }

    async fn open_work_of_in(
        transaction: &mut Transaction,
        agent: AgentId,
        issue: IssueId,
    ) -> Result<Option<IssueWork>, StoreError> {
        let mut select = work_select();
        select
            .and_where(Expr::col(IssueWorkRow::Agent).eq(agent.stored()))
            .and_where(Expr::col(IssueWorkRow::Issue).eq(issue.stored()));
        only_open_work(&mut select);
        first(transaction, &select, read_work).await
    }

    async fn attempts_of_in(
        transaction: &mut Transaction,
        work: AgentWorkId,
    ) -> Result<Vec<Attempt>, StoreError> {
        let mut select = attempt_select();
        select
            .and_where(Expr::col(AttemptRow::Work).eq(work.stored()))
            .order_by(AttemptRow::Number, Order::Asc);
        all(transaction, &select, read_attempt).await
    }

    async fn attempt_in(
        transaction: &mut Transaction,
        id: AttemptId,
    ) -> Result<Option<Attempt>, StoreError> {
        let mut select = attempt_select();
        select.and_where(Expr::col(AttemptRow::Id).eq(id.stored()));
        first(transaction, &select, read_attempt).await
    }

    async fn questions_of_in(
        transaction: &mut Transaction,
        work: AgentWorkId,
    ) -> Result<Vec<Question>, StoreError> {
        let mut select = question_select();
        select
            .and_where(Expr::col(QuestionRow::Work).eq(work.stored()))
            .order_by(QuestionRow::Id, Order::Asc);
        all(transaction, &select, read_question).await
    }

    async fn question_in(
        transaction: &mut Transaction,
        id: AgentNoteId,
    ) -> Result<Option<Question>, StoreError> {
        let mut select = question_select();
        select.and_where(Expr::col(QuestionRow::Id).eq(id.stored()));
        first(transaction, &select, read_question).await
    }

    async fn results_of_in(
        transaction: &mut Transaction,
        attempt: AttemptId,
    ) -> Result<Vec<AttemptResult>, StoreError> {
        let mut select = results_select();
        select
            .and_where(Expr::col(AttemptResultRow::Attempt).eq(attempt.stored()))
            .order_by(AttemptResultRow::Id, Order::Asc);
        all(transaction, &select, read_result).await
    }

    async fn works_of_hand_over_in(
        transaction: &mut Transaction,
        hand_over: HandOverId,
    ) -> Result<Vec<IssueWork>, StoreError> {
        let mut select = work_select();
        select
            .and_where(Expr::col(IssueWorkRow::HandOver).eq(hand_over.stored()))
            .order_by(IssueWorkRow::Id, Order::Asc);
        all(transaction, &select, read_work).await
    }

    async fn work_on_issue_in(
        transaction: &mut Transaction,
        issue: IssueId,
        agent: Option<AgentId>,
        limit: u64,
    ) -> Result<Vec<IssueWork>, StoreError> {
        let mut select = work_select();
        select
            .and_where(Expr::col(IssueWorkRow::Issue).eq(issue.stored()))
            // Newest first by the moment of the hand-over: the IDs are random and tell nothing of time.
            .order_by(IssueWorkRow::MadeAt, Order::Desc)
            .order_by(IssueWorkRow::Id, Order::Desc)
            .limit(limit);
        if let Some(agent) = agent {
            select.and_where(Expr::col(IssueWorkRow::Agent).eq(agent.stored()));
        }
        all(transaction, &select, read_work).await
    }

    async fn open_work_on_issues_in(
        transaction: &mut Transaction,
        issues: Vec<IssueId>,
        now: Timestamp,
    ) -> Result<Vec<IssueWork>, StoreError> {
        if issues.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = work_select();
        select
            .and_where(
                Expr::col(IssueWorkRow::Issue).is_in(
                    issues
                        .iter()
                        .map(|issue| issue.stored())
                        .collect::<Vec<_>>(),
                ),
            )
            .order_by(IssueWorkRow::MadeAt, Order::Desc)
            .order_by(IssueWorkRow::Id, Order::Desc);
        only_open_work_at(&mut select, now);
        all(transaction, &select, read_work).await
    }

    async fn details_of_works_in(
        transaction: &mut Transaction,
        works: Vec<IssueWork>,
    ) -> Result<WorkDetails, StoreError> {
        let mut details = WorkDetails::default();
        if works.is_empty() {
            return Ok(details);
        }
        let work_ids: Vec<Vec<u8>> = works.iter().map(|work| work.id.stored()).collect();
        let mut hand_over_ids: Vec<Vec<u8>> =
            works.iter().map(|work| work.hand_over.stored()).collect();
        hand_over_ids.sort();
        hand_over_ids.dedup();
        let mut hand_overs = hand_over_select();
        hand_overs.and_where(Expr::col(HandOverRow::Id).is_in(hand_over_ids));
        details.hand_overs = all(transaction, &hand_overs, read_hand_over).await?;
        let mut questions = question_select();
        questions
            .and_where(Expr::col(QuestionRow::Work).is_in(work_ids.clone()))
            .order_by(QuestionRow::Id, Order::Asc);
        details.questions = all(transaction, &questions, read_question).await?;
        let mut attempts = attempt_select();
        attempts
            .and_where(Expr::col(AttemptRow::Work).is_in(work_ids.clone()))
            .order_by(AttemptRow::Number, Order::Asc);
        details.attempts = all(transaction, &attempts, read_attempt).await?;
        let attempt_ids: Vec<Vec<u8>> = details
            .attempts
            .iter()
            .map(|attempt| attempt.id.stored())
            .collect();
        if !attempt_ids.is_empty() {
            let mut results = results_select();
            results
                .and_where(Expr::col(AttemptResultRow::Attempt).is_in(attempt_ids))
                .order_by(AttemptResultRow::Id, Order::Asc);
            details.results = all(transaction, &results, read_result).await?;
        }
        let mut reviews = reviews_select();
        reviews
            .and_where(Expr::col(ReviewRow::Work).is_in(work_ids))
            .order_by(ReviewRow::Id, Order::Asc);
        details.reviews = all(transaction, &reviews, read_review).await?;
        Ok(details)
    }

    async fn another_in_attempt_in(
        transaction: &mut Transaction,
        issue: IssueId,
        except: AgentWorkId,
    ) -> Result<bool, StoreError> {
        let mut select = Query::select();
        select
            .expr(Expr::val(1))
            .from(IssueWorkRow::Table)
            .and_where(Expr::col(IssueWorkRow::Issue).eq(issue.stored()))
            .and_where(Expr::col(IssueWorkRow::Id).ne(except.stored()))
            .and_where(
                Expr::col(IssueWorkRow::State)
                    .is_in(AgentWorkState::IN_ATTEMPT.map(AgentWorkState::code)),
            )
            .limit(1);
        only_open_work(&mut select);
        Ok(transaction.fetch_optional(&select).await?.is_some())
    }

    async fn working_issues_of_in(
        transaction: &mut Transaction,
        agent: AgentId,
    ) -> Result<u64, StoreError> {
        let mut select = Query::select();
        select
            .expr_as(Expr::col(IssueWorkRow::Id).count(), Computed::Total)
            .from(IssueWorkRow::Table)
            .and_where(Expr::col(IssueWorkRow::Agent).eq(agent.stored()))
            .and_where(
                Expr::col(IssueWorkRow::State)
                    .is_in(AgentWorkState::IN_ATTEMPT.map(AgentWorkState::code)),
            );
        only_open_work(&mut select);
        let count: i64 = match transaction.fetch_optional(&select).await? {
            Some(row) => row.get(Computed::Total)?,
            None => 0,
        };
        Ok(u64::try_from(count).unwrap_or(0))
    }

    async fn agent_has_open_giving_in(
        transaction: &mut Transaction,
        agent: AgentId,
    ) -> Result<bool, StoreError> {
        let mut work = Query::select();
        work.expr(Expr::val(1))
            .from(IssueWorkRow::Table)
            .and_where(Expr::col(IssueWorkRow::Agent).eq(agent.stored()))
            .limit(1);
        only_open_work(&mut work);
        let mut hand_overs = Query::select();
        hand_overs
            .expr(Expr::val(1))
            .from(HandOverRow::Table)
            .and_where(Expr::col(HandOverRow::Agent).eq(agent.stored()))
            .and_where(Expr::col(HandOverRow::State).eq(GivingState::Open.code()))
            .limit(1);
        let mut rules = Query::select();
        rules
            .expr(Expr::val(1))
            .from(StandingRuleRow::Table)
            .and_where(Expr::col(StandingRuleRow::Agent).eq(agent.stored()))
            .and_where(Expr::col(StandingRuleRow::State).eq(GivingState::Open.code()))
            .limit(1);
        for select in [work, hand_overs, rules] {
            if transaction.fetch_optional(&select).await?.is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn work_of_agent_in(
        transaction: &mut Transaction,
        agent: AgentId,
        cursor: Option<AgentWorkId>,
        limit: u64,
    ) -> Result<Vec<IssueWork>, StoreError> {
        let mut select = work_select();
        select.and_where(Expr::col(IssueWorkRow::Agent).eq(agent.stored()));
        all(
            transaction,
            &after(select, IssueWorkRow::Id, cursor, limit),
            read_work,
        )
        .await
    }

    async fn reviews_of_in(
        transaction: &mut Transaction,
        work: AgentWorkId,
    ) -> Result<Vec<Review>, StoreError> {
        let mut select = reviews_select();
        select
            .and_where(Expr::col(ReviewRow::Work).eq(work.stored()))
            .order_by(ReviewRow::Id, Order::Asc);
        all(transaction, &select, read_review).await
    }

    async fn people_of_work_in(
        transaction: &mut Transaction,
        work: AgentWorkId,
    ) -> Result<Vec<UserId>, StoreError> {
        let mut select = Query::select();
        select
            .column((HandOverRow::Table, HandOverRow::Person))
            .column((IssueWorkRow::Table, IssueWorkRow::Person))
            .from(IssueWorkRow::Table)
            .inner_join(
                HandOverRow::Table,
                Expr::col((HandOverRow::Table, HandOverRow::Id))
                    .equals((IssueWorkRow::Table, IssueWorkRow::HandOver)),
            )
            .and_where(Expr::col((IssueWorkRow::Table, IssueWorkRow::Id)).eq(work.stored()));
        let mut people = Vec::new();
        for row in transaction.fetch_all(&select).await? {
            // The person responsible for the agent first, then the one who handed over.
            for person in [
                row.id::<UserId>(IssueWorkRow::Person)?,
                row.id::<UserId>(HandOverRow::Person)?,
            ] {
                if !people.contains(&person) {
                    people.push(person);
                }
            }
        }
        Ok(people)
    }

    async fn lock_working_of(
        transaction: &mut Transaction,
        agent: AgentId,
    ) -> Result<(), StoreError> {
        transaction
            .advisory_lock(LockClass::AgentTaking, lock_key(agent.to_bytes()))
            .await
    }

    async fn lock_taking_of(
        transaction: &mut Transaction,
        issue: IssueId,
    ) -> Result<(), StoreError> {
        transaction
            .advisory_lock(LockClass::IssueTaking, lock_key(issue.to_bytes()))
            .await
    }

    async fn insert_work(
        transaction: &mut Transaction,
        work: &IssueWork,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(IssueWorkRow::Table)
            .columns([
                IssueWorkRow::Id,
                IssueWorkRow::Issue,
                IssueWorkRow::HandOver,
                IssueWorkRow::Agent,
                IssueWorkRow::Person,
                IssueWorkRow::State,
                IssueWorkRow::GaveUp,
                IssueWorkRow::StoppedFor,
                IssueWorkRow::Closed,
                IssueWorkRow::AcceptedByItself,
                IssueWorkRow::StatusPending,
                IssueWorkRow::MadeAt,
                IssueWorkRow::Version,
            ])
            .values([
                Expr::val(work.id.stored()),
                Expr::val(work.issue.stored()),
                Expr::val(work.hand_over.stored()),
                Expr::val(work.agent.stored()),
                Expr::val(work.person.stored()),
                Expr::val(work.state.code()),
                Expr::val(work.gave_up.clone()),
                Expr::val(work.stopped_for.map(StopCause::code)),
                Expr::val(work.closed),
                Expr::val(work.accepted_by_itself),
                Expr::val(work.status_pending),
                Expr::val(to_micros(work.made_at)),
                Expr::val(work.version),
            ])
            .map_err(|_| StoreError::Corrupt("agent work"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn set_work_state(
        transaction: &mut Transaction,
        id: AgentWorkId,
        state: AgentWorkState,
        gave_up: Option<&str>,
        accepted_by_itself: bool,
        version: i64,
    ) -> Result<bool, StoreError> {
        let gave_up = gave_up.map(str::to_owned);
        update_at(
            transaction,
            IssueWorkRow::Table,
            IssueWorkRow::Id,
            id.stored(),
            IssueWorkRow::Version,
            version,
            |update| {
                update
                    .value(IssueWorkRow::State, state.code())
                    .value(IssueWorkRow::GaveUp, gave_up)
                    .value(IssueWorkRow::AcceptedByItself, accepted_by_itself);
            },
        )
        .await
    }

    async fn set_stop_cause(
        transaction: &mut Transaction,
        id: AgentWorkId,
        cause: StopCause,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(IssueWorkRow::Table)
            .value(IssueWorkRow::StoppedFor, cause.code())
            .and_where(Expr::col(IssueWorkRow::Id).eq(id.stored()));
        transaction.execute(&update).await?;
        Ok(())
    }

    async fn close_work(
        transaction: &mut Transaction,
        id: AgentWorkId,
        accepted_by_itself: bool,
        status_pending: bool,
        version: i64,
    ) -> Result<bool, StoreError> {
        update_at(
            transaction,
            IssueWorkRow::Table,
            IssueWorkRow::Id,
            id.stored(),
            IssueWorkRow::Version,
            version,
            |update| {
                update
                    .value(IssueWorkRow::Closed, true)
                    .value(IssueWorkRow::AcceptedByItself, accepted_by_itself)
                    .value(IssueWorkRow::StatusPending, status_pending);
            },
        )
        .await
    }

    async fn mark_work_seen(
        transaction: &mut Transaction,
        id: AgentWorkId,
        version: i64,
    ) -> Result<bool, StoreError> {
        update_at(
            transaction,
            IssueWorkRow::Table,
            IssueWorkRow::Id,
            id.stored(),
            IssueWorkRow::Version,
            version,
            |update| {
                update
                    .value(IssueWorkRow::AcceptedByItself, false)
                    .value(IssueWorkRow::StatusPending, false);
            },
        )
        .await
    }

    async fn insert_attempt(
        transaction: &mut Transaction,
        attempt: &Attempt,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(AttemptRow::Table)
            .columns([
                AttemptRow::Id,
                AttemptRow::Work,
                AttemptRow::Number,
                AttemptRow::Plan,
                AttemptRow::PlanApproved,
                AttemptRow::ReportedMinutes,
                AttemptRow::ReportedCost,
                AttemptRow::StartedAt,
                AttemptRow::EndedAt,
                AttemptRow::Run,
            ])
            .values([
                Expr::val(attempt.id.stored()),
                Expr::val(attempt.work.stored()),
                Expr::val(attempt.number),
                Expr::val(attempt.plan.clone()),
                Expr::val(attempt.plan_approved),
                Expr::val(attempt.reported_minutes),
                Expr::val(attempt.reported_cost),
                Expr::val(to_micros(attempt.started_at)),
                Expr::val(attempt.ended_at.map(to_micros)),
                Expr::val(attempt.run.map(StoredId::stored)),
            ])
            .map_err(|_| StoreError::Corrupt("an attempt"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn update_attempt(
        transaction: &mut Transaction,
        attempt: &Attempt,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(AttemptRow::Table)
            .value(AttemptRow::Plan, attempt.plan.clone())
            .value(AttemptRow::PlanApproved, attempt.plan_approved)
            .value(AttemptRow::ReportedMinutes, attempt.reported_minutes)
            .value(AttemptRow::ReportedCost, attempt.reported_cost)
            .value(AttemptRow::EndedAt, attempt.ended_at.map(to_micros))
            .and_where(Expr::col(AttemptRow::Id).eq(attempt.id.stored()));
        transaction.execute(&update).await?;
        Ok(())
    }

    async fn insert_result(
        transaction: &mut Transaction,
        result: &AttemptResult,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(AttemptResultRow::Table)
            .columns([
                AttemptResultRow::Id,
                AttemptResultRow::Attempt,
                AttemptResultRow::Kind,
                AttemptResultRow::Label,
                AttemptResultRow::Link,
                AttemptResultRow::Outcome,
            ])
            .values([
                Expr::val(result.id.stored()),
                Expr::val(result.attempt.stored()),
                Expr::val(result.kind.code()),
                Expr::val(result.label.as_str()),
                Expr::val(result.link.clone()),
                Expr::val(result.outcome.map(CheckResult::code)),
            ])
            .map_err(|_| StoreError::Corrupt("a result"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn insert_question(
        transaction: &mut Transaction,
        question: &Question,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(QuestionRow::Table)
            .columns([
                QuestionRow::Id,
                QuestionRow::Work,
                QuestionRow::Text,
                QuestionRow::Answer,
                QuestionRow::AnsweredBy,
                QuestionRow::AnsweredAt,
                QuestionRow::AskedAt,
            ])
            .values([
                Expr::val(question.id.stored()),
                Expr::val(question.work.stored()),
                Expr::val(question.text.as_str()),
                Expr::val(question.answer.clone()),
                Expr::val(question.answered_by.map(StoredId::stored)),
                Expr::val(question.answered_at.map(to_micros)),
                Expr::val(to_micros(question.asked_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a question"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn set_answer(
        transaction: &mut Transaction,
        id: AgentNoteId,
        answer: &str,
        by: UserId,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(QuestionRow::Table)
            .value(QuestionRow::Answer, answer)
            .value(QuestionRow::AnsweredBy, by.stored())
            .value(QuestionRow::AnsweredAt, to_micros(at))
            .and_where(Expr::col(QuestionRow::Id).eq(id.stored()));
        transaction.execute(&update).await?;
        Ok(())
    }

    async fn insert_review(
        transaction: &mut Transaction,
        review: &Review,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(ReviewRow::Table)
            .columns([
                ReviewRow::Id,
                ReviewRow::Work,
                ReviewRow::Answer,
                ReviewRow::Note,
                ReviewRow::By,
                ReviewRow::At,
            ])
            .values([
                Expr::val(review.id.stored()),
                Expr::val(review.work.stored()),
                Expr::val(review.answer.code()),
                Expr::val(review.note.clone()),
                Expr::val(review.by.map(StoredId::stored)),
                Expr::val(to_micros(review.at)),
            ])
            .map_err(|_| StoreError::Corrupt("a review"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn stop_work_of_agent(
        transaction: &mut Transaction,
        agent: AgentId,
        cause: StopCause,
    ) -> Result<Vec<AgentWorkId>, StoreError> {
        let mut select = Query::select();
        select
            .column(IssueWorkRow::Id)
            .from(IssueWorkRow::Table)
            .and_where(Expr::col(IssueWorkRow::Agent).eq(agent.stored()))
            .order_by(IssueWorkRow::Id, Order::Asc);
        only_open_work(&mut select);
        let mut stopped = Vec::new();
        for row in transaction.fetch_all(&select).await? {
            stopped.push(row.id::<AgentWorkId>(IssueWorkRow::Id)?);
        }
        if !stopped.is_empty() {
            let mut update = Query::update();
            update
                .table(IssueWorkRow::Table)
                .value(IssueWorkRow::State, AgentWorkState::Stopped.code())
                .value(IssueWorkRow::StoppedFor, cause.code())
                .value(
                    IssueWorkRow::Version,
                    Expr::col(IssueWorkRow::Version).add(1),
                )
                .and_where(
                    Expr::col(IssueWorkRow::Id)
                        .is_in(stopped.iter().map(|id| id.stored()).collect::<Vec<_>>()),
                );
            transaction.execute(&update).await?;
        }
        Ok(stopped)
    }
}
