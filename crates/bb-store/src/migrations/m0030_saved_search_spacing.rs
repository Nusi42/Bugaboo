//! Migration 30: `saved_search` loses `spacing` (ADR-0034, amendment).
//!
//! How close together the rows are is the person's setting *Spacing* (*My settings ›
//! Appearance*), which every list follows; a saved search no longer keeps one. The column is taken
//! away the way migration 20 took away `mention.read_at`: one `ALTER TABLE … DROP COLUMN`, which
//! both SQLite (3.35 and later) and PostgreSQL run in place. No index or key of `saved_search`
//! names the column, so SQLite accepts it. Every other part of a saved search stays as it was.

use sea_query::Table;

use super::Step;
use crate::schema::{Retired, SavedSearchRow};

pub(super) fn steps() -> Vec<Step> {
    vec![Step::Alter(
        Table::alter()
            .table(SavedSearchRow::Table)
            .drop_column(Retired::Spacing)
            .to_owned(),
    )]
}

#[cfg(test)]
mod tests {
    use bb_app::saved_searches::SavedSearchStore as _;
    use bb_core::id::{SavedSearchId, SequentialIds, UserId};
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, ExprTrait as _, Query};

    use super::super::Database;
    use crate::auth::AuthStorage;
    use crate::db::StoredId as _;
    use crate::schema::{Retired, SavedSearchRow};
    use crate::test_support::{at, insert_old_saved_search};

    #[tokio::test]
    async fn qry_06_migration_30_takes_the_spacing_of_a_saved_search_away_and_keeps_the_search()
    -> TestResult {
        each_database("migration_30_saved_search_spacing", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(29, at(0)).await?;
            let mut ids = SequentialIds::default();
            let id = SavedSearchId::generate(&mut ids)?;
            let maker = UserId::generate(&mut ids);
            // A search kept Compact before the spacing was the person's alone.
            insert_old_saved_search(&database, (id.stored(), maker.stored()), "compact").await?;
            assert_eq!(database.migrate_until(30, at(1)).await?, [30]);

            // The column is gone: an UPDATE that names it fails (a SELECT would read the missing
            // column as a string on SQLite).
            let mut with_spacing = Query::update();
            with_spacing
                .table(SavedSearchRow::Table)
                .value(Retired::Spacing, "compact")
                .and_where(Expr::val(1_i64).eq(0_i64));
            assert!(database.execute(&with_spacing).await.is_err());
            // The search opens as it was, its maker and its text kept.
            let mut read = database.begin_read().await?;
            let kept = AuthStorage::saved_search_in(&mut read, id).await?;
            let kept = kept.ok_or("the saved search is gone")?;
            assert_eq!(
                (kept.name.as_str(), kept.text.as_str(), kept.maker),
                ("Open bugs", "status is open", maker)
            );
            Ok(())
        })
        .await
    }
}
