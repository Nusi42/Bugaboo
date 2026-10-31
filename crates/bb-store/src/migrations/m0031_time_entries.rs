//! Migration 31: time on issues (TIME-01, TIME-02, TIME-13, ADR-0035): the timer of each person, the
//! time entries, and the marks of the starter fields of time entries.
//!
//! - `timer` has one row for each person who ever started a timer; the person is the primary key,
//!   so two devices that start at the same moment make one timer and one refusal. It runs while it
//!   names an issue.
//! - `time_entry` holds the entries, with the day as the number of days since 1970 (the way a day
//!   of an own field is stored). The index of the issue serves the entries of an issue and the
//!   minutes spent on it; the index of the person and the day serves a person's week and the count
//!   of a day.
//! - `time_entry_value` holds the values of the own fields of time entries: the columns of
//!   `field_value`, apart, because every part of them is about a person's time (`employee_time`).
//! - `starter_field` names the own field of each starter mark (`activity`, `cost-center`), once.
//! - *Activity* and *Cost center* leave the catalogue of built-in fields (ADR-0035): the levels
//!   roles and projects set for them, plain and for one issue type, and their places in layouts
//!   are deleted, because the program no longer reads the two words there. A rule of a workflow
//!   never held them: a rule may name only the description, the people an issue is assigned to
//!   and own fields (`rule_may_name`, since workflows exist).
//!
//! Nothing is written into the tables: the starter fields are made by the use case of the first
//! start, which runs at every start, so an upgraded server gets them too. No column refers to a
//! person (a removed person leaves a tombstone, PRIV-04).

use sea_query::{
    ColumnDef, DynIden, Expr, ExprTrait as _, ForeignKey, Index, IntoIden as _, Query, Table,
};

use super::{Step, id_column};
use crate::schema::{
    FieldRow, FieldValueRow, IssueRow, LayoutFieldRoleRow, LayoutFieldRow, LayoutOffFieldRow,
    ProjectFieldLevelRow, ProjectTypeFieldLevelRow, RoleFieldLevelRow, RoleTypeFieldLevelRow,
    StarterFieldRow, TimeEntryRow, TimeEntryValueRow, TimerRow,
};

/// The keys of the two built-in fields that left the catalogue, as a database of migration 30
/// holds them. Written out here, not read from the program, so the migration stays what it was
/// however the catalogue changes later.
pub(super) const RETIRED_FIELDS: [&str; 2] = ["activity", "cost-center"];

pub(super) fn steps() -> Vec<Step> {
    let mut steps = vec![timers()];
    steps.extend(entries());
    steps.extend(entry_values());
    steps.push(starter_fields());
    steps.extend(retired_fields());
    steps
}

/// Deletes the levels and the places in layouts of the fields that left the catalogue.
fn retired_fields() -> Vec<Step> {
    let tables: [(DynIden, DynIden); 7] = [
        (
            RoleFieldLevelRow::Table.into_iden(),
            RoleFieldLevelRow::Field.into_iden(),
        ),
        (
            ProjectFieldLevelRow::Table.into_iden(),
            ProjectFieldLevelRow::Field.into_iden(),
        ),
        (
            RoleTypeFieldLevelRow::Table.into_iden(),
            RoleTypeFieldLevelRow::Field.into_iden(),
        ),
        (
            ProjectTypeFieldLevelRow::Table.into_iden(),
            ProjectTypeFieldLevelRow::Field.into_iden(),
        ),
        // The roles of a field come before the field, which they belong to.
        (
            LayoutFieldRoleRow::Table.into_iden(),
            LayoutFieldRoleRow::Field.into_iden(),
        ),
        (
            LayoutOffFieldRow::Table.into_iden(),
            LayoutOffFieldRow::Field.into_iden(),
        ),
        (
            LayoutFieldRow::Table.into_iden(),
            LayoutFieldRow::Field.into_iden(),
        ),
    ];
    tables
        .into_iter()
        .map(|(table, field)| {
            Step::Delete(
                Query::delete()
                    .from_table(table)
                    .and_where(Expr::col(field).is_in(RETIRED_FIELDS))
                    .to_owned(),
            )
        })
        .collect()
}

fn timers() -> Step {
    Step::Table(
        Table::create()
            .table(TimerRow::Table)
            .col(id_column(TimerRow::Person).primary_key())
            .col(ColumnDef::new(TimerRow::Issue).blob())
            .col(ColumnDef::new(TimerRow::StartedAt).big_integer())
            .col(ColumnDef::new(TimerRow::WarnedAt).big_integer())
            .col(ColumnDef::new(TimerRow::ChangedAt).big_integer().not_null())
            .col(ColumnDef::new(TimerRow::Version).big_integer().not_null())
            .foreign_key(
                ForeignKey::create()
                    .from(TimerRow::Table, TimerRow::Issue)
                    .to(IssueRow::Table, IssueRow::Id),
            )
            .to_owned(),
    )
}

fn entries() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(TimeEntryRow::Table)
                .col(id_column(TimeEntryRow::Id).primary_key())
                .col(id_column(TimeEntryRow::Person))
                .col(id_column(TimeEntryRow::Issue))
                .col(ColumnDef::new(TimeEntryRow::Day).big_integer().not_null())
                .col(
                    ColumnDef::new(TimeEntryRow::Minutes)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(TimeEntryRow::Note).text().not_null())
                .col(ColumnDef::new(TimeEntryRow::Billable).boolean().not_null())
                .col(ColumnDef::new(TimeEntryRow::StartedAt).big_integer())
                .col(
                    ColumnDef::new(TimeEntryRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(TimeEntryRow::Version)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(TimeEntryRow::Table, TimeEntryRow::Issue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .to_owned(),
        ),
        // The entries of an issue, and the minutes spent on it.
        Step::Index(
            Index::create()
                .name("time_entry_by_issue")
                .table(TimeEntryRow::Table)
                .col(TimeEntryRow::Issue)
                .col(TimeEntryRow::Minutes)
                .to_owned(),
        ),
        // A person's entries by day: their week, and how many they booked on a day.
        Step::Index(
            Index::create()
                .name("time_entry_by_person")
                .table(TimeEntryRow::Table)
                .col(TimeEntryRow::Person)
                .col(TimeEntryRow::Day)
                .col(TimeEntryRow::Id)
                .to_owned(),
        ),
    ]
}

/// The values of the own fields of time entries: the columns of `field_value` (migration 9), with
/// the marks of time at work.
fn entry_values() -> Vec<Step> {
    vec![
        Step::Table(super::m0009_fields::values_table(
            Table::create().table(TimeEntryValueRow::Table),
            TimeEntryValueRow::Table.into_iden(),
        )),
        // The entries that hold a choice: combining two choices moves them.
        Step::Index(
            Index::create()
                .name("time_entry_value_by_ref")
                .table(TimeEntryValueRow::Table)
                .col(FieldValueRow::Field)
                .col(FieldValueRow::ValueRef)
                .to_owned(),
        ),
    ]
}

fn starter_fields() -> Step {
    Step::Table(
        Table::create()
            .table(StarterFieldRow::Table)
            .col(
                ColumnDef::new(StarterFieldRow::Starter)
                    .text()
                    .not_null()
                    .primary_key(),
            )
            .col(id_column(StarterFieldRow::Field).unique_key())
            .foreign_key(
                ForeignKey::create()
                    .from(StarterFieldRow::Table, StarterFieldRow::Field)
                    .to(FieldRow::Table, FieldRow::Id),
            )
            .to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use bb_app::layouts::LayoutStore as _;
    use bb_core::fields::{Field, FieldKey};
    use bb_core::id::{LayoutId, RoleId, SequentialIds};
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, Query};

    use super::super::Database;
    use super::*;
    use crate::auth::AuthStorage;
    use crate::db::StoredId as _;
    use crate::policy::level_rules_of_roles_in;
    use crate::schema::{LayoutRow, LayoutSectionRow, RoleRow};
    use crate::test_support::at;

    /// Inserts one row of `values` into `table`, as a database of migration 30 holds it.
    async fn insert<const N: usize>(
        database: &Database,
        table: impl sea_query::IntoIden,
        columns: [DynIden; N],
        values: [Expr; N],
    ) -> TestResult {
        let mut row = Query::insert();
        row.into_table(table).columns(columns).values(values)?;
        database.execute(&row).await?;
        Ok(())
    }

    /// A role of the old demo with its levels, as a database of migration 30 holds them.
    async fn old_role(database: &Database, role: RoleId) -> TestResult {
        insert(
            database,
            RoleRow::Table,
            [
                RoleRow::Id.into_iden(),
                RoleRow::Name.into_iden(),
                RoleRow::Position.into_iden(),
                RoleRow::TwoStep.into_iden(),
                RoleRow::Version.into_iden(),
            ],
            [
                Expr::val(role.stored()),
                Expr::val("Old demo role"),
                Expr::val(0_i64),
                Expr::val(false),
                Expr::val(1_i64),
            ],
        )
        .await?;
        // The old demo hid Cost center; a reset role held a level for every field.
        for (field, level) in [
            ("cost-center", "hidden"),
            ("activity", "can-see"),
            ("due-date", "hidden"),
        ] {
            insert(
                database,
                RoleFieldLevelRow::Table,
                [
                    RoleFieldLevelRow::Role.into_iden(),
                    RoleFieldLevelRow::Field.into_iden(),
                    RoleFieldLevelRow::Level.into_iden(),
                ],
                [Expr::val(role.stored()), Expr::val(field), Expr::val(level)],
            )
            .await?;
        }
        Ok(())
    }

    /// A layout of the issue page that places Cost center and Due date for the role and took
    /// Activity off its form.
    async fn old_layout(database: &Database, layout: LayoutId, role: RoleId) -> TestResult {
        insert(
            database,
            LayoutRow::Table,
            [
                LayoutRow::Id.into_iden(),
                LayoutRow::Kind.into_iden(),
                LayoutRow::Version.into_iden(),
            ],
            [
                Expr::val(layout.stored()),
                Expr::val("issue-page"),
                Expr::val(1_i64),
            ],
        )
        .await?;
        insert(
            database,
            LayoutSectionRow::Table,
            [
                LayoutSectionRow::Layout.into_iden(),
                LayoutSectionRow::Position.into_iden(),
                LayoutSectionRow::Tab.into_iden(),
                LayoutSectionRow::Title.into_iden(),
            ],
            [
                Expr::val(layout.stored()),
                Expr::val(0_i64),
                Expr::val("Details"),
                Expr::val(""),
            ],
        )
        .await?;
        for (position, field) in [(0_i64, "cost-center"), (1, "due-date")] {
            insert(
                database,
                LayoutFieldRow::Table,
                [
                    LayoutFieldRow::Layout.into_iden(),
                    LayoutFieldRow::Field.into_iden(),
                    LayoutFieldRow::Section.into_iden(),
                    LayoutFieldRow::Position.into_iden(),
                ],
                [
                    Expr::val(layout.stored()),
                    Expr::val(field),
                    Expr::val(0_i64),
                    Expr::val(position),
                ],
            )
            .await?;
            insert(
                database,
                LayoutFieldRoleRow::Table,
                [
                    LayoutFieldRoleRow::Layout.into_iden(),
                    LayoutFieldRoleRow::Field.into_iden(),
                    LayoutFieldRoleRow::Role.into_iden(),
                ],
                [
                    Expr::val(layout.stored()),
                    Expr::val(field),
                    Expr::val(role.stored()),
                ],
            )
            .await?;
        }
        insert(
            database,
            LayoutOffFieldRow::Table,
            [
                LayoutOffFieldRow::Layout.into_iden(),
                LayoutOffFieldRow::Field.into_iden(),
            ],
            [Expr::val(layout.stored()), Expr::val("activity")],
        )
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn time_13_migration_31_takes_the_retired_fields_out_of_levels_and_layouts() -> TestResult
    {
        each_database("migration_31_retired_fields", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(30, at(0)).await?;
            let mut ids = SequentialIds::default();
            let role = RoleId::generate(&mut ids)?;
            let layout = LayoutId::generate(&mut ids)?;
            old_role(&database, role).await?;
            old_layout(&database, layout, role).await?;

            assert_eq!(database.migrate_until(31, at(1)).await?, [31]);

            // The rights of the role are read and judged again: only the kept field is left.
            let mut read = database.begin_read().await?;
            let rules = level_rules_of_roles_in(&mut read, vec![role]).await?;
            let fields: Vec<FieldKey> = rules.fields.iter().map(|rule| rule.subject).collect();
            assert_eq!(fields, [FieldKey::BuiltIn(Field::DueDate)]);
            let kept = AuthStorage::stored_layout_in(&mut read, layout).await?;
            let kept = kept.ok_or("the layout is gone")?;
            let placed: Vec<FieldKey> = kept
                .content
                .sections
                .iter()
                .flat_map(|section| section.fields.iter().map(|placed| placed.field))
                .collect();
            assert_eq!(placed, [FieldKey::BuiltIn(Field::DueDate)]);
            assert!(kept.content.off.is_empty());
            Ok(())
        })
        .await
    }
}
