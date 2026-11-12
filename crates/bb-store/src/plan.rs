//! The adapter of the port of the plan (`bb_app::plan::TimelineStore`): the time factors of issues and
//! the accepted plan warnings (PLAN-13, PLAN-14, ADR-0030 and its amendment M8).

use bb_app::plan::TimelineStore;
use bb_core::id::{IssueId, UserId};
use bb_core::plan::TimeFactor;
use bb_core::plan::record::{AcceptedWarning, IssueTimeFactor};
use bb_core::plan::warning::WarningKind;
use sea_query::{Expr, ExprTrait as _, OnConflict, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction};
use crate::error::StoreError;
use crate::issues::{read_day, stored_day};
use crate::rows::to_micros;
use crate::schema::{AcceptedWarningRow, TimeFactorRow};

fn factor_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            TimeFactorRow::Issue,
            TimeFactorRow::Percent,
            TimeFactorRow::Person,
            TimeFactorRow::SetBy,
        ])
        .from(TimeFactorRow::Table);
    select
}

fn read_factor(row: &Row) -> Result<IssueTimeFactor, StoreError> {
    let percent: i64 = row.get(TimeFactorRow::Percent)?;
    Ok(IssueTimeFactor {
        issue: row.id::<IssueId>(TimeFactorRow::Issue)?,
        factor: u16::try_from(percent)
            .ok()
            .and_then(TimeFactor::new)
            .ok_or(StoreError::Corrupt("a time factor"))?,
        person: row.id::<UserId>(TimeFactorRow::Person)?,
        set_by: row.id::<UserId>(TimeFactorRow::SetBy)?,
    })
}

fn read_accepted(row: &Row) -> Result<AcceptedWarning, StoreError> {
    Ok(AcceptedWarning {
        issue: row.id::<IssueId>(AcceptedWarningRow::Issue)?,
        kind: WarningKind::from_code(&row.get::<String>(AcceptedWarningRow::Kind)?)
            .ok_or(StoreError::Corrupt("a kind of plan warning"))?,
        start_date: read_day(row, AcceptedWarningRow::StartDate)?,
        due_date: read_day(row, AcceptedWarningRow::DueDate)?,
        accepted_by: row.id::<UserId>(AcceptedWarningRow::AcceptedBy)?,
        accepted_at: row.time(AcceptedWarningRow::AcceptedAt)?,
    })
}

fn stored_issues(issues: &[IssueId]) -> Vec<Vec<u8>> {
    issues.iter().map(|issue| issue.stored()).collect()
}

impl TimelineStore for AuthStorage {
    async fn time_factor_in(
        transaction: &mut Transaction,
        issue: IssueId,
    ) -> Result<Option<IssueTimeFactor>, StoreError> {
        let mut select = factor_select();
        select.and_where(Expr::col(TimeFactorRow::Issue).eq(issue.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_factor)
            .transpose()
    }

    async fn time_factors_of(
        &self,
        issues: Vec<IssueId>,
    ) -> Result<Vec<IssueTimeFactor>, StoreError> {
        if issues.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = factor_select();
        select.and_where(Expr::col(TimeFactorRow::Issue).is_in(stored_issues(&issues)));
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_factor)
            .collect()
    }

    async fn accepted_warnings_of(
        &self,
        issues: Vec<IssueId>,
    ) -> Result<Vec<AcceptedWarning>, StoreError> {
        if issues.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = Query::select();
        select
            .columns([
                AcceptedWarningRow::Issue,
                AcceptedWarningRow::Kind,
                AcceptedWarningRow::StartDate,
                AcceptedWarningRow::DueDate,
                AcceptedWarningRow::AcceptedBy,
                AcceptedWarningRow::AcceptedAt,
            ])
            .from(AcceptedWarningRow::Table)
            .and_where(Expr::col(AcceptedWarningRow::Issue).is_in(stored_issues(&issues)));
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_accepted)
            .collect()
    }

    async fn put_time_factor(
        transaction: &mut Transaction,
        factor: &IssueTimeFactor,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(TimeFactorRow::Table)
            .columns([
                TimeFactorRow::Issue,
                TimeFactorRow::Percent,
                TimeFactorRow::Person,
                TimeFactorRow::SetBy,
            ])
            .values([
                Expr::val(factor.issue.stored()),
                Expr::val(i64::from(factor.factor.percent())),
                Expr::val(factor.person.stored()),
                Expr::val(factor.set_by.stored()),
            ])
            .map_err(|_| StoreError::Corrupt("a time factor"))?
            .on_conflict(
                OnConflict::column(TimeFactorRow::Issue)
                    .update_columns([
                        TimeFactorRow::Percent,
                        TimeFactorRow::Person,
                        TimeFactorRow::SetBy,
                    ])
                    .to_owned(),
            );
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn delete_time_factor(
        transaction: &mut Transaction,
        issue: IssueId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(TimeFactorRow::Table)
            .and_where(Expr::col(TimeFactorRow::Issue).eq(issue.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn put_accepted_warning(
        transaction: &mut Transaction,
        warning: &AcceptedWarning,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(AcceptedWarningRow::Table)
            .columns([
                AcceptedWarningRow::Issue,
                AcceptedWarningRow::Kind,
                AcceptedWarningRow::StartDate,
                AcceptedWarningRow::DueDate,
                AcceptedWarningRow::AcceptedBy,
                AcceptedWarningRow::AcceptedAt,
            ])
            .values([
                Expr::val(warning.issue.stored()),
                Expr::val(warning.kind.code()),
                Expr::val(stored_day(warning.start_date)),
                Expr::val(stored_day(warning.due_date)),
                Expr::val(warning.accepted_by.stored()),
                Expr::val(to_micros(warning.accepted_at)),
            ])
            .map_err(|_| StoreError::Corrupt("an accepted plan warning"))?
            .on_conflict(
                OnConflict::columns([AcceptedWarningRow::Issue, AcceptedWarningRow::Kind])
                    .update_columns([
                        AcceptedWarningRow::StartDate,
                        AcceptedWarningRow::DueDate,
                        AcceptedWarningRow::AcceptedBy,
                        AcceptedWarningRow::AcceptedAt,
                    ])
                    .to_owned(),
            );
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn delete_accepted_warning(
        transaction: &mut Transaction,
        issue: IssueId,
        kind: WarningKind,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(AcceptedWarningRow::Table)
            .and_where(Expr::col(AcceptedWarningRow::Issue).eq(issue.stored()))
            .and_where(Expr::col(AcceptedWarningRow::Kind).eq(kind.code()));
        Ok(transaction.execute(&delete).await? > 0)
    }
}
