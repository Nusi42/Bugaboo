//! Migration 49: an issue gets *Start date* and *Due date*, and a kind of relation gets a date
//! effect (PLAN-01, PLAN-02, ADR-0043, decisions 1 and 4).
//!
//! Both dates are calendar days kept as the days since 1970-01-01 (the way a *Date* field keeps a
//! day) and empty until somebody gives them; every issue made before keeps what it had and has
//! neither. A kind of relation holds the word of its date effect, `none` until a kind is given one.
//! The starting kind *blocks* gets `must-finish-first`: it is the one at the first place whose
//! name is the starting word. The names of kinds could be changed since the first start and kinds
//! had no mark before this migration, so a kind that was renamed is no longer found, and an
//! organization gives it the effect itself. The word and the place are written out here, not read
//! from the program, so the migration stays what it was however the starting kinds change later.
//! One `ALTER TABLE … ADD COLUMN` for each column, because SQLite adds one column at a time. No
//! index yet: the benchmark of the plan decides whether sorting by the dates needs one.

use sea_query::{ColumnDef, Expr, ExprTrait as _, Func, Query, Table};

use super::Step;
use crate::schema::{IssueRow, RelationKindRow};

/// The word of the date effect *Must finish first*, as it is stored.
const MUST_FINISH_FIRST: &str = "must-finish-first";

/// The word of no date effect, as it is stored: the starting value of every kind.
const NO_EFFECT: &str = "none";

/// The name of the starting kind *blocks*, in lower case, and its place among the starting kinds.
const BLOCKS: &str = "blocks";
const BLOCKS_PLACE: i64 = 0;

pub(super) fn steps() -> Vec<Step> {
    let day = |column: IssueRow| {
        Step::Alter(
            Table::alter()
                .table(IssueRow::Table)
                .add_column(ColumnDef::new(column).big_integer())
                .to_owned(),
        )
    };
    let mut effect = ColumnDef::new(RelationKindRow::DateEffect);
    effect.text().not_null().default(NO_EFFECT);
    vec![
        day(IssueRow::StartDate),
        day(IssueRow::DueDate),
        Step::Alter(
            Table::alter()
                .table(RelationKindRow::Table)
                .add_column(effect)
                .to_owned(),
        ),
        Step::Update(
            Query::update()
                .table(RelationKindRow::Table)
                .value(RelationKindRow::DateEffect, MUST_FINISH_FIRST)
                .and_where(Expr::col(RelationKindRow::Position).eq(BLOCKS_PLACE))
                .and_where(Func::lower(Expr::col(RelationKindRow::Forward)).eq(BLOCKS))
                .to_owned(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use bb_core::id::{IssueId, IssueTypeId, ProjectId, RelationKindId};
    use bb_testing::{TestResult, each_database};
    use jiff::Timestamp;
    use sea_query::{Expr, ExprTrait as _, IntoIden as _};

    use super::super::old_rows::insert;
    use super::super::{Database, latest_version};
    use crate::db::StoredId as _;
    use crate::issues::{issue_select, read_issue};
    use crate::schema::{IssueRow, RelationKindRow};
    use crate::test_support::{at, insert_old_issue, insert_old_issue_type, insert_old_project};

    /// A kind of relation as migration 10 made it: two names and a place.
    async fn old_kind(
        database: &Database,
        id: RelationKindId,
        names: (&str, &str),
        place: i64,
    ) -> TestResult {
        insert(
            database,
            RelationKindRow::Table,
            [
                RelationKindRow::Id.into_iden(),
                RelationKindRow::Forward.into_iden(),
                RelationKindRow::Backward.into_iden(),
                RelationKindRow::Position.into_iden(),
                RelationKindRow::Version.into_iden(),
            ],
            [
                Expr::val(id.stored()),
                Expr::val(names.0.to_owned()),
                Expr::val(names.1.to_owned()),
                Expr::val(place),
                Expr::val(1_i64),
            ],
        )
        .await
    }

    async fn effect_of(
        database: &Database,
        id: RelationKindId,
    ) -> Result<String, bb_testing::Failure> {
        let mut read = sea_query::Query::select();
        read.column(RelationKindRow::DateEffect)
            .from(RelationKindRow::Table)
            .and_where(Expr::col(RelationKindRow::Id).eq(id.stored()));
        let row = database.fetch_optional(&read).await?;
        Ok(row
            .ok_or("the kind is gone")?
            .get::<String>(RelationKindRow::DateEffect)?)
    }

    #[tokio::test]
    async fn plan_02_blocks_has_the_date_effect_after_migration_49() -> TestResult {
        each_database("migration_49_dates_and_date_effect", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(48, at(0)).await?;
            let issue_type = IssueTypeId::from_parts(Timestamp::UNIX_EPOCH, [2; 10])?;
            insert_old_issue_type(&database, issue_type, "Task").await?;
            let project = ProjectId::from_parts(Timestamp::UNIX_EPOCH, [3; 10])?;
            insert_old_project(&database, project.stored()).await?;
            let issue = IssueId::from_parts(Timestamp::UNIX_EPOCH, [4; 10])?;
            insert_old_issue(
                &database,
                (project.stored(), issue.stored()),
                issue_type,
                "1",
            )
            .await?;
            let kinds: Vec<RelationKindId> = (1..=4_u8)
                .map(|byte| RelationKindId::from_parts(Timestamp::UNIX_EPOCH, [byte; 10]))
                .collect::<Result<_, _>>()?;
            // The four starting kinds, as the first start made them, and a fifth that someone made.
            for (kind, (names, place)) in kinds.iter().zip([
                (("blocks", "is blocked by"), 0),
                (("duplicate of", "has duplicate"), 1),
                (("causes", "is caused by"), 2),
                (("related to", "related to"), 3),
            ]) {
                old_kind(&database, *kind, names, place).await?;
            }
            let applied = database.migrate_until(49, at(1)).await?;
            assert_eq!(applied.last(), Some(&49));
            assert!(latest_version() >= 49);

            // Only blocks has the effect; the other three have none.
            let mut effects = Vec::new();
            for kind in &kinds {
                effects.push(effect_of(&database, *kind).await?);
            }
            assert_eq!(
                effects,
                ["must-finish-first", "none", "none", "none"].map(str::to_owned)
            );
            // The issue reads as it was, with no dates.
            let mut read = issue_select();
            read.and_where(Expr::col(IssueRow::Id).eq(issue.stored()));
            let row = database.fetch_optional(&read).await?;
            let kept = read_issue(&row.ok_or("the issue is gone")?)?;
            assert_eq!((kept.start_date, kept.due_date), (None, None));
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn plan_02_a_kind_that_is_not_the_first_called_blocks_gets_no_effect() -> TestResult {
        each_database("migration_49_other_blocks", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(48, at(0)).await?;
            let first = RelationKindId::from_parts(Timestamp::UNIX_EPOCH, [1; 10])?;
            let renamed = RelationKindId::from_parts(Timestamp::UNIX_EPOCH, [2; 10])?;
            // The first kind was renamed, and another one at a later place got the old word: it
            // is no starting kind, so it is not given the effect.
            old_kind(&database, first, ("Waits for", "Is waited for by"), 0).await?;
            old_kind(&database, renamed, ("Blocks", "Is blocked by"), 5).await?;
            database.migrate_until(49, at(1)).await?;
            assert_eq!(effect_of(&database, first).await?, "none");
            assert_eq!(effect_of(&database, renamed).await?, "none");
            Ok(())
        })
        .await
    }
}
