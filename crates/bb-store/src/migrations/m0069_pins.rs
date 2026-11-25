//! Migration 69: the pins of a person (CFG-05, ADR-0050, decision 12).
//!
//! One row for each pin: the person, what it points to (a kind and the ID of the record) and its
//! place among the person's pins. A person pins a record once, so the key is the person with the
//! kind and the record; it starts with the person, so it also finds the pins of a person, which the
//! sidebar reads and the walk of a person's data asks (ADR-0044). The target has no reference: it is
//! a record of one of several tables, and a pin goes with its record through the use case that
//! deletes it.

use sea_query::{ColumnDef, Index, Table};

use super::{Step, id_column};
use crate::schema::PinRow;

pub(super) fn steps() -> Vec<Step> {
    vec![Step::Table(
        Table::create()
            .table(PinRow::Table)
            .col(id_column(PinRow::Person))
            .col(ColumnDef::new(PinRow::Kind).text().not_null())
            .col(id_column(PinRow::Target))
            .col(ColumnDef::new(PinRow::Place).big_integer().not_null())
            .primary_key(
                Index::create()
                    .col(PinRow::Person)
                    .col(PinRow::Kind)
                    .col(PinRow::Target),
            )
            .to_owned(),
    )]
}

#[cfg(test)]
mod tests {
    use bb_app::pins::PinStore as _;
    use bb_core::id::UserId;
    use bb_testing::{TestResult, each_database};

    use super::super::Database;
    use crate::auth::AuthStorage;
    use crate::test_support::at;

    #[tokio::test]
    async fn cfg_05_migration_69_makes_the_table_of_pins() -> TestResult {
        each_database("migration_69_pins", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(66, at(0)).await?;
            assert_eq!(database.migrate_until(69, at(1)).await?, [67, 68, 69]);
            let mut read = database.begin_read().await?;
            let person = UserId::from_random([9; 16]);
            assert!(AuthStorage::pins_of_in(&mut read, person).await?.is_empty());
            Ok(())
        })
        .await
    }
}
