//! Migration 57: the standing of a person (PRIV-04, ADR-0044, decisions 5 and 6).
//!
//! One row for each person who is not active: switched off or removed, when, by whom, and for a
//! removed person the code of the tombstone (unique, so no two removed people share a label) and
//! the legal-hold field (the name, its reason and its last day). An active person has no row. The
//! person has a reference: their row stays, as the tombstone of a removed person. The administrator
//! has none: they may be removed later, and their ID then reads as *Former user*.
//!
//! Beside it, the lines of the replay list that a change wrote and the file does not hold yet
//! (ADR-0044, decision 8): one row for each line, deleted when the file has it.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{PersonRow, PersonStandingRow, ReplayWaitingRow};

/// The number of this migration: a database below it has no table of waiting lines.
pub(crate) const VERSION: i64 = 57;

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(PersonStandingRow::Table)
                .col(id_column(PersonStandingRow::Person).primary_key())
                .col(
                    ColumnDef::new(PersonStandingRow::Standing)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(PersonStandingRow::Since)
                        .big_integer()
                        .not_null(),
                )
                .col(id_column(PersonStandingRow::By))
                .col(ColumnDef::new(PersonStandingRow::Label).text())
                .col(ColumnDef::new(PersonStandingRow::HoldName).text())
                .col(ColumnDef::new(PersonStandingRow::HoldReason).text())
                .col(ColumnDef::new(PersonStandingRow::HoldUntil).text())
                .foreign_key(
                    ForeignKey::create()
                        .from(PersonStandingRow::Table, PersonStandingRow::Person)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        // A label belongs to one removed person; the rows of switched-off people have none.
        Step::Index(
            Index::create()
                .name("person_standing_label")
                .table(PersonStandingRow::Table)
                .col(PersonStandingRow::Label)
                .unique()
                .to_owned(),
        ),
        // The lines of the replay list written in their change, until the file has them.
        Step::Table(
            Table::create()
                .table(ReplayWaitingRow::Table)
                .col(
                    ColumnDef::new(ReplayWaitingRow::Line)
                        .text()
                        .not_null()
                        .primary_key(),
                )
                .to_owned(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use bb_testing::{TestResult, each_database};

    use super::VERSION;
    use crate::auth::replay_waiting::{waiting_lines, waiting_lines_of_any_version};
    use crate::db::Database;
    use crate::test_support::at;

    #[tokio::test]
    async fn priv_04_a_database_before_the_waiting_lines_has_none_to_restore() -> TestResult {
        each_database("m0057_no_waiting_lines_before", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(VERSION - 1, at(0)).await?;
            // A restore onto a database of an older release reads no waiting lines, and fails not.
            assert!(waiting_lines(&database).await.is_err());
            assert_eq!(
                waiting_lines_of_any_version(&database).await?,
                Vec::<String>::new()
            );
            database.migrate_until(VERSION, at(1)).await?;
            assert_eq!(
                waiting_lines_of_any_version(&database).await?,
                Vec::<String>::new()
            );
            Ok(())
        })
        .await
    }
}
