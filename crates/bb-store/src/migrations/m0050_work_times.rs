//! Migration 50: work times (PLAN-11, PLAN-15, ADR-0030, decision 2 and amendment M8).
//!
//! The usual work days of a person (one row for each weekday), the share of a person in a project
//! and the away times. A date is `YYYY-MM-DD` so it sorts as text. Nothing is written into these
//! tables. **No table has a column for the reason of an away time**: Bugaboo does not know whether
//! it is a holiday, a course or an illness (GDPR Art. 9). The tables hold no reference to the table
//! of people: a person's work times are deleted by the one function that deletes them when the
//! person leaves the organization.

use sea_query::{ColumnDef, Index, Table};

use super::{Step, id_column};
use crate::schema::{AwayTimeRow, ProjectShareRow, UsualWorkDayRow};

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(UsualWorkDayRow::Table)
                .col(id_column(UsualWorkDayRow::Person))
                .col(
                    ColumnDef::new(UsualWorkDayRow::Weekday)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(UsualWorkDayRow::Works).boolean().not_null())
                .col(
                    ColumnDef::new(UsualWorkDayRow::Minutes)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(UsualWorkDayRow::Person)
                        .col(UsualWorkDayRow::Weekday),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(ProjectShareRow::Table)
                .col(id_column(ProjectShareRow::Person))
                .col(id_column(ProjectShareRow::Project))
                .col(
                    ColumnDef::new(ProjectShareRow::Percent)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(ProjectShareRow::Person)
                        .col(ProjectShareRow::Project),
                )
                .to_owned(),
        ),
        // The shares of a project: the people of one project with their shares.
        Step::Index(
            Index::create()
                .name("project_share_by_project")
                .table(ProjectShareRow::Table)
                .col(ProjectShareRow::Project)
                .col(ProjectShareRow::Person)
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(AwayTimeRow::Table)
                .col(
                    ColumnDef::new(AwayTimeRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(id_column(AwayTimeRow::Person))
                .col(ColumnDef::new(AwayTimeRow::First).text().not_null())
                .col(ColumnDef::new(AwayTimeRow::Last).text().not_null())
                .col(
                    ColumnDef::new(AwayTimeRow::FirstAfternoonOnly)
                        .boolean()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(AwayTimeRow::LastMorningOnly)
                        .boolean()
                        .not_null(),
                )
                .col(ColumnDef::new(AwayTimeRow::Note).text())
                .col(id_column(AwayTimeRow::EnteredBy))
                .col(
                    ColumnDef::new(AwayTimeRow::Version)
                        .big_integer()
                        .not_null(),
                )
                .to_owned(),
        ),
        // The away times of a person that touch a stretch of days.
        Step::Index(
            Index::create()
                .name("away_time_by_person_first")
                .table(AwayTimeRow::Table)
                .col(AwayTimeRow::Person)
                .col(AwayTimeRow::First)
                .to_owned(),
        ),
        // The away times that are old enough to delete, of everybody.
        Step::Index(
            Index::create()
                .name("away_time_by_last")
                .table(AwayTimeRow::Table)
                .col(AwayTimeRow::Last)
                .to_owned(),
        ),
    ]
}
