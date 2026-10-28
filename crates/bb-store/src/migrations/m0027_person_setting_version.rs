//! Migration 27: a person's own settings get a version (M5-F97).
//!
//! The same person works in several tabs and on several devices, and without a version the last
//! write won: a size of one form set in one tab undid another form's size set in another. A value
//! that is there is at version 1 from now on (`person_setting.version`), and every change counts
//! it up; a change made from an older version is refused (ADR-0019). A value that is not there is
//! at version 0.

use sea_query::{ColumnDef, Table};

use super::Step;
use crate::schema::PersonSettingRow;

pub(super) fn steps() -> Vec<Step> {
    vec![Step::Alter(
        Table::alter()
            .table(PersonSettingRow::Table)
            .add_column(
                ColumnDef::new(PersonSettingRow::Version)
                    .big_integer()
                    .not_null()
                    .default(1),
            )
            .to_owned(),
    )]
}

#[cfg(test)]
mod tests {
    use bb_core::id::UserId;
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, ExprTrait as _, Query};

    use super::super::Database;
    use crate::db::StoredId as _;
    use crate::schema::{PersonRow, PersonSettingRow};
    use crate::test_support::{at, ids};

    #[tokio::test]
    async fn m5_f97_migration_27_puts_a_value_written_before_it_at_version_1() -> TestResult {
        each_database("m5_f97_migration_27", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(26, at(0)).await?;
            let person = UserId::generate(&mut ids()).stored();
            let mut made = Query::insert();
            made.into_table(PersonRow::Table)
                .columns([
                    PersonRow::Id,
                    PersonRow::DisplayName,
                    PersonRow::Email,
                    PersonRow::CreatedAt,
                ])
                .values([
                    Expr::val(person.clone()),
                    Expr::val("Ada"),
                    Expr::val("ada@example.test"),
                    Expr::val(1_i64),
                ])?;
            database.execute(&made).await?;
            let mut old = Query::insert();
            old.into_table(PersonSettingRow::Table)
                .columns([
                    PersonSettingRow::Person,
                    PersonSettingRow::Key,
                    PersonSettingRow::Value,
                    PersonSettingRow::UpdatedAt,
                ])
                .values([
                    Expr::val(person.clone()),
                    Expr::val("appearance.mode"),
                    Expr::val("\"dark\""),
                    Expr::val(2_i64),
                ])?;
            database.execute(&old).await?;
            assert_eq!(database.migrate(at(1)).await?, [27, 28]);
            let mut select = Query::select();
            select
                .column(PersonSettingRow::Version)
                .from(PersonSettingRow::Table)
                .and_where(Expr::col(PersonSettingRow::Person).eq(person));
            let row = database.fetch_optional(&select).await?.unwrap();
            assert_eq!(row.get::<i64>(PersonSettingRow::Version)?, 1);
            Ok(())
        })
        .await
    }
}
