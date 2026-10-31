//! The timer of each person and the marks of the starter fields: their rows in `timer` and
//! `starter_field`.

use bb_core::id::{FieldId, IssueId, UserId};
use bb_core::time::entry::{Running, StarterField, StarterFieldMark, Timer};
use bb_core::version::Version;
use sea_query::{Expr, ExprTrait as _, OnConflict, Query};

use crate::db::{Row, StoredId as _, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{StarterFieldRow, TimerRow};

/// The columns of a timer, in the order they are written.
const TIMER_COLUMNS: [TimerRow; 6] = [
    TimerRow::Person,
    TimerRow::Issue,
    TimerRow::StartedAt,
    TimerRow::WarnedAt,
    TimerRow::ChangedAt,
    TimerRow::Version,
];

fn read_timer(row: &Row) -> Result<Timer, StoreError> {
    let issue = row.optional_id::<IssueId>(TimerRow::Issue)?;
    let started_at = row.optional_time(TimerRow::StartedAt)?;
    let running = match (issue, started_at) {
        (Some(issue), Some(started_at)) => Some(Running {
            issue,
            started_at,
            warned_at: row.optional_time(TimerRow::WarnedAt)?,
        }),
        (None, None) => None,
        _ => return Err(StoreError::Corrupt("a timer")),
    };
    Ok(Timer {
        person: row.id::<UserId>(TimerRow::Person)?,
        running,
        changed_at: row.time(TimerRow::ChangedAt)?,
        version: row.get(TimerRow::Version)?,
    })
}

/// The values of a timer's columns, in the order of [`TIMER_COLUMNS`].
fn timer_values(timer: &Timer) -> [Expr; 6] {
    let running = timer.running();
    [
        Expr::val(timer.person.stored()),
        Expr::val(running.map(|run| run.issue.stored())),
        Expr::val(running.map(|run| to_micros(run.started_at))),
        Expr::val(running.and_then(|run| run.warned_at).map(to_micros)),
        Expr::val(to_micros(timer.changed_at)),
        Expr::val(timer.version),
    ]
}

/// The timer of a person.
pub(super) async fn timer_in(
    transaction: &mut Transaction,
    person: UserId,
) -> Result<Option<Timer>, StoreError> {
    let mut select = Query::select();
    select
        .columns(TIMER_COLUMNS)
        .from(TimerRow::Table)
        .and_where(Expr::col(TimerRow::Person).eq(person.stored()));
    transaction
        .fetch_optional(&select)
        .await?
        .as_ref()
        .map(read_timer)
        .transpose()
}

/// Writes a timer: a new row when `version` is none (false when the person has one), else the row
/// that is still at `version` (false when it is not).
pub(super) async fn put_timer(
    transaction: &mut Transaction,
    timer: &Timer,
    version: Option<Version>,
) -> Result<bool, StoreError> {
    let values = timer_values(timer);
    let Some(version) = version else {
        let mut insert = Query::insert();
        insert
            .into_table(TimerRow::Table)
            .columns(TIMER_COLUMNS)
            .values(values)
            .map_err(|_| StoreError::Corrupt("a timer"))?
            .on_conflict(OnConflict::column(TimerRow::Person).do_nothing().to_owned());
        return Ok(transaction.execute(&insert).await? > 0);
    };
    let mut update = Query::update();
    update.table(TimerRow::Table);
    for (column, value) in TIMER_COLUMNS.into_iter().zip(values).skip(1) {
        update.value(column, value);
    }
    update
        .and_where(Expr::col(TimerRow::Person).eq(timer.person.stored()))
        .and_where(Expr::col(TimerRow::Version).eq(version.get()));
    Ok(transaction.execute(&update).await? > 0)
}

/// The marks of the starter fields that were made.
pub(super) async fn starter_fields_in(
    transaction: &mut Transaction,
) -> Result<Vec<StarterFieldMark>, StoreError> {
    let mut select = Query::select();
    select
        .columns([StarterFieldRow::Starter, StarterFieldRow::Field])
        .from(StarterFieldRow::Table);
    transaction
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| {
            Ok(StarterFieldMark {
                starter: StarterField::from_code(&row.get::<String>(StarterFieldRow::Starter)?)
                    .ok_or(StoreError::Corrupt("a starter field"))?,
                field: row.id::<FieldId>(StarterFieldRow::Field)?,
            })
        })
        .collect()
}

/// Writes the mark of a starter field; false when the mark is there already.
pub(super) async fn insert_starter_field(
    transaction: &mut Transaction,
    mark: &StarterFieldMark,
) -> Result<bool, StoreError> {
    let mut insert = Query::insert();
    insert
        .into_table(StarterFieldRow::Table)
        .columns([StarterFieldRow::Starter, StarterFieldRow::Field])
        .values([
            Expr::val(mark.starter.code()),
            Expr::val(mark.field.stored()),
        ])
        .map_err(|_| StoreError::Corrupt("a starter field"))?
        .on_conflict(
            OnConflict::column(StarterFieldRow::Starter)
                .do_nothing()
                .to_owned(),
        );
    Ok(transaction.execute(&insert).await? > 0)
}
