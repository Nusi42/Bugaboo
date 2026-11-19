//! The standing of a person (PRIV-04): the adapter of `bb_app::people::StandingStore`, and the
//! standing that every read of people carries.

use std::collections::BTreeSet;

use bb_app::auth::PeopleOf;
use bb_app::people::StandingStore;
use bb_core::id::UserId;
use bb_core::person::{HoldReason, PersonStanding, Standing};
use sea_query::{Cond, Expr, ExprTrait as _, OnConflict, Query, SelectStatement};

use super::AuthStorage;
use crate::db::{Row, StoredId as _, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{PersonRow, PersonStandingRow};

/// The columns of a standing.
pub(crate) const STANDING_COLUMNS: [PersonStandingRow; 8] = [
    PersonStandingRow::Person,
    PersonStandingRow::Standing,
    PersonStandingRow::Since,
    PersonStandingRow::By,
    PersonStandingRow::Label,
    PersonStandingRow::HoldName,
    PersonStandingRow::HoldReason,
    PersonStandingRow::HoldUntil,
];

/// Joins the standing of each person to a select of people: the row has none while they are
/// active.
pub(crate) fn join_standing(select: &mut SelectStatement) {
    select.columns(STANDING_COLUMNS.map(|column| (PersonStandingRow::Table, column)));
    join_standing_rows(select);
}

/// Joins the standing of each person to a select of people without reading it: enough for
/// [`only`].
pub(crate) fn join_standing_rows(select: &mut SelectStatement) {
    select.left_join(
        PersonStandingRow::Table,
        Expr::col((PersonStandingRow::Table, PersonStandingRow::Person))
            .equals((PersonRow::Table, PersonRow::Id)),
    );
}

/// Keeps the people of `whom` in a select of people joined with [`join_standing`].
pub(crate) fn only(select: &mut SelectStatement, whom: PeopleOf) {
    let state = Expr::col((PersonStandingRow::Table, PersonStandingRow::Standing));
    match whom {
        PeopleOf::Active => select.and_where(state.is_null()),
        PeopleOf::NotRemoved => select.cond_where(
            Cond::any()
                .add(state.clone().is_null())
                .add(state.ne(Standing::Removed.code())),
        ),
    };
}

/// The standing of a row read with [`join_standing`] or from the table itself; none for an active
/// person.
pub(crate) fn read_standing(row: &Row) -> Result<Option<PersonStanding>, StoreError> {
    let Some(state) = row.get::<Option<String>>(PersonStandingRow::Standing)? else {
        return Ok(None);
    };
    let corrupt = || StoreError::Corrupt("the standing of a person");
    let hold_until = row
        .get::<Option<String>>(PersonStandingRow::HoldUntil)?
        .map(|date| date.parse().map_err(|_| corrupt()))
        .transpose()?;
    let hold_reason = row
        .get::<Option<String>>(PersonStandingRow::HoldReason)?
        .map(|reason| HoldReason::from_code(&reason).ok_or_else(corrupt))
        .transpose()?;
    Ok(Some(PersonStanding {
        person: row.id(PersonStandingRow::Person)?,
        standing: Standing::from_code(&state).ok_or_else(corrupt)?,
        since: row.time(PersonStandingRow::Since)?,
        by: row.id(PersonStandingRow::By)?,
        label: row.get(PersonStandingRow::Label)?,
        hold_name: row.get(PersonStandingRow::HoldName)?,
        hold_reason,
        hold_until,
    }))
}

/// Everybody who is switched off or removed.
pub(crate) async fn not_active_in(
    transaction: &mut Transaction,
) -> Result<BTreeSet<UserId>, StoreError> {
    let mut select = Query::select();
    select
        .column(PersonStandingRow::Person)
        .from(PersonStandingRow::Table);
    transaction
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| row.id(PersonStandingRow::Person))
        .collect()
}

impl StandingStore for AuthStorage {
    async fn standing_in(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Option<PersonStanding>, StoreError> {
        let mut select = Query::select();
        select
            .columns(STANDING_COLUMNS)
            .from(PersonStandingRow::Table)
            .and_where(Expr::col(PersonStandingRow::Person).eq(person.stored()));
        match transaction.fetch_optional(&select).await? {
            Some(row) => read_standing(&row),
            None => Ok(None),
        }
    }

    async fn former_labels(&self) -> Result<Vec<String>, StoreError> {
        let mut select = Query::select();
        select
            .column(PersonStandingRow::Label)
            .from(PersonStandingRow::Table)
            .and_where(Expr::col(PersonStandingRow::Label).is_not_null());
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| row.get(PersonStandingRow::Label))
            .collect()
    }

    async fn set_standing(
        transaction: &mut Transaction,
        standing: &PersonStanding,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(PersonStandingRow::Table)
            .columns(STANDING_COLUMNS)
            .values([
                Expr::val(standing.person.stored()),
                Expr::val(standing.standing.code()),
                Expr::val(to_micros(standing.since)),
                Expr::val(standing.by.stored()),
                Expr::val(standing.label.clone()),
                Expr::val(standing.hold_name.clone()),
                Expr::val(standing.hold_reason.map(HoldReason::code)),
                Expr::val(standing.hold_until.map(|date| date.to_string())),
            ])
            .map_err(|_| StoreError::Corrupt("the standing of a person"))?
            .on_conflict(
                OnConflict::column(PersonStandingRow::Person)
                    .update_columns(STANDING_COLUMNS[1..].iter().copied())
                    .to_owned(),
            );
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn clear_standing(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(PersonStandingRow::Table)
            .and_where(Expr::col(PersonStandingRow::Person).eq(person.stored()))
            .and_where(Expr::col(PersonStandingRow::Standing).eq(Standing::SwitchedOff.code()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn clear_legal_hold(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(PersonStandingRow::Table)
            .values([
                (PersonStandingRow::HoldName, Option::<String>::None.into()),
                (PersonStandingRow::HoldReason, Option::<String>::None.into()),
                (PersonStandingRow::HoldUntil, Option::<String>::None.into()),
            ])
            .and_where(Expr::col(PersonStandingRow::Person).eq(person.stored()))
            .and_where(Expr::col(PersonStandingRow::HoldName).is_not_null());
        Ok(transaction.execute(&update).await? > 0)
    }
}

#[cfg(test)]
#[path = "../tests/standing.rs"]
mod tests;
