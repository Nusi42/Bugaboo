//! The work on an issue: attempts, results, questions and reviews.

use bb_app::agent_work::AgentWorkRecords;

use super::*;

impl AgentWorkRecords for AuthStorage {
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
            .from(AttemptResultRow::Table)
            .and_where(Expr::col(AttemptResultRow::Attempt).eq(attempt.stored()))
            .order_by(AttemptResultRow::Id, Order::Asc);
        all(transaction, &select, |row| {
            let outcome = row
                .get::<Option<String>>(AttemptResultRow::Outcome)?
                .map(|code| {
                    CheckResult::from_code(&code).ok_or(StoreError::Corrupt("a check result"))
                })
                .transpose()?;
            Ok(AttemptResult {
                id: row.id(AttemptResultRow::Id)?,
                attempt: row.id(AttemptResultRow::Attempt)?,
                kind: word::<ResultKind>(row, AttemptResultRow::Kind)?,
                label: row.get(AttemptResultRow::Label)?,
                link: row.get(AttemptResultRow::Link)?,
                outcome,
            })
        })
        .await
    }

    async fn work_on_issue_in(
        transaction: &mut Transaction,
        issue: IssueId,
    ) -> Result<Vec<IssueWork>, StoreError> {
        let mut select = work_select();
        select
            .and_where(Expr::col(IssueWorkRow::Issue).eq(issue.stored()))
            .order_by(IssueWorkRow::Id, Order::Desc);
        all(transaction, &select, read_work).await
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
            .from(ReviewRow::Table)
            .and_where(Expr::col(ReviewRow::Work).eq(work.stored()))
            .order_by(ReviewRow::Id, Order::Asc);
        all(transaction, &select, |row| {
            Ok(Review {
                id: row.id(ReviewRow::Id)?,
                work: row.id(ReviewRow::Work)?,
                answer: word::<ReviewAnswer>(row, ReviewRow::Answer)?,
                note: row.get(ReviewRow::Note)?,
                by: row.optional_id(ReviewRow::By)?,
                at: row.time(ReviewRow::At)?,
            })
        })
        .await
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
                IssueWorkRow::AcceptedByItself,
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
                Expr::val(work.accepted_by_itself),
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

    async fn mark_work_seen(
        transaction: &mut Transaction,
        id: AgentWorkId,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(IssueWorkRow::Table)
            .value(IssueWorkRow::AcceptedByItself, false)
            .and_where(Expr::col(IssueWorkRow::Id).eq(id.stored()));
        transaction.execute(&update).await?;
        Ok(())
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
                Expr::val(result.outcome.map(|outcome| outcome.code())),
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
                QuestionRow::AskedAt,
            ])
            .values([
                Expr::val(question.id.stored()),
                Expr::val(question.work.stored()),
                Expr::val(question.text.as_str()),
                Expr::val(question.answer.clone()),
                Expr::val(question.answered_by.map(StoredId::stored)),
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
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(QuestionRow::Table)
            .value(QuestionRow::Answer, answer)
            .value(QuestionRow::AnsweredBy, by.stored())
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
