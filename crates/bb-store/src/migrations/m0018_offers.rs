//! Migration 18: where issue types and groups of labels are offered (ISS-02, TAG-02, ADR-0027
//! amendment).
//!
//! A type and a group get the project they were created in (none for the ones that are there, which
//! belong to the whole organization), a reference to that project. Where one is offered is a row per
//! place and subject. Nothing is written into the table: with no row, a type or group of the
//! organization is offered everywhere, so an upgraded database offers what it offered before.
//!
//! The name of a group of labels is no longer unique in the whole organization: it is unique among
//! the groups that can meet in one project, which the program judges inside the change that writes
//! it (`bb_core::offer::can_meet`), so the unique key of migration 11 goes.
//!
//! The rows of `offer` have **no reference** for `subject`: it holds the ID of an issue type or of
//! a group of labels, which `kind` tells apart, and one column cannot refer to two tables. Nothing
//! deletes an issue type or a group of labels (they are retired), so no row is left without its
//! subject; a change that deletes them must delete their rows in the same change.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

/// The reference of a column that holds the project a subject was created in.
const PROJECT_REFERENCE: &str = "REFERENCES \"project\" (\"id\")";

use super::{Step, id_column};
use crate::schema::{IssueTypeRow, LabelGroupRow, OfferRow, ProjectRow};

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Alter(
            Table::alter()
                .table(IssueTypeRow::Table)
                .add_column(
                    ColumnDef::new(IssueTypeRow::Project)
                        .blob()
                        .extra(PROJECT_REFERENCE),
                )
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(LabelGroupRow::Table)
                .add_column(
                    ColumnDef::new(LabelGroupRow::Project)
                        .blob()
                        .extra(PROJECT_REFERENCE),
                )
                .to_owned(),
        ),
        Step::DropIndex(
            Index::drop()
                .name("label_group_by_key")
                .table(LabelGroupRow::Table)
                .to_owned(),
        ),
        // A place is a project that can be none, so the one row for a place and subject is kept by
        // the program, inside the change that writes it, not by a unique key.
        Step::Table(
            Table::create()
                .table(OfferRow::Table)
                .col(ColumnDef::new(OfferRow::Kind).text().not_null())
                .col(id_column(OfferRow::Subject))
                .col(ColumnDef::new(OfferRow::Project).blob())
                .col(ColumnDef::new(OfferRow::Offered).boolean().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .from(OfferRow::Table, OfferRow::Project)
                        .to(ProjectRow::Table, ProjectRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("offer_by_subject")
                .table(OfferRow::Table)
                .col(OfferRow::Kind)
                .col(OfferRow::Subject)
                .to_owned(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use bb_core::id::{IssueId, IssueTypeId, LabelGroupId, ProjectId};
    use bb_core::many_changes::{ManyChangeSkip, SkipReason};
    use bb_core::offer::{OfferSubject, is_offered};
    use bb_testing::{TestResult, each_database};
    use jiff::Timestamp;
    use sea_query::{Expr, Query};

    use super::super::Database;
    use crate::db::StoredId as _;
    use crate::schema::{IssueTypeRow, LabelGroupRow, ManyChangeRunRow, OfferRow, ProjectRow};
    use crate::test_support::{at, insert_old_issue_type};

    // The rows of the "M4 database before this package" are written by hand, up to migration 17: a
    // type and a group, the way the program wrote them (no project; the columns did not exist).
    #[tokio::test]
    async fn iss_02_an_upgraded_database_offers_every_type_everywhere() -> TestResult {
        each_database("iss_02_upgrade_offers", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(17, at(0)).await?;
            let issue_type = IssueTypeId::from_parts(Timestamp::UNIX_EPOCH, [2; 10])?;
            let group = LabelGroupId::from_parts(Timestamp::UNIX_EPOCH, [3; 10])?;
            insert_old_issue_type(&database, issue_type, "Task").await?;
            let mut insert = Query::insert();
            insert
                .into_table(LabelGroupRow::Table)
                .columns([
                    LabelGroupRow::Id,
                    LabelGroupRow::Name,
                    LabelGroupRow::NameKey,
                    LabelGroupRow::PickOne,
                    LabelGroupRow::Position,
                    LabelGroupRow::Version,
                ])
                .values([
                    Expr::val(group.stored()),
                    Expr::val("Area"),
                    Expr::val("area"),
                    Expr::val(false),
                    Expr::val(0_i64),
                    Expr::val(1_i64),
                ])?;
            database.execute(&insert).await?;
            assert_eq!(database.migrate_until(18, at(1)).await?, [18]);
            // No row says anything: the type and the group belong to the whole organization and are
            // offered everywhere, in every project, as before.
            let mut offers = Query::select();
            offers.column(OfferRow::Kind).from(OfferRow::Table);
            assert!(database.fetch_all(&offers).await?.is_empty());
            let stored = database
                .stored_issue_type(issue_type)
                .await?
                .ok_or("the type is gone")?;
            assert_eq!(stored.project, None);
            assert_eq!(stored.version, 1);
            assert!(is_offered(
                OfferSubject::IssueType(issue_type),
                None,
                &[],
                &[]
            ));
            assert!(is_offered(OfferSubject::LabelGroup(group), None, &[], &[]));
            Ok(())
        })
        .await
    }

    async fn insert_project(database: &Database, id: ProjectId, short: &str) -> TestResult {
        let mut insert = Query::insert();
        insert
            .into_table(ProjectRow::Table)
            .columns([
                ProjectRow::Id,
                ProjectRow::ShortName,
                ProjectRow::Name,
                ProjectRow::Version,
            ])
            .values([
                Expr::val(id.stored()),
                Expr::val(short),
                Expr::val(short),
                Expr::val(1_i64),
            ])?;
        database.execute(&insert).await?;
        Ok(())
    }

    fn group_row(id: LabelGroupId, project: Option<ProjectId>) -> sea_query::InsertStatement {
        let mut insert = Query::insert();
        insert
            .into_table(LabelGroupRow::Table)
            .columns([
                LabelGroupRow::Id,
                LabelGroupRow::Name,
                LabelGroupRow::NameKey,
                LabelGroupRow::PickOne,
                LabelGroupRow::Project,
                LabelGroupRow::Position,
                LabelGroupRow::Version,
            ])
            .values_panic([
                Expr::val(id.stored()),
                Expr::val("Area"),
                Expr::val("area"),
                Expr::val(false),
                Expr::val(project.map(crate::db::StoredId::stored)),
                Expr::val(0_i64),
                Expr::val(1_i64),
            ]);
        insert
    }

    // A type or a group refers to the project it was created in, and two groups of two projects may
    // have one name (it is unique among the groups that can meet, which the program judges).
    #[tokio::test]
    async fn iss_02_a_subject_refers_to_its_project_and_a_group_name_may_repeat() -> TestResult {
        each_database("iss_02_project_reference", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(18, at(0)).await?;
            let (acme, other) = (
                ProjectId::from_parts(Timestamp::UNIX_EPOCH, [4; 10])?,
                ProjectId::from_parts(Timestamp::UNIX_EPOCH, [5; 10])?,
            );
            insert_project(&database, acme, "ACME").await?;
            insert_project(&database, other, "OTHER").await?;
            let group = |number: u8| LabelGroupId::from_parts(Timestamp::UNIX_EPOCH, [number; 10]);
            database.execute(&group_row(group(6)?, Some(acme))).await?;
            database.execute(&group_row(group(7)?, Some(other))).await?;
            // A project that is not there is refused, for a group and for a type.
            let nowhere = ProjectId::from_parts(Timestamp::UNIX_EPOCH, [9; 10])?;
            assert!(
                database
                    .execute(&group_row(group(8)?, Some(nowhere)))
                    .await
                    .is_err()
            );
            let mut insert = Query::insert();
            insert
                .into_table(IssueTypeRow::Table)
                .columns([
                    IssueTypeRow::Id,
                    IssueTypeRow::Name,
                    IssueTypeRow::Icon,
                    IssueTypeRow::Project,
                    IssueTypeRow::Position,
                    IssueTypeRow::Version,
                ])
                .values([
                    Expr::val(IssueTypeId::from_parts(Timestamp::UNIX_EPOCH, [3; 10])?.stored()),
                    Expr::val("Question"),
                    Expr::val("circle-help"),
                    Expr::val(nowhere.stored()),
                    Expr::val(0_i64),
                    Expr::val(1_i64),
                ])?;
            assert!(database.execute(&insert).await.is_err());
            Ok(())
        })
        .await
    }

    // A run of a change to many issues that skipped an issue for a type the other project did not
    // offer, stored before this migration, is read after it.
    #[tokio::test]
    async fn iss_09_a_stored_run_that_skipped_for_a_missing_type_is_still_read() -> TestResult {
        each_database("iss_09_old_skip_reason", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(17, at(0)).await?;
            let issue = IssueId::from_parts(Timestamp::UNIX_EPOCH, [6; 10])?;
            let skipped = format!(r#"[{{"issue":"{issue}","reason":"type-missing"}}]"#);
            let mut run = Query::insert();
            run.into_table(ManyChangeRunRow::Table)
                .columns([
                    ManyChangeRunRow::Id,
                    ManyChangeRunRow::Person,
                    ManyChangeRunRow::Action,
                    ManyChangeRunRow::Items,
                    ManyChangeRunRow::Status,
                    ManyChangeRunRow::Done,
                    ManyChangeRunRow::Changed,
                    ManyChangeRunRow::Skipped,
                    ManyChangeRunRow::CreatedAt,
                ])
                .values([
                    Expr::val(vec![1_u8; 16]),
                    Expr::val(vec![2_u8; 16]),
                    Expr::val("{}"),
                    Expr::val("[]"),
                    Expr::val("done"),
                    Expr::val(1_i64),
                    Expr::val(0_i64),
                    Expr::val(skipped),
                    Expr::val(5_i64),
                ])?;
            database.execute(&run).await?;
            assert_eq!(database.migrate_until(18, at(1)).await?, [18]);
            let mut select = Query::select();
            select
                .column(ManyChangeRunRow::Skipped)
                .from(ManyChangeRunRow::Table);
            let rows = database.fetch_all(&select).await?;
            let stored: Vec<ManyChangeSkip> =
                serde_json::from_str(&rows[0].get::<String>(ManyChangeRunRow::Skipped)?)?;
            assert_eq!(stored.len(), 1);
            assert_eq!(stored[0].reason, SkipReason::TypeMissing);
            Ok(())
        })
        .await
    }
}
