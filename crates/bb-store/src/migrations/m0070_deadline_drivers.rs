//! Migration 70: the switch *Deadline drivers* of the timeline of a saved search (PLAN-05, ADR-0043,
//! amendment of 2026-11-24), kept with the options of its timeline like *Plan by effort*. Every
//! search kept before has it off.

use sea_query::{ColumnDef, Table};

use super::Step;
use crate::schema::SavedSearchRow;

pub(super) fn steps() -> Vec<Step> {
    let mut drivers = ColumnDef::new(SavedSearchRow::DeadlineDrivers);
    drivers.boolean().not_null().default(false);
    vec![Step::Alter(
        Table::alter()
            .table(SavedSearchRow::Table)
            .add_column(drivers)
            .to_owned(),
    )]
}

#[cfg(test)]
mod tests {
    use bb_app::saved_searches::SavedSearchStore as _;
    use bb_core::id::{SavedSearchId, UserId};
    use bb_core::saved_search::TimelineOptions;
    use bb_testing::{TestResult, each_database};
    use jiff::Timestamp;

    use super::super::Database;
    use super::super::old_rows::old_saved_list;
    use crate::auth::AuthStorage;
    use crate::test_support::at;

    #[tokio::test]
    async fn plan_05_migration_70_starts_every_saved_search_with_deadline_drivers_off() -> TestResult
    {
        each_database("migration_70_drivers", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(66, at(0)).await?;
            let search = SavedSearchId::from_parts(Timestamp::UNIX_EPOCH, [7; 10])?;
            let maker = UserId::from_random([9; 16]);
            old_saved_list(&database, search, maker).await?;
            assert_eq!(database.migrate_until(70, at(1)).await?, [70]);
            let mut read = database.begin_read().await?;
            let kept = AuthStorage::saved_search_in(&mut read, search)
                .await?
                .ok_or("the saved search is gone")?;
            assert_eq!(kept.timeline, TimelineOptions::default());
            assert!(!kept.timeline.deadline_drivers);
            Ok(())
        })
        .await
    }
}
