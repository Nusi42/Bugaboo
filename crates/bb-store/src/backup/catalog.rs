//! What PostgreSQL knows about its own tables, for the copy table by table (ADR-0046, decision 2):
//! the tables in the order they were made (which is the order of the migrations, so a table comes
//! after the tables it points to), the columns of each that hold data (a generated column is made
//! again by the database), the columns that point into the same table, and the columns that count
//! up by themselves.

use sea_query::{Alias, Expr, ExprTrait as _, Iden, JoinType, Order, Query};

use crate::db::Transaction;
use crate::error::StoreError;
use crate::schema::Migration;

use super::value::ColumnKind;

/// The names of what the catalog answers.
#[derive(Clone, Copy, Iden)]
enum Answer {
    Name,
    Kind,
    Start,
    Child,
    Parent,
}

/// A column of a table, with the kind of value it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Column {
    /// Its name.
    pub(crate) name: String,
    /// The kind of value.
    pub(crate) kind: ColumnKind,
    /// Whether it counts up by itself (its start value comes from a sequence).
    pub(crate) counts_up: bool,
}

fn catalog(table: &str) -> (Alias, Alias) {
    (Alias::new("pg_catalog"), Alias::new(table))
}

/// The tables of the schema in the order they were made, without the record of migrations (a
/// restore writes it by running the migrations).
pub(crate) async fn tables(transaction: &mut Transaction) -> Result<Vec<String>, StoreError> {
    let mut select = Query::select();
    select
        .expr_as(Expr::cust("c.relname::text"), Answer::Name)
        .from_as(catalog("pg_class"), Alias::new("c"))
        .and_where(Expr::cust("c.relkind = 'r'"))
        .and_where(Expr::cust(
            "c.relnamespace = current_schema()::regnamespace",
        ))
        .order_by((Alias::new("c"), Alias::new("oid")), Order::Asc);
    let record = Migration::Table.to_string();
    let mut names = Vec::new();
    for row in transaction.fetch_all(&select).await? {
        let name: String = row.get(Answer::Name)?;
        if name != record {
            names.push(name);
        }
    }
    Ok(names)
}

/// The columns of `table` that hold data, in their order.
pub(crate) async fn columns(
    transaction: &mut Transaction,
    table: &str,
) -> Result<Vec<Column>, StoreError> {
    let mut select = Query::select();
    select
        .expr_as(Expr::cust("column_name::text"), Answer::Name)
        .expr_as(Expr::cust("data_type::text"), Answer::Kind)
        .expr_as(
            Expr::cust(
                "pg_get_serial_sequence(quote_ident(table_name::text), column_name::text)                  IS NOT NULL",
            ),
            Answer::Start,
        )
        .from((Alias::new("information_schema"), Alias::new("columns")))
        .and_where(Expr::cust("table_schema = current_schema()"))
        .and_where(Expr::col(Alias::new("table_name")).eq(table))
        .and_where(Expr::col(Alias::new("is_generated")).eq("NEVER"))
        .order_by(Alias::new("ordinal_position"), Order::Asc);
    transaction
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| {
            let kind: String = row.get(Answer::Kind)?;
            let counts_up: bool = row.get(Answer::Start)?;
            Ok(Column {
                name: row.get(Answer::Name)?,
                kind: ColumnKind::from_type(&kind)
                    .ok_or(StoreError::Corrupt("a column a backup cannot copy"))?,
                counts_up,
            })
        })
        .collect()
}

/// The pairs (column, column it points to) of `table` that point into the same table, such as
/// the parent of a project: a restore writes the rows they point to first.
pub(crate) async fn self_references(
    transaction: &mut Transaction,
    table: &str,
) -> Result<Vec<(String, String)>, StoreError> {
    let mut select = Query::select();
    select
        .expr_as(Expr::cust("a.attname::text"), Answer::Child)
        .expr_as(Expr::cust("b.attname::text"), Answer::Parent)
        .from_as(catalog("pg_constraint"), Alias::new("c"))
        .join_as(
            JoinType::InnerJoin,
            catalog("pg_attribute"),
            Alias::new("a"),
            Expr::cust("a.attrelid = c.conrelid AND a.attnum = c.conkey[1]"),
        )
        .join_as(
            JoinType::InnerJoin,
            catalog("pg_attribute"),
            Alias::new("b"),
            Expr::cust("b.attrelid = c.confrelid AND b.attnum = c.confkey[1]"),
        )
        .and_where(Expr::cust("c.contype = 'f' AND c.conrelid = c.confrelid"))
        .and_where(Expr::cust_with_values(
            "c.conrelid = to_regclass(quote_ident($1))",
            [table],
        ));
    transaction
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| Ok((row.get(Answer::Child)?, row.get(Answer::Parent)?)))
        .collect()
}

/// Sets the sequence behind `column` of `table` after the highest value the rows have, so the
/// next row the program writes gets a new number.
pub(crate) async fn continue_count(
    transaction: &mut Transaction,
    table: &str,
    column: &str,
) -> Result<(), StoreError> {
    let mut highest = Query::select();
    highest
        .expr_as(
            Expr::col(Alias::new(column))
                .max()
                .cast_as(Alias::new("bigint")),
            Answer::Name,
        )
        .from(Alias::new(table));
    let top: Option<i64> = match transaction.fetch_optional(&highest).await? {
        Some(row) => row.get(Answer::Name)?,
        None => None,
    };
    let mut set = Query::select();
    set.expr(Expr::cust_with_values(
        "setval(pg_get_serial_sequence(quote_ident($1), $2), $3, false)",
        [
            sea_query::Value::from(table),
            sea_query::Value::from(column),
            sea_query::Value::from(top.unwrap_or(0).saturating_add(1)),
        ],
    ));
    transaction.fetch_all(&set).await.map(|_| ())
}
