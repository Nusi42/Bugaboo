//! The adapter of the port of work times (`bb_app::work_times::WorkTimesStore`): the usual work
//! days, the shares and the away times of the people (PLAN-11, PLAN-15, ADR-0030), and the deletion
//! of away times one year after their last day and of everything of a person who leaves, with all
//! that tells about it.

mod old;

use std::collections::BTreeMap;

use bb_app::work_times::WorkTimesStore;
use bb_core::id::{AwayTimeId, ProjectId, UserId};
use bb_core::work_times::record::AwayTime;
use bb_core::work_times::{Share, UsualDay, UsualDays};
use jiff::civil::{Date, Weekday};
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction};
use crate::error::StoreError;
use crate::schema::{AwayTimeRow, ProjectShareRow, UsualWorkDayRow};

/// The weekdays in the order they are stored: 1 for Monday to 7 for Sunday.
const WEEKDAYS: [Weekday; 7] = [
    Weekday::Monday,
    Weekday::Tuesday,
    Weekday::Wednesday,
    Weekday::Thursday,
    Weekday::Friday,
    Weekday::Saturday,
    Weekday::Sunday,
];

/// The number a weekday is stored as.
fn weekday_number(weekday: Weekday) -> i64 {
    i64::from(weekday.to_monday_one_offset())
}

fn usual_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            UsualWorkDayRow::Person,
            UsualWorkDayRow::Weekday,
            UsualWorkDayRow::Works,
            UsualWorkDayRow::Minutes,
        ])
        .from(UsualWorkDayRow::Table);
    select
}

/// The usual work days of the people whose rows these are, a person's seven rows together. A weekday
/// without a row has the starting values, which never happens for rows this adapter wrote.
fn usual_days_from(rows: &[Row]) -> Result<Vec<(UserId, UsualDays)>, StoreError> {
    let mut by_person: BTreeMap<[u8; 16], (UserId, [UsualDay; 7])> = BTreeMap::new();
    for row in rows {
        let person: UserId = row.id(UsualWorkDayRow::Person)?;
        let weekday: i64 = row.get(UsualWorkDayRow::Weekday)?;
        let at = usize::try_from(weekday - 1)
            .ok()
            .filter(|at| *at < WEEKDAYS.len())
            .ok_or(StoreError::Corrupt("a weekday"))?;
        let minutes: i64 = row.get(UsualWorkDayRow::Minutes)?;
        let entry = by_person
            .entry(person.to_bytes())
            .or_insert_with(|| (person, *UsualDays::start().days()));
        entry.1[at] = UsualDay {
            works: row.get(UsualWorkDayRow::Works)?,
            minutes: u16::try_from(minutes).map_err(|_| StoreError::Corrupt("work hours"))?,
        };
    }
    by_person
        .into_values()
        .map(|(person, days)| {
            UsualDays::new(days)
                .map(|days| (person, days))
                .ok_or(StoreError::Corrupt("work hours"))
        })
        .collect()
}

fn read_share(row: &Row) -> Result<Share, StoreError> {
    let percent: i64 = row.get(ProjectShareRow::Percent)?;
    u8::try_from(percent)
        .ok()
        .and_then(Share::new)
        .ok_or(StoreError::Corrupt("a share"))
}

fn share_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            ProjectShareRow::Person,
            ProjectShareRow::Project,
            ProjectShareRow::Percent,
        ])
        .from(ProjectShareRow::Table);
    select
}

const AWAY_COLUMNS: [AwayTimeRow; 9] = [
    AwayTimeRow::Id,
    AwayTimeRow::Person,
    AwayTimeRow::First,
    AwayTimeRow::Last,
    AwayTimeRow::FirstAfternoonOnly,
    AwayTimeRow::LastMorningOnly,
    AwayTimeRow::Note,
    AwayTimeRow::EnteredBy,
    AwayTimeRow::Version,
];

fn away_select() -> SelectStatement {
    let mut select = Query::select();
    select.columns(AWAY_COLUMNS).from(AwayTimeRow::Table);
    select
}

fn date_of(row: &Row, column: AwayTimeRow) -> Result<Date, StoreError> {
    row.get::<String>(column)?
        .parse()
        .map_err(|_| StoreError::Corrupt("a date"))
}

fn read_away(row: &Row) -> Result<AwayTime, StoreError> {
    Ok(AwayTime {
        id: row.id(AwayTimeRow::Id)?,
        person: row.id(AwayTimeRow::Person)?,
        first: date_of(row, AwayTimeRow::First)?,
        last: date_of(row, AwayTimeRow::Last)?,
        first_afternoon_only: row.get(AwayTimeRow::FirstAfternoonOnly)?,
        last_morning_only: row.get(AwayTimeRow::LastMorningOnly)?,
        note: row.get(AwayTimeRow::Note)?,
        entered_by: row.id(AwayTimeRow::EnteredBy)?,
        version: row.get(AwayTimeRow::Version)?,
    })
}

fn stored_ids(people: Vec<UserId>) -> Vec<Vec<u8>> {
    people.into_iter().map(StoredId::stored).collect()
}

/// Writes the seven rows of a person's usual work days, in place of the ones they had.
async fn write_usual(
    transaction: &mut Transaction,
    person: UserId,
    days: &UsualDays,
) -> Result<(), StoreError> {
    let mut delete = Query::delete();
    delete
        .from_table(UsualWorkDayRow::Table)
        .and_where(Expr::col(UsualWorkDayRow::Person).eq(person.stored()));
    transaction.execute(&delete).await?;
    let mut insert = Query::insert();
    insert.into_table(UsualWorkDayRow::Table).columns([
        UsualWorkDayRow::Person,
        UsualWorkDayRow::Weekday,
        UsualWorkDayRow::Works,
        UsualWorkDayRow::Minutes,
    ]);
    for (weekday, day) in WEEKDAYS.iter().zip(days.days()) {
        insert
            .values([
                Expr::val(person.stored()),
                Expr::val(weekday_number(*weekday)),
                Expr::val(day.works),
                Expr::val(i64::from(day.minutes)),
            ])
            .map_err(|_| StoreError::Corrupt("a usual work day"))?;
    }
    transaction.execute(&insert).await.map(|_| ())
}

impl WorkTimesStore for AuthStorage {
    async fn usual_days_in(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Option<UsualDays>, StoreError> {
        let mut select = usual_select();
        select.and_where(Expr::col(UsualWorkDayRow::Person).eq(person.stored()));
        let rows = transaction.fetch_all(&select).await?;
        Ok(usual_days_from(&rows)?
            .into_iter()
            .next()
            .map(|(_, days)| days))
    }

    async fn share_in(
        transaction: &mut Transaction,
        person: UserId,
        project: ProjectId,
    ) -> Result<Option<Share>, StoreError> {
        let mut select = share_select();
        select
            .and_where(Expr::col(ProjectShareRow::Person).eq(person.stored()))
            .and_where(Expr::col(ProjectShareRow::Project).eq(project.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .map(|row| read_share(&row))
            .transpose()
    }

    async fn away_time_in(
        transaction: &mut Transaction,
        id: AwayTimeId,
    ) -> Result<Option<AwayTime>, StoreError> {
        let mut select = away_select();
        select.and_where(Expr::col(AwayTimeRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .map(|row| read_away(&row))
            .transpose()
    }

    async fn usual_days_of(
        &self,
        people: Vec<UserId>,
    ) -> Result<Vec<(UserId, UsualDays)>, StoreError> {
        if people.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = usual_select();
        select.and_where(Expr::col(UsualWorkDayRow::Person).is_in(stored_ids(people)));
        let rows = self.database().fetch_all(&select).await?;
        usual_days_from(&rows)
    }

    async fn shares_of(
        &self,
        people: Vec<UserId>,
    ) -> Result<Vec<(UserId, ProjectId, Share)>, StoreError> {
        if people.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = share_select();
        select
            .and_where(Expr::col(ProjectShareRow::Person).is_in(stored_ids(people)))
            .order_by(ProjectShareRow::Person, Order::Asc)
            .order_by(ProjectShareRow::Project, Order::Asc);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                Ok((
                    row.id(ProjectShareRow::Person)?,
                    row.id(ProjectShareRow::Project)?,
                    read_share(row)?,
                ))
            })
            .collect()
    }

    async fn shares_in(
        &self,
        project: ProjectId,
        people: Vec<UserId>,
    ) -> Result<Vec<(UserId, Share)>, StoreError> {
        if people.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = share_select();
        select
            .and_where(Expr::col(ProjectShareRow::Project).eq(project.stored()))
            .and_where(Expr::col(ProjectShareRow::Person).is_in(stored_ids(people)));
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| Ok((row.id(ProjectShareRow::Person)?, read_share(row)?)))
            .collect()
    }

    async fn away_times_of(
        &self,
        people: Vec<UserId>,
        first: Date,
        last: Date,
    ) -> Result<Vec<AwayTime>, StoreError> {
        if people.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = away_select();
        select
            .and_where(Expr::col(AwayTimeRow::Person).is_in(stored_ids(people)))
            .and_where(Expr::col(AwayTimeRow::First).lte(last.to_string()))
            .and_where(Expr::col(AwayTimeRow::Last).gte(first.to_string()))
            .order_by(AwayTimeRow::First, Order::Asc)
            .order_by(AwayTimeRow::Id, Order::Asc);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_away)
            .collect()
    }

    async fn put_usual_days(
        transaction: &mut Transaction,
        person: UserId,
        days: &UsualDays,
    ) -> Result<(), StoreError> {
        write_usual(transaction, person, days).await
    }

    async fn put_share(
        transaction: &mut Transaction,
        person: UserId,
        project: ProjectId,
        share: Share,
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(ProjectShareRow::Table)
            .and_where(Expr::col(ProjectShareRow::Person).eq(person.stored()))
            .and_where(Expr::col(ProjectShareRow::Project).eq(project.stored()));
        transaction.execute(&delete).await?;
        let mut insert = Query::insert();
        insert
            .into_table(ProjectShareRow::Table)
            .columns([
                ProjectShareRow::Person,
                ProjectShareRow::Project,
                ProjectShareRow::Percent,
            ])
            .values([
                Expr::val(person.stored()),
                Expr::val(project.stored()),
                Expr::val(i64::from(share.percent())),
            ])
            .map_err(|_| StoreError::Corrupt("a share"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn insert_away_time(
        transaction: &mut Transaction,
        time: &AwayTime,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(AwayTimeRow::Table)
            .columns(AWAY_COLUMNS)
            .values([
                Expr::val(time.id.stored()),
                Expr::val(time.person.stored()),
                Expr::val(time.first.to_string()),
                Expr::val(time.last.to_string()),
                Expr::val(time.first_afternoon_only),
                Expr::val(time.last_morning_only),
                Expr::val(time.note.clone()),
                Expr::val(time.entered_by.stored()),
                Expr::val(time.version),
            ])
            .map_err(|_| StoreError::Corrupt("an away time"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn update_away_time(
        transaction: &mut Transaction,
        time: &AwayTime,
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(AwayTimeRow::Table)
            .values([
                (AwayTimeRow::First, Expr::val(time.first.to_string())),
                (AwayTimeRow::Last, Expr::val(time.last.to_string())),
                (
                    AwayTimeRow::FirstAfternoonOnly,
                    Expr::val(time.first_afternoon_only),
                ),
                (
                    AwayTimeRow::LastMorningOnly,
                    Expr::val(time.last_morning_only),
                ),
                (AwayTimeRow::Note, Expr::val(time.note.clone())),
                (AwayTimeRow::Version, Expr::val(time.version)),
            ])
            .and_where(Expr::col(AwayTimeRow::Id).eq(time.id.stored()))
            .and_where(Expr::col(AwayTimeRow::Version).eq(version));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn delete_away_time(
        transaction: &mut Transaction,
        id: AwayTimeId,
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(AwayTimeRow::Table)
            .and_where(Expr::col(AwayTimeRow::Id).eq(id.stored()))
            .and_where(Expr::col(AwayTimeRow::Version).eq(version));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn delete_old_away_times(
        transaction: &mut Transaction,
        before: Date,
    ) -> Result<Vec<(UserId, u64)>, StoreError> {
        old::delete(transaction, before).await
    }

    async fn delete_work_times_of(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<(), StoreError> {
        old::delete_of_person(transaction, person).await
    }
}
