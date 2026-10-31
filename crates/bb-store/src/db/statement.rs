//! The query a statement of the store is built from, ready to be built for a dialect.

use sea_query::{
    DeleteStatement, InsertStatement, PostgresQueryBuilder, SelectStatement, SqliteQueryBuilder,
    UpdateStatement, WithQuery,
};
use sea_query_sqlx::{SqlxBinder as _, SqlxValues};

use super::Dialect;

/// A query built with sea-query, ready to be built for a dialect.
#[derive(Clone, Copy)]
pub(crate) enum Statement<'a> {
    Select(&'a SelectStatement),
    Insert(&'a InsertStatement),
    Update(&'a UpdateStatement),
    Delete(&'a DeleteStatement),
    With(&'a WithQuery),
}

impl Statement<'_> {
    pub(super) fn build(&self, dialect: Dialect) -> (String, SqlxValues) {
        match (self, dialect) {
            (Self::Select(query), Dialect::Sqlite) => query.build_sqlx(SqliteQueryBuilder),
            (Self::Select(query), Dialect::Postgres) => query.build_sqlx(PostgresQueryBuilder),
            (Self::Insert(query), Dialect::Sqlite) => query.build_sqlx(SqliteQueryBuilder),
            (Self::Insert(query), Dialect::Postgres) => query.build_sqlx(PostgresQueryBuilder),
            (Self::Update(query), Dialect::Sqlite) => query.build_sqlx(SqliteQueryBuilder),
            (Self::Update(query), Dialect::Postgres) => query.build_sqlx(PostgresQueryBuilder),
            (Self::Delete(query), Dialect::Sqlite) => query.build_sqlx(SqliteQueryBuilder),
            (Self::Delete(query), Dialect::Postgres) => query.build_sqlx(PostgresQueryBuilder),
            (Self::With(query), Dialect::Sqlite) => query.build_sqlx(SqliteQueryBuilder),
            (Self::With(query), Dialect::Postgres) => query.build_sqlx(PostgresQueryBuilder),
        }
    }
}

macro_rules! statement_from {
    ($($query:ident => $variant:ident),+) => {
        $(impl<'a> From<&'a $query> for Statement<'a> {
            fn from(query: &'a $query) -> Self {
                Self::$variant(query)
            }
        })+
    };
}

statement_from!(
    SelectStatement => Select,
    InsertStatement => Insert,
    UpdateStatement => Update,
    DeleteStatement => Delete,
    WithQuery => With
);
