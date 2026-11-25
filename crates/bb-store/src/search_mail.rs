//! The adapter of the port of e-mailed searches (`bb_app::search_mail::SearchMailStore`, QRY-07).

use bb_app::jobs::QueuedJob;
use bb_app::search_mail::SearchMailStore;
use bb_core::id::{SavedSearchId, SearchMailId, UserId};
use bb_core::search_mail::{MailDays, MailHour, MailPause, SearchMail};
use jiff::Timestamp;
use sea_query::{Cond, Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::jobs::{enqueue, new_job};
use crate::schema::{Computed, SearchMailRow};

const COLUMNS: [SearchMailRow; 12] = [
    SearchMailRow::Id,
    SearchMailRow::Holder,
    SearchMailRow::Search,
    SearchMailRow::Days,
    SearchMailRow::Hour,
    SearchMailRow::OnlyWhenChanged,
    SearchMailRow::Paused,
    SearchMailRow::NextAt,
    SearchMailRow::LastDue,
    SearchMailRow::LastSentAt,
    SearchMailRow::CreatedAt,
    SearchMailRow::UpdatedAt,
];

fn select() -> SelectStatement {
    let mut select = Query::select();
    select.columns(COLUMNS).from(SearchMailRow::Table);
    select
}

fn read_mail(row: &Row) -> Result<SearchMail, StoreError> {
    let days = u8::try_from(row.get::<i64>(SearchMailRow::Days)?)
        .ok()
        .and_then(MailDays::from_bits)
        .ok_or(StoreError::Corrupt("the weekdays of an e-mailed search"))?;
    let hour = MailHour::new(row.get::<i64>(SearchMailRow::Hour)?)
        .ok_or(StoreError::Corrupt("the hour of an e-mailed search"))?;
    let paused = row
        .get::<Option<String>>(SearchMailRow::Paused)?
        .map(|code| {
            MailPause::from_code(&code).ok_or(StoreError::Corrupt("why an e-mailed search waits"))
        })
        .transpose()?;
    Ok(SearchMail {
        id: row.id::<SearchMailId>(SearchMailRow::Id)?,
        holder: row.id::<UserId>(SearchMailRow::Holder)?,
        search: row.id::<SavedSearchId>(SearchMailRow::Search)?,
        days,
        hour,
        only_when_changed: row.get::<bool>(SearchMailRow::OnlyWhenChanged)?,
        paused,
        next_at: row.time(SearchMailRow::NextAt)?,
        last_due: row.optional_time(SearchMailRow::LastDue)?,
        last_sent_at: row.optional_time(SearchMailRow::LastSentAt)?,
        created_at: row.time(SearchMailRow::CreatedAt)?,
        updated_at: row.time(SearchMailRow::UpdatedAt)?,
    })
}

/// Deletes the e-mailed searches of these saved searches: they go with them (ADR-0034, amendment
/// 2026-11-24).
pub(crate) async fn delete_of_searches(
    transaction: &mut Transaction,
    searches: &[SavedSearchId],
) -> Result<(), StoreError> {
    if searches.is_empty() {
        return Ok(());
    }
    let mut delete = Query::delete();
    delete
        .from_table(SearchMailRow::Table)
        .and_where(Expr::col(SearchMailRow::Search).is_in(searches.iter().map(|id| id.stored())));
    transaction.execute(&delete).await?;
    Ok(())
}

impl SearchMailStore for AuthStorage {
    async fn search_mail_in(
        transaction: &mut Transaction,
        id: SearchMailId,
    ) -> Result<Option<SearchMail>, StoreError> {
        let mut select = select();
        select.and_where(Expr::col(SearchMailRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_mail)
            .transpose()
    }

    async fn search_mail_of_in(
        transaction: &mut Transaction,
        holder: UserId,
        search: SavedSearchId,
    ) -> Result<Option<SearchMail>, StoreError> {
        let mut select = select();
        select
            .and_where(Expr::col(SearchMailRow::Holder).eq(holder.stored()))
            .and_where(Expr::col(SearchMailRow::Search).eq(search.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_mail)
            .transpose()
    }

    async fn count_search_mails_of_in(
        transaction: &mut Transaction,
        holder: UserId,
    ) -> Result<u64, StoreError> {
        let mut select = Query::select();
        select
            .expr_as(Expr::col(SearchMailRow::Id).count(), Computed::Total)
            .from(SearchMailRow::Table)
            .and_where(Expr::col(SearchMailRow::Holder).eq(holder.stored()));
        let total: i64 = match transaction.fetch_optional(&select).await? {
            Some(row) => row.get(Computed::Total)?,
            None => 0,
        };
        u64::try_from(total).map_err(|_| StoreError::Corrupt("a count"))
    }

    async fn search_mails_of(&self, holder: UserId) -> Result<Vec<SearchMail>, StoreError> {
        let mut select = select();
        select
            .and_where(Expr::col(SearchMailRow::Holder).eq(holder.stored()))
            .order_by(SearchMailRow::Id, Order::Asc);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_mail)
            .collect()
    }

    async fn search_mails_due(
        &self,
        until: Timestamp,
        limit: u64,
    ) -> Result<Vec<SearchMail>, StoreError> {
        let mut select = select();
        select
            .and_where(Expr::col(SearchMailRow::Paused).is_null())
            .and_where(Expr::col(SearchMailRow::NextAt).lte(to_micros(until)))
            .order_by(SearchMailRow::NextAt, Order::Asc)
            .order_by(SearchMailRow::Id, Order::Asc)
            .limit(limit);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_mail)
            .collect()
    }

    async fn queue_search_mail(
        &self,
        id: SearchMailId,
        due: Timestamp,
        next: Timestamp,
        job: &QueuedJob,
        now: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.database().begin().await?;
        let mut update = Query::update();
        update
            .table(SearchMailRow::Table)
            .value(SearchMailRow::NextAt, to_micros(next))
            .and_where(Expr::col(SearchMailRow::Id).eq(id.stored()))
            .and_where(Expr::col(SearchMailRow::NextAt).eq(to_micros(due)))
            .and_where(Expr::col(SearchMailRow::Paused).is_null());
        if transaction.execute(&update).await? == 0 {
            transaction.commit().await?;
            return Ok(false);
        }
        enqueue(
            &mut transaction,
            job.id,
            &new_job(&job.kind, &job.payload, job.run_at),
            now,
        )
        .await?;
        transaction.commit().await?;
        Ok(true)
    }

    async fn claim_search_mail(
        &self,
        id: SearchMailId,
        due: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(SearchMailRow::Table)
            .value(SearchMailRow::LastDue, to_micros(due))
            .and_where(Expr::col(SearchMailRow::Id).eq(id.stored()))
            .cond_where(
                Cond::any()
                    .add(Expr::col(SearchMailRow::LastDue).is_null())
                    .add(Expr::col(SearchMailRow::LastDue).lt(to_micros(due))),
            );
        Ok(self.database().execute(&update).await? == 1)
    }

    async fn give_back_search_mail(
        &self,
        id: SearchMailId,
        due: Timestamp,
        before: Option<Timestamp>,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(SearchMailRow::Table)
            .value(SearchMailRow::LastDue, Expr::val(before.map(to_micros)))
            .and_where(Expr::col(SearchMailRow::Id).eq(id.stored()))
            .and_where(Expr::col(SearchMailRow::LastDue).eq(to_micros(due)));
        self.database().execute(&update).await?;
        Ok(())
    }

    async fn search_mail_sent(&self, id: SearchMailId, at: Timestamp) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(SearchMailRow::Table)
            .value(SearchMailRow::LastSentAt, to_micros(at))
            .and_where(Expr::col(SearchMailRow::Id).eq(id.stored()));
        self.database().execute(&update).await?;
        Ok(())
    }

    async fn pause_search_mail(
        &self,
        id: SearchMailId,
        pause: MailPause,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(SearchMailRow::Table)
            .value(SearchMailRow::Paused, pause.code())
            .and_where(Expr::col(SearchMailRow::Id).eq(id.stored()));
        self.database().execute(&update).await?;
        Ok(())
    }

    async fn put_search_mail(
        transaction: &mut Transaction,
        mail: &SearchMail,
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(SearchMailRow::Table)
            .and_where(Expr::col(SearchMailRow::Id).eq(mail.id.stored()));
        transaction.execute(&delete).await?;
        let mut insert = Query::insert();
        insert
            .into_table(SearchMailRow::Table)
            .columns(COLUMNS)
            .values([
                Expr::val(mail.id.stored()),
                Expr::val(mail.holder.stored()),
                Expr::val(mail.search.stored()),
                Expr::val(i64::from(mail.days.bits())),
                Expr::val(i64::from(mail.hour.get())),
                Expr::val(mail.only_when_changed),
                Expr::val(mail.paused.map(MailPause::code)),
                Expr::val(to_micros(mail.next_at)),
                Expr::val(mail.last_due.map(to_micros)),
                Expr::val(mail.last_sent_at.map(to_micros)),
                Expr::val(to_micros(mail.created_at)),
                Expr::val(to_micros(mail.updated_at)),
            ])
            .map_err(|_| StoreError::Corrupt("an e-mailed search"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn delete_search_mail(
        transaction: &mut Transaction,
        id: SearchMailId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(SearchMailRow::Table)
            .and_where(Expr::col(SearchMailRow::Id).eq(id.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn pause_search_mails_of(
        transaction: &mut Transaction,
        holder: UserId,
        pause: MailPause,
    ) -> Result<u64, StoreError> {
        let mut update = Query::update();
        update
            .table(SearchMailRow::Table)
            .value(SearchMailRow::Paused, pause.code())
            .and_where(Expr::col(SearchMailRow::Holder).eq(holder.stored()))
            .and_where(Expr::col(SearchMailRow::Paused).is_null());
        transaction.execute(&update).await
    }
}

#[cfg(test)]
#[path = "tests/search_mail.rs"]
mod tests;
