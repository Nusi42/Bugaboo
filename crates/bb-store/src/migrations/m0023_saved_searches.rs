//! Migration 23: saved searches (QRY-06, ADR-0034).
//!
//! A saved search, its columns, the groups and projects it is shared with, and the saved search a
//! project shows as its list. Nothing is written into the tables: a database that has none of them
//! simply has no saved search. The maker has no reference to the person (a removed person leaves
//! a tombstone, PRIV-04); the rows of `saved_search_share` have none for their target, which is
//! the ID of a group or of a project, told apart by `kind`.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{
    ProjectDefaultListRow, ProjectRow, SavedSearchRow, SearchColumnRow, SearchShareRow,
};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = searches();
    steps.extend(columns());
    steps.extend(shares());
    steps.extend(default_lists());
    steps
}

fn searches() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(SavedSearchRow::Table)
                .col(
                    ColumnDef::new(SavedSearchRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(SavedSearchRow::Name).text().not_null())
                .col(ColumnDef::new(SavedSearchRow::Text).text().not_null())
                .col(id_column(SavedSearchRow::Maker))
                .col(ColumnDef::new(SavedSearchRow::View).text().not_null())
                .col(ColumnDef::new(SavedSearchRow::Spacing).text().not_null())
                .col(ColumnDef::new(SavedSearchRow::GroupBy).text())
                .col(
                    ColumnDef::new(SavedSearchRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(SavedSearchRow::UpdatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(SavedSearchRow::DeletedAt).big_integer())
                .col(
                    ColumnDef::new(SavedSearchRow::Version)
                        .big_integer()
                        .not_null(),
                )
                .to_owned(),
        ),
        // The searches of one person, oldest first.
        Step::Index(
            Index::create()
                .name("saved_search_by_maker")
                .table(SavedSearchRow::Table)
                .col(SavedSearchRow::Maker)
                .col(SavedSearchRow::Id)
                .to_owned(),
        ),
    ]
}

fn columns() -> Vec<Step> {
    vec![Step::Table(
        Table::create()
            .table(SearchColumnRow::Table)
            .col(id_column(SearchColumnRow::Search))
            .col(
                ColumnDef::new(SearchColumnRow::Position)
                    .big_integer()
                    .not_null(),
            )
            .col(ColumnDef::new(SearchColumnRow::Field).text().not_null())
            .col(ColumnDef::new(SearchColumnRow::Width).big_integer())
            .primary_key(
                Index::create()
                    .col(SearchColumnRow::Search)
                    .col(SearchColumnRow::Position),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(SearchColumnRow::Table, SearchColumnRow::Search)
                    .to(SavedSearchRow::Table, SavedSearchRow::Id),
            )
            .to_owned(),
    )]
}

fn shares() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(SearchShareRow::Table)
                .col(id_column(SearchShareRow::Search))
                .col(ColumnDef::new(SearchShareRow::Kind).text().not_null())
                .col(id_column(SearchShareRow::Target))
                .col(
                    ColumnDef::new(SearchShareRow::SharedAt)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(SearchShareRow::Search)
                        .col(SearchShareRow::Kind)
                        .col(SearchShareRow::Target),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(SearchShareRow::Table, SearchShareRow::Search)
                        .to(SavedSearchRow::Table, SavedSearchRow::Id),
                )
                .to_owned(),
        ),
        // The searches shared with one group or project.
        Step::Index(
            Index::create()
                .name("saved_search_share_by_target")
                .table(SearchShareRow::Table)
                .col(SearchShareRow::Kind)
                .col(SearchShareRow::Target)
                .col(SearchShareRow::Search)
                .to_owned(),
        ),
    ]
}

fn default_lists() -> Vec<Step> {
    vec![Step::Table(
        Table::create()
            .table(ProjectDefaultListRow::Table)
            .col(
                ColumnDef::new(ProjectDefaultListRow::Project)
                    .blob()
                    .not_null()
                    .primary_key(),
            )
            .col(id_column(ProjectDefaultListRow::Search))
            .foreign_key(
                ForeignKey::create()
                    .from(ProjectDefaultListRow::Table, ProjectDefaultListRow::Project)
                    .to(ProjectRow::Table, ProjectRow::Id),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(ProjectDefaultListRow::Table, ProjectDefaultListRow::Search)
                    .to(SavedSearchRow::Table, SavedSearchRow::Id),
            )
            .to_owned(),
    )]
}

#[cfg(test)]
mod tests {
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, Query};

    use super::super::Database;
    use crate::schema::{SavedSearchRow, SearchColumnRow, SearchShareRow};
    use crate::test_support::at;

    fn search_row(id: u8) -> Result<sea_query::InsertStatement, sea_query::error::Error> {
        let mut insert = Query::insert();
        insert
            .into_table(SavedSearchRow::Table)
            .columns([
                SavedSearchRow::Id,
                SavedSearchRow::Name,
                SavedSearchRow::Text,
                SavedSearchRow::Maker,
                SavedSearchRow::View,
                SavedSearchRow::Spacing,
                SavedSearchRow::CreatedAt,
                SavedSearchRow::UpdatedAt,
                SavedSearchRow::Version,
            ])
            .values([
                Expr::val(vec![id; 16]),
                Expr::val("Open bugs"),
                Expr::val("status is open"),
                Expr::val(vec![9_u8; 16]),
                Expr::val("list"),
                Expr::val("comfortable"),
                Expr::val(1_i64),
                Expr::val(1_i64),
                Expr::val(1_i64),
            ])?;
        Ok(insert)
    }

    // The tables come with migration 23 on an M4 database, hold a search with its columns and
    // shares, refuse a column or a share of a search that is not there, and refuse the same share
    // twice.
    #[tokio::test]
    async fn qry_06_migration_23_makes_the_tables_of_saved_searches() -> TestResult {
        each_database("migration_23_saved_searches", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(21, at(0)).await?;
            assert_eq!(database.migrate(at(1)).await?, [22, 23]);
            database.execute(&search_row(1)?).await?;
            let column = |search: u8, position: i64| {
                let mut insert = Query::insert();
                insert
                    .into_table(SearchColumnRow::Table)
                    .columns([
                        SearchColumnRow::Search,
                        SearchColumnRow::Position,
                        SearchColumnRow::Field,
                    ])
                    .values_panic([
                        Expr::val(vec![search; 16]),
                        Expr::val(position),
                        Expr::val("title"),
                    ]);
                insert
            };
            database.execute(&column(1, 0)).await?;
            assert!(database.execute(&column(1, 0)).await.is_err());
            assert!(database.execute(&column(2, 0)).await.is_err());
            let share = |search: u8, target: u8| {
                let mut insert = Query::insert();
                insert
                    .into_table(SearchShareRow::Table)
                    .columns([
                        SearchShareRow::Search,
                        SearchShareRow::Kind,
                        SearchShareRow::Target,
                        SearchShareRow::SharedAt,
                    ])
                    .values_panic([
                        Expr::val(vec![search; 16]),
                        Expr::val("group"),
                        Expr::val(vec![target; 16]),
                        Expr::val(1_i64),
                    ]);
                insert
            };
            database.execute(&share(1, 7)).await?;
            assert!(database.execute(&share(1, 7)).await.is_err());
            assert!(database.execute(&share(2, 7)).await.is_err());
            Ok(())
        })
        .await
    }
}
