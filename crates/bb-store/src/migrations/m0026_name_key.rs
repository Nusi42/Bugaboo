//! Migration 26: a person is found by the key of the name (M5-F94, ADR-0005 decision 9).
//!
//! `person.name_key` holds the name as names are told apart (`bb_core::names::name_key`: lower
//! case, repeated spaces squeezed, accents kept), as tags, labels and groups keep theirs. The
//! search for the start of a name, the order of the people and the position in that order read it,
//! because `lower()` of a database folds only the letters of ASCII.
//!
//! The migration adds the column (empty for the people that are there) and its index; it does not
//! make a key itself, so it does not depend on the code of today. Every start fills the keys that
//! are still empty with the code of the program that runs, a page at a time
//! (`Database::fill_name_keys`, called by `Database::migrate` right after the migrations). A
//! person is written with the key from then on. A later change of the rule of `name_key` must
//! empty the keys first (a migration that sets them to the empty text), so the start fills them
//! again with the new rule.

use sea_query::{ColumnDef, Index, Table};

use super::Step;
use crate::schema::PersonRow;

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Alter(
            Table::alter()
                .table(PersonRow::Table)
                .add_column(
                    ColumnDef::new(PersonRow::NameKey)
                        .text()
                        .not_null()
                        .default(""),
                )
                .to_owned(),
        ),
        // The people in the order of their names, and the pages of that order. A `LIKE` with an
        // escape does not use it (SQLite skips it; PostgreSQL would need `text_pattern_ops`).
        Step::Index(
            Index::create()
                .name("person_by_name_key")
                .table(PersonRow::Table)
                .col(PersonRow::NameKey)
                .col(PersonRow::Id)
                .to_owned(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use bb_app::auth::{AccountStore as _, SearchIn};
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, ExprTrait as _, Query};

    use super::super::Database;
    use crate::db::StoredId as _;
    use crate::schema::PersonRow;
    use crate::test_support::{at, ids};
    use bb_core::id::UserId;

    /// A person written before the migration has no key; the migrating start gives it one, and
    /// the name is found by its start with a lower-case letter outside ASCII.
    #[tokio::test]
    async fn m5_f94_migration_26_makes_a_person_written_before_it_found_by_the_start_of_the_name()
    -> TestResult {
        each_database("migration_26_name_key", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(25, at(0)).await?;
            let mut insert = Query::insert();
            insert
                .into_table(PersonRow::Table)
                .columns([
                    PersonRow::Id,
                    PersonRow::DisplayName,
                    PersonRow::Email,
                    PersonRow::CreatedAt,
                ])
                .values([
                    Expr::val(UserId::generate(&mut ids()).stored()),
                    Expr::val("\u{d6}zil  Cem"),
                    Expr::val("oz@example.test"),
                    Expr::val(1_i64),
                ])?;
            database.execute(&insert).await?;
            assert_eq!(database.migrate(at(1)).await?, [26, 27, 28]);
            let folder = tempfile::tempdir()?;
            let keys = std::sync::Arc::new(database.open_keys(&folder.path().join("keys")).await?);
            let seal = crate::anchor::LogSeal::new(&keys, folder.path().join("head.log"));
            let storage = crate::AuthStorage::new(database.clone(), keys, seal);
            let found = storage
                .people_page(Some("\u{f6}zil c"), SearchIn::NameStarts, None, 10)
                .await?;
            assert_eq!(found.len(), 1);
            assert_eq!(
                database.fill_name_keys().await?,
                0,
                "nothing is left to fill"
            );
            Ok(())
        })
        .await
    }

    /// More people than one page hold are all given a key, each by the rule.
    #[tokio::test]
    async fn m5_f94_migration_26_fills_the_keys_of_more_people_than_one_page_holds() -> TestResult {
        each_database("migration_26_many_keys", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(25, at(0)).await?;
            let mut source = ids();
            let wanted = 1_100_u32;
            for number in 0..wanted {
                let mut insert = Query::insert();
                insert
                    .into_table(PersonRow::Table)
                    .columns([
                        PersonRow::Id,
                        PersonRow::DisplayName,
                        PersonRow::Email,
                        PersonRow::CreatedAt,
                    ])
                    .values([
                        Expr::val(UserId::generate(&mut source).stored()),
                        Expr::val(format!("\u{d6}ZIL  No {number}")),
                        Expr::val(format!("{number}@example.test")),
                        Expr::val(1_i64),
                    ])?;
                database.execute(&insert).await?;
            }
            database.migrate_until(26, at(1)).await?;
            assert_eq!(database.fill_name_keys().await?, u64::from(wanted));
            assert_eq!(database.fill_name_keys().await?, 0);
            // A key is the value of the rule, not only a non-empty text.
            let mut select = Query::select();
            select
                .column(PersonRow::NameKey)
                .from(PersonRow::Table)
                .and_where(Expr::col(PersonRow::Email).eq("5@example.test"));
            let row = database.fetch_optional(&select).await?.ok_or("no person")?;
            assert_eq!(
                row.get::<String>(PersonRow::NameKey)?,
                bb_core::names::name_key("\u{d6}ZIL  No 5")
            );
            Ok(())
        })
        .await
    }
}
