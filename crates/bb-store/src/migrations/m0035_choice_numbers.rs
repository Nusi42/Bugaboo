//! Migration 35: a choice of an own field may have a number (TIME-13, CF-08, M6-F41).
//!
//! The number is up to nine digits kept as typed (`0410` stays `0410`), empty until an administrator gives one; every choice made before
//! keeps its name and its place and has none. A number is unique within its field: the index
//! allows any count of empty numbers, because an empty value never equals another, on both
//! databases.

use sea_query::{ColumnDef, Index, Table};

use super::Step;
use crate::schema::FieldChoiceRow;

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Alter(
            Table::alter()
                .table(FieldChoiceRow::Table)
                .add_column(ColumnDef::new(FieldChoiceRow::Number).text())
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("field_option_number_by_field")
                .table(FieldChoiceRow::Table)
                .col(FieldChoiceRow::Field)
                .col(FieldChoiceRow::Number)
                .unique()
                .to_owned(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use bb_core::id::{ChoiceId, FieldId};
    use bb_testing::{TestResult, each_database};
    use jiff::Timestamp;
    use sea_query::{Expr, ExprTrait as _, Query};

    use super::super::{Database, latest_version};
    use crate::db::StoredId as _;
    use crate::schema::{FieldChoiceRow, FieldRow};
    use crate::test_support::at;

    async fn insert_old_field(database: &Database, field: FieldId) -> TestResult {
        let mut insert = Query::insert();
        insert
            .into_table(FieldRow::Table)
            .columns([
                FieldRow::Id,
                FieldRow::Name,
                FieldRow::Help,
                FieldRow::Kind,
                FieldRow::AppliesTo,
                FieldRow::Rules,
                FieldRow::DefaultValue,
                FieldRow::Look,
                FieldRow::NeverToAssistant,
                FieldRow::Position,
                FieldRow::Version,
            ])
            .values([
                Expr::val(field.stored()),
                Expr::val("Cost center"),
                Expr::val(""),
                Expr::val("pick-one"),
                Expr::val("time-entries"),
                Expr::val("{}"),
                Expr::val(""),
                Expr::val("badge"),
                Expr::val(false),
                Expr::val(0_i64),
                Expr::val(1_i64),
            ])?;
        database.execute(&insert).await?;
        Ok(())
    }

    async fn insert_old_choice(
        database: &Database,
        field: FieldId,
        choice: ChoiceId,
        (name, position): (&str, i64),
    ) -> TestResult {
        let mut insert = Query::insert();
        insert
            .into_table(FieldChoiceRow::Table)
            .columns([
                FieldChoiceRow::Id,
                FieldChoiceRow::Field,
                FieldChoiceRow::Name,
                FieldChoiceRow::Color,
                FieldChoiceRow::Position,
            ])
            .values([
                Expr::val(choice.stored()),
                Expr::val(field.stored()),
                Expr::val(name),
                Expr::val("cat1"),
                Expr::val(position),
            ])?;
        database.execute(&insert).await?;
        Ok(())
    }

    async fn set_number(database: &Database, choice: ChoiceId, number: &str) -> TestResult {
        let mut update = Query::update();
        update
            .table(FieldChoiceRow::Table)
            .value(FieldChoiceRow::Number, number)
            .and_where(Expr::col(FieldChoiceRow::Id).eq(choice.stored()));
        database.execute(&update).await?;
        Ok(())
    }

    #[tokio::test]
    async fn time_13_migration_35_keeps_the_choices_and_allows_one_number_once_in_a_field()
    -> TestResult {
        each_database("migration_35_choice_numbers", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(34, at(0)).await?;
            let field = FieldId::from_parts(Timestamp::UNIX_EPOCH, [1; 10])?;
            let (first, second, third) = (
                ChoiceId::from_parts(Timestamp::UNIX_EPOCH, [2; 10])?,
                ChoiceId::from_parts(Timestamp::UNIX_EPOCH, [3; 10])?,
                ChoiceId::from_parts(Timestamp::UNIX_EPOCH, [4; 10])?,
            );
            insert_old_field(&database, field).await?;
            insert_old_choice(&database, field, first, ("Research", 0)).await?;
            insert_old_choice(&database, field, second, ("Product development", 1)).await?;
            let applied = database.migrate_until(35, at(1)).await?;
            assert_eq!(applied.last(), Some(&35));
            assert!(latest_version() >= 35);

            // The choices read as they were, in their order, with no number.
            let definitions = crate::fields::definitions_in(&mut &database).await?;
            let names: Vec<_> = definitions[0]
                .choices
                .iter()
                .map(|choice| {
                    (
                        choice.name.as_str(),
                        choice.number.as_deref(),
                        choice.position,
                    )
                })
                .collect();
            assert_eq!(
                names,
                [("Research", None, 0), ("Product development", None, 1)]
            );

            // A number is told once within the field; choices without one do not clash.
            set_number(&database, first, "4100").await?;
            assert!(set_number(&database, second, "4100").await.is_err());
            insert_old_choice(&database, field, third, ("Sales", 2)).await?;
            // A number is text: 0410 and 410 are two numbers, and 0410 is kept as typed.
            set_number(&database, third, "0410").await?;
            set_number(&database, second, "410").await?;
            assert!(set_number(&database, second, "0410").await.is_err());
            Ok(())
        })
        .await
    }
}
