//! The adapter of the pin port (`bb_app::pins::PinStore`): what each person pinned (CFG-05).

use bb_app::pins::PinStore;
use bb_core::id::{EntityId, UserId};
use bb_core::pin::{Pin, PinKind, PinTarget};
use sea_query::{Expr, ExprTrait as _, Order, Query};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction};
use crate::error::StoreError;
use crate::locks::{LockClass, lock_key};
use crate::schema::PinRow;

fn read_pin(row: &Row) -> Result<Pin, StoreError> {
    Ok(Pin {
        person: row.id::<UserId>(PinRow::Person)?,
        kind: PinKind::from_code(&row.get::<String>(PinRow::Kind)?)
            .ok_or(StoreError::Corrupt("a kind of pin"))?,
        target: row.id::<EntityId>(PinRow::Target)?,
        place: row.get(PinRow::Place)?,
    })
}

impl PinStore for AuthStorage {
    async fn pins_of_in(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Vec<Pin>, StoreError> {
        let mut select = Query::select();
        select
            .columns([PinRow::Person, PinRow::Kind, PinRow::Target, PinRow::Place])
            .from(PinRow::Table)
            .and_where(Expr::col(PinRow::Person).eq(person.stored()))
            .order_by(PinRow::Place, Order::Asc)
            .order_by(PinRow::Target, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_pin)
            .collect()
    }

    async fn lock_pins_of(transaction: &mut Transaction, person: UserId) -> Result<(), StoreError> {
        transaction
            .advisory_lock(LockClass::SavedSearches, lock_key(person.to_bytes()))
            .await
    }

    async fn insert_pin(
        transaction: &mut Transaction,
        person: UserId,
        target: PinTarget,
        place: i64,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(PinRow::Table)
            .columns([PinRow::Person, PinRow::Kind, PinRow::Target, PinRow::Place])
            .values([
                Expr::val(person.stored()),
                Expr::val(target.kind().code()),
                Expr::val(target.record().stored()),
                Expr::val(place),
            ])
            .map_err(|_| StoreError::Corrupt("a pin"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn delete_pin(
        transaction: &mut Transaction,
        person: UserId,
        target: PinTarget,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(PinRow::Table)
            .and_where(Expr::col(PinRow::Person).eq(person.stored()))
            .and_where(Expr::col(PinRow::Kind).eq(target.kind().code()))
            .and_where(Expr::col(PinRow::Target).eq(target.record().stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn put_pin_places(
        transaction: &mut Transaction,
        person: UserId,
        places: &[(PinTarget, i64)],
    ) -> Result<(), StoreError> {
        for (target, place) in places {
            let mut update = Query::update();
            update
                .table(PinRow::Table)
                .value(PinRow::Place, *place)
                .and_where(Expr::col(PinRow::Person).eq(person.stored()))
                .and_where(Expr::col(PinRow::Kind).eq(target.kind().code()))
                .and_where(Expr::col(PinRow::Target).eq(target.record().stored()));
            transaction.execute(&update).await?;
        }
        Ok(())
    }

    async fn delete_pins_to(
        transaction: &mut Transaction,
        targets: &[PinTarget],
    ) -> Result<(), StoreError> {
        for target in targets {
            let mut delete = Query::delete();
            delete
                .from_table(PinRow::Table)
                .and_where(Expr::col(PinRow::Kind).eq(target.kind().code()))
                .and_where(Expr::col(PinRow::Target).eq(target.record().stored()));
            transaction.execute(&delete).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/pins.rs"]
mod tests;
