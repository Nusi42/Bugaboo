//! Migration 18: where issue types and groups of labels are offered (ISS-02, TAG-02, ADR-0027
//! amendment).
//!
//! A type and a group get the project they were made in (none for the ones that are there, which
//! belong to the whole organization). Where one is offered is a row per place and subject. Nothing
//! is written into the table: with no row, a type or group of the organization is offered
//! everywhere, so an upgraded database offers what it offered before.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{IssueTypeRow, LabelGroupRow, OfferRow, ProjectRow};

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Alter(
            Table::alter()
                .table(IssueTypeRow::Table)
                .add_column(ColumnDef::new(IssueTypeRow::Project).blob())
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(LabelGroupRow::Table)
                .add_column(ColumnDef::new(LabelGroupRow::Project).blob())
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
    use bb_core::id::{IssueTypeId, LabelGroupId};
    use bb_core::offer::{OfferSubject, is_offered};
    use bb_testing::{TestResult, each_database};
    use jiff::Timestamp;
    use sea_query::{Expr, Query};

    use super::super::Database;
    use crate::db::StoredId as _;
    use crate::schema::{LabelGroupRow, OfferRow};
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
}
