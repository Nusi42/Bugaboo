//! Migration 28: the versions of a person's own settings never go back (M5-F97).
//!
//! Migration 27 gave each value a version that counted up from 1, so a reset (which deletes the
//! row) followed by a new value started again at 1, and a tab that had read version 1 before
//! could overwrite the new value unseen. Now one counter per person gives the version of every
//! write of any of their values. The migration makes the counter of each person the highest
//! version they hold, so no version already given out is given again.

use sea_query::{ColumnDef, Expr, ForeignKey, Func, Query, Table};

use super::Step;
use crate::schema::{PersonRow, PersonSettingCounterRow, PersonSettingRow};

pub(super) fn steps() -> Result<Vec<Step>, sea_query::error::Error> {
    let mut highest = Query::select();
    highest
        .column(PersonSettingRow::Person)
        .expr(Func::max(Expr::col(PersonSettingRow::Version)))
        .from(PersonSettingRow::Table)
        .group_by_col(PersonSettingRow::Person);
    let mut seed = Query::insert();
    seed.into_table(PersonSettingCounterRow::Table)
        .columns([
            PersonSettingCounterRow::Person,
            PersonSettingCounterRow::Counter,
        ])
        .select_from(highest)?;
    Ok(vec![
        Step::Table(
            Table::create()
                .table(PersonSettingCounterRow::Table)
                .col(
                    ColumnDef::new(PersonSettingCounterRow::Person)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(PersonSettingCounterRow::Counter)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(
                            PersonSettingCounterRow::Table,
                            PersonSettingCounterRow::Person,
                        )
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        Step::Seed(seed),
    ])
}

#[cfg(test)]
mod tests {
    use bb_core::id::UserId;
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, ExprTrait as _, Query};

    use super::super::Database;
    use crate::db::StoredId as _;
    use crate::schema::{PersonSettingCounterRow, PersonSettingRow};
    use crate::test_support::{at, ids, insert_old_person};

    #[tokio::test]
    async fn m5_f97_migration_28_starts_the_counter_at_the_highest_version_a_person_holds()
    -> TestResult {
        each_database("m5_f97_migration_28", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(27, at(0)).await?;
            let person = UserId::generate(&mut ids()).stored();
            insert_old_person(&database, person.clone()).await?;
            for (key, version) in [("appearance.mode", 3_i64), ("region.time-zone", 9_i64)] {
                let mut old = Query::insert();
                old.into_table(PersonSettingRow::Table)
                    .columns([
                        PersonSettingRow::Person,
                        PersonSettingRow::Key,
                        PersonSettingRow::Value,
                        PersonSettingRow::UpdatedAt,
                        PersonSettingRow::Version,
                    ])
                    .values([
                        Expr::val(person.clone()),
                        Expr::val(key),
                        Expr::val("\"x\""),
                        Expr::val(2_i64),
                        Expr::val(version),
                    ])?;
                database.execute(&old).await?;
            }
            assert_eq!(database.migrate(at(1)).await?, [28, 29]);
            let mut select = Query::select();
            select
                .column(PersonSettingCounterRow::Counter)
                .from(PersonSettingCounterRow::Table)
                .and_where(Expr::col(PersonSettingCounterRow::Person).eq(person));
            let row = database.fetch_optional(&select).await?.unwrap();
            assert_eq!(row.get::<i64>(PersonSettingCounterRow::Counter)?, 9);
            Ok(())
        })
        .await
    }
}
