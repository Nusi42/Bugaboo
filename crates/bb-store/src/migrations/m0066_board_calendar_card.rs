//! Migration 66: the options of a board and a calendar on a saved search, and the layout of the
//! card (PLAN-06, PLAN-09, ADR-0034 and ADR-0027, amendments of 2026-11-24).
//!
//! - The board: the field its columns come from and the field of its rows, by their search words
//!   (none for *Status* and for no rows), and its card limits, one row for each column that has
//!   one, in order.
//! - The calendar: the date field the issues stand on (none for *Due date*) and what it shows
//!   first (a month). Every search kept before starts with these.
//! - The layout *Card* is a kind of layout like the issue page: its rows are those of the layout
//!   tables, under the kind `card`, and none exists until somebody customizes a card, so the
//!   starting card counts.
//!
//! One `ALTER TABLE … ADD COLUMN` for each column, because SQLite adds one column at a time.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{SavedSearchRow, SearchCardLimitRow};

/// What the calendar of every saved search kept before shows first, as it is stored.
const MONTH: &str = "month";

pub(super) fn steps() -> Vec<Step> {
    let mut span = ColumnDef::new(SavedSearchRow::CalendarSpan);
    span.text().not_null().default(MONTH);
    let add = |column: ColumnDef| {
        Step::Alter(
            Table::alter()
                .table(SavedSearchRow::Table)
                .add_column(column)
                .to_owned(),
        )
    };
    vec![
        add(ColumnDef::new(SavedSearchRow::BoardColumns)
            .text()
            .to_owned()),
        add(ColumnDef::new(SavedSearchRow::BoardRows).text().to_owned()),
        add(ColumnDef::new(SavedSearchRow::CalendarDates)
            .text()
            .to_owned()),
        add(span),
        Step::Table(
            Table::create()
                .table(SearchCardLimitRow::Table)
                .col(id_column(SearchCardLimitRow::Search))
                .col(
                    ColumnDef::new(SearchCardLimitRow::Position)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(SearchCardLimitRow::ColumnValue).text())
                .col(
                    ColumnDef::new(SearchCardLimitRow::CardLimit)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(SearchCardLimitRow::Search)
                        .col(SearchCardLimitRow::Position),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(SearchCardLimitRow::Table, SearchCardLimitRow::Search)
                        .to(SavedSearchRow::Table, SavedSearchRow::Id),
                )
                .to_owned(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use bb_app::saved_searches::SavedSearchStore as _;
    use bb_core::id::{SavedSearchId, UserId};
    use bb_core::saved_search::{BoardOptions, CalendarOptions, CalendarSpan};
    use bb_testing::{TestResult, each_database};
    use jiff::Timestamp;

    use super::super::Database;
    use super::super::old_rows::old_saved_list;
    use crate::auth::AuthStorage;
    use crate::test_support::at;

    #[tokio::test]
    async fn plan_06_migration_66_starts_every_saved_search_with_a_status_board_and_a_month()
    -> TestResult {
        each_database("migration_66_views", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(65, at(0)).await?;
            let search = SavedSearchId::from_parts(Timestamp::UNIX_EPOCH, [6; 10])?;
            let maker = UserId::from_random([9; 16]);
            old_saved_list(&database, search, maker).await?;
            assert_eq!(database.migrate_until(66, at(1)).await?, [66]);
            // The search is read as the program reads it now, with the columns of later migrations.
            database.migrate(at(2)).await?;
            let mut read = database.begin_read().await?;
            let kept = AuthStorage::saved_search_in(&mut read, search)
                .await?
                .ok_or("the saved search is gone")?;
            assert_eq!(kept.board, BoardOptions::default());
            assert_eq!(
                kept.calendar,
                CalendarOptions {
                    dates: None,
                    span: CalendarSpan::Month,
                }
            );
            Ok(())
        })
        .await
    }
}
