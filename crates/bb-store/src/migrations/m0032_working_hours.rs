//! Migration 32: working hours (TIME-08, ADR-0035, decisions 5 to 7).
//!
//! A work day, its periods of work and its breaks: at most one day for a person and a date (a
//! unique key), every moment in microseconds since 1970 in UTC, the date as `YYYY-MM-DD` so it
//! sorts as text. The person has no reference to the table of people: a removed person's days stay
//! with the link (ADR-0035, decision 7). Nothing is written into these tables.
//!
//! The starting role *HR* (ADR-0025, amendment M6) is made by the use case of the starting roles,
//! never here: a server whose starting roles were made before it existed gets a row in
//! `starting_role_due`, and the next start makes the role once and takes the row away.

use sea_query::{ColumnDef, Expr, ForeignKey, Index, Query, Table};

use super::{Step, id_column};
use crate::schema::{
    StartingRoleDueRow, StartingRolesMadeRow, WorkBreakRow, WorkDayRow, WorkPeriodRow,
};

/// The key of the starting role that came after the starting roles were first made: the frozen code
/// of `StartingRole::Hr`, written out because an applied migration never changes with the program
/// (a test holds the two together).
const HR: &str = "hr";

pub(super) fn steps() -> Result<Vec<Step>, sea_query::error::Error> {
    let mut steps = days();
    steps.extend(spans());
    steps.extend(due_roles()?);
    Ok(steps)
}

fn days() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(WorkDayRow::Table)
                .col(
                    ColumnDef::new(WorkDayRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(id_column(WorkDayRow::Person))
                .col(ColumnDef::new(WorkDayRow::Date).text().not_null())
                .col(ColumnDef::new(WorkDayRow::Zone).text().not_null())
                .col(ColumnDef::new(WorkDayRow::Warnings).text().not_null())
                .col(
                    ColumnDef::new(WorkDayRow::UpdatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(WorkDayRow::Version).big_integer().not_null())
                .to_owned(),
        ),
        // One day for a person and a date; the days of one person by date.
        Step::Index(
            Index::create()
                .name("work_day_by_person_date")
                .table(WorkDayRow::Table)
                .col(WorkDayRow::Person)
                .col(WorkDayRow::Date)
                .unique()
                .to_owned(),
        ),
        // The days older than the keep period, of everybody.
        Step::Index(
            Index::create()
                .name("work_day_by_date")
                .table(WorkDayRow::Table)
                .col(WorkDayRow::Date)
                .col(WorkDayRow::Person)
                .to_owned(),
        ),
    ]
}

fn spans() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(WorkPeriodRow::Table)
                .col(id_column(WorkPeriodRow::Day))
                .col(
                    ColumnDef::new(WorkPeriodRow::Start)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(WorkPeriodRow::End).big_integer())
                .primary_key(
                    Index::create()
                        .col(WorkPeriodRow::Day)
                        .col(WorkPeriodRow::Start),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(WorkPeriodRow::Table, WorkPeriodRow::Day)
                        .to(WorkDayRow::Table, WorkDayRow::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(WorkBreakRow::Table)
                .col(id_column(WorkBreakRow::Day))
                .col(ColumnDef::new(WorkBreakRow::Start).big_integer().not_null())
                .col(ColumnDef::new(WorkBreakRow::End).big_integer())
                .primary_key(
                    Index::create()
                        .col(WorkBreakRow::Day)
                        .col(WorkBreakRow::Start),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(WorkBreakRow::Table, WorkBreakRow::Day)
                        .to(WorkDayRow::Table, WorkDayRow::Id),
                )
                .to_owned(),
        ),
    ]
}

fn due_roles() -> Result<Vec<Step>, sea_query::error::Error> {
    let mut made = Query::select();
    made.expr(Expr::val(HR))
        .from(StartingRolesMadeRow::Table)
        .limit(1);
    let mut due = Query::insert();
    due.into_table(StartingRoleDueRow::Table)
        .columns([StartingRoleDueRow::Key])
        .select_from(made)?;
    Ok(vec![
        Step::Table(
            Table::create()
                .table(StartingRoleDueRow::Table)
                .col(
                    ColumnDef::new(StartingRoleDueRow::Key)
                        .text()
                        .not_null()
                        .primary_key(),
                )
                .to_owned(),
        ),
        Step::Seed(due),
    ])
}

#[cfg(test)]
mod tests {
    use bb_policy::StartingRole;
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, ExprTrait as _, Query};

    use super::super::Database;
    use super::*;
    use crate::test_support::at;

    #[test]
    fn the_key_is_the_code_of_the_starting_role_hr() {
        assert_eq!(HR, StartingRole::Hr.code());
    }

    // A server whose starting roles were made before HR existed gets HR due once; a new server
    // gets nothing due, because its first start makes every starting role.
    #[tokio::test]
    async fn sec_08_migration_32_makes_hr_due_only_where_the_starting_roles_were_made() -> TestResult
    {
        each_database("migration_32_hr_due", async |db| {
            let due = async |database: &Database| -> Result<usize, crate::error::StoreError> {
                let mut select = Query::select();
                select
                    .column(StartingRoleDueRow::Key)
                    .from(StartingRoleDueRow::Table)
                    .and_where(Expr::col(StartingRoleDueRow::Key).eq(HR));
                Ok(database.fetch_all(&select).await?.len())
            };
            let old = Database::connect(db.url()).await?;
            old.migrate_until(30, at(0)).await?;
            let mut made = Query::insert();
            made.into_table(StartingRolesMadeRow::Table)
                .columns([StartingRolesMadeRow::MadeAt])
                .values_panic([Expr::val(1_i64)]);
            old.execute(&made).await?;
            old.migrate(at(1)).await?;
            assert_eq!(due(&old).await?, 1);
            Ok(())
        })
        .await?;
        each_database("migration_32_nothing_due", async |db| {
            let fresh = Database::connect(db.url()).await?;
            fresh.migrate(at(0)).await?;
            let mut select = Query::select();
            select
                .column(StartingRoleDueRow::Key)
                .from(StartingRoleDueRow::Table);
            assert!(fresh.fetch_all(&select).await?.is_empty());
            Ok(())
        })
        .await
    }
}
