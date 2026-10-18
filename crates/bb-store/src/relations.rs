//! The adapter of the relations port (`bb_app::relations::RelationStore`): kinds of relation, the
//! relations between issues, and which issue sits under which (ISS-03, ISS-04).
//!
//! Like the issues adapter, reads run in one snapshot and writes are associated functions that run
//! inside the transaction of a change. The unique key of a relation (kind, from, to) keeps the same
//! relation from being added twice at the same moment.

use bb_app::relations::RelationStore;
use bb_core::id::{IssueId, RelationId, RelationKindId, UserId};
use bb_core::issue::Issue;
use bb_core::relation::{IssueParent, Relation, RelationKind};
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, OnConflict, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::issues::{issue_select, read_issue};
use crate::locks::LockClass;
use crate::schema::{IssueParentRow, IssueRow, RelationKindRow, RelationRow};

fn kind_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            RelationKindRow::Id,
            RelationKindRow::Forward,
            RelationKindRow::Backward,
            RelationKindRow::Position,
            RelationKindRow::RetiredAt,
            RelationKindRow::Version,
        ])
        .from(RelationKindRow::Table);
    select
}

fn read_kind(row: &Row) -> Result<RelationKind, StoreError> {
    Ok(RelationKind {
        id: row.id::<RelationKindId>(RelationKindRow::Id)?,
        forward: row.get(RelationKindRow::Forward)?,
        backward: row.get(RelationKindRow::Backward)?,
        position: row.get(RelationKindRow::Position)?,
        retired_at: row.optional_time(RelationKindRow::RetiredAt)?,
        version: row.get(RelationKindRow::Version)?,
    })
}

fn relation_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            RelationRow::Id,
            RelationRow::Kind,
            RelationRow::FromIssue,
            RelationRow::ToIssue,
            RelationRow::AddedBy,
            RelationRow::AddedAt,
        ])
        .from(RelationRow::Table);
    select
}

fn read_relation(row: &Row) -> Result<Relation, StoreError> {
    Ok(Relation {
        id: row.id::<RelationId>(RelationRow::Id)?,
        kind: row.id::<RelationKindId>(RelationRow::Kind)?,
        from: row.id::<IssueId>(RelationRow::FromIssue)?,
        to: row.id::<IssueId>(RelationRow::ToIssue)?,
        added_by: row.id::<UserId>(RelationRow::AddedBy)?,
        added_at: row.time(RelationRow::AddedAt)?,
    })
}

fn parent_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            IssueParentRow::Child,
            IssueParentRow::Parent,
            IssueParentRow::SetBy,
            IssueParentRow::SetAt,
        ])
        .from(IssueParentRow::Table);
    select
}

fn read_parent(row: &Row) -> Result<IssueParent, StoreError> {
    Ok(IssueParent {
        child: row.id::<IssueId>(IssueParentRow::Child)?,
        parent: row.id::<IssueId>(IssueParentRow::Parent)?,
        set_by: row.id::<UserId>(IssueParentRow::SetBy)?,
        set_at: row.time(IssueParentRow::SetAt)?,
    })
}

impl RelationStore for AuthStorage {
    async fn relation_kinds_in(
        transaction: &mut Transaction,
    ) -> Result<Vec<RelationKind>, StoreError> {
        let mut select = kind_select();
        select
            .order_by(RelationKindRow::Position, Order::Asc)
            .order_by(RelationKindRow::Id, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_kind)
            .collect()
    }

    async fn relation_in(
        transaction: &mut Transaction,
        id: RelationId,
    ) -> Result<Option<Relation>, StoreError> {
        let mut select = relation_select();
        select.and_where(Expr::col(RelationRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_relation)
            .transpose()
    }

    async fn relations_of_in(
        transaction: &mut Transaction,
        issue: IssueId,
    ) -> Result<Vec<Relation>, StoreError> {
        let mut select = relation_select();
        select
            .cond_where(
                sea_query::Condition::any()
                    .add(Expr::col(RelationRow::FromIssue).eq(issue.stored()))
                    .add(Expr::col(RelationRow::ToIssue).eq(issue.stored())),
            )
            .order_by(RelationRow::AddedAt, Order::Asc)
            .order_by(RelationRow::Id, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_relation)
            .collect()
    }

    async fn relation_between_in(
        transaction: &mut Transaction,
        kind: RelationKindId,
        from: IssueId,
        to: IssueId,
    ) -> Result<Option<Relation>, StoreError> {
        let mut select = relation_select();
        select
            .and_where(Expr::col(RelationRow::Kind).eq(kind.stored()))
            .and_where(Expr::col(RelationRow::FromIssue).eq(from.stored()))
            .and_where(Expr::col(RelationRow::ToIssue).eq(to.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_relation)
            .transpose()
    }

    async fn parent_of_in(
        transaction: &mut Transaction,
        child: IssueId,
    ) -> Result<Option<IssueParent>, StoreError> {
        let mut select = parent_select();
        select.and_where(Expr::col(IssueParentRow::Child).eq(child.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_parent)
            .transpose()
    }

    async fn sub_issues_of_in(
        transaction: &mut Transaction,
        parent: IssueId,
    ) -> Result<Vec<IssueParent>, StoreError> {
        let mut select = parent_select();
        select
            .and_where(Expr::col(IssueParentRow::Parent).eq(parent.stored()))
            .order_by(IssueParentRow::SetAt, Order::Asc)
            .order_by(IssueParentRow::Child, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_parent)
            .collect()
    }

    async fn issues_by_ids(&self, ids: &[IssueId]) -> Result<Vec<Issue>, StoreError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = issue_select();
        select.and_where(
            Expr::col(IssueRow::Id).is_in(ids.iter().map(|id| id.stored()).collect::<Vec<_>>()),
        );
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_issue)
            .collect()
    }

    async fn insert_relation_kind(
        transaction: &mut Transaction,
        kind: &RelationKind,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(RelationKindRow::Table)
            .columns([
                RelationKindRow::Id,
                RelationKindRow::Forward,
                RelationKindRow::Backward,
                RelationKindRow::Position,
                RelationKindRow::RetiredAt,
                RelationKindRow::Version,
            ])
            .values([
                Expr::val(kind.id.stored()),
                Expr::val(kind.forward.as_str()),
                Expr::val(kind.backward.as_str()),
                Expr::val(kind.position),
                Expr::val(kind.retired_at.map(to_micros)),
                Expr::val(kind.version),
            ])
            .map_err(|_| StoreError::Corrupt("a kind of relation"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn update_relation_kind(
        transaction: &mut Transaction,
        id: RelationKindId,
        forward: &str,
        backward: &str,
        retired_at: Option<Timestamp>,
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(RelationKindRow::Table)
            .value(RelationKindRow::Forward, forward)
            .value(RelationKindRow::Backward, backward)
            .value(RelationKindRow::RetiredAt, retired_at.map(to_micros))
            .value(
                RelationKindRow::Version,
                Expr::col(RelationKindRow::Version).add(1),
            )
            .and_where(Expr::col(RelationKindRow::Id).eq(id.stored()))
            .and_where(Expr::col(RelationKindRow::Version).eq(version));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn insert_relation(
        transaction: &mut Transaction,
        relation: &Relation,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(RelationRow::Table)
            .columns([
                RelationRow::Id,
                RelationRow::Kind,
                RelationRow::FromIssue,
                RelationRow::ToIssue,
                RelationRow::AddedBy,
                RelationRow::AddedAt,
            ])
            .values([
                Expr::val(relation.id.stored()),
                Expr::val(relation.kind.stored()),
                Expr::val(relation.from.stored()),
                Expr::val(relation.to.stored()),
                Expr::val(relation.added_by.stored()),
                Expr::val(to_micros(relation.added_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a relation"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn delete_relation(
        transaction: &mut Transaction,
        id: RelationId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(RelationRow::Table)
            .and_where(Expr::col(RelationRow::Id).eq(id.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn set_issue_parent(
        transaction: &mut Transaction,
        parent: &IssueParent,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(IssueParentRow::Table)
            .columns([
                IssueParentRow::Child,
                IssueParentRow::Parent,
                IssueParentRow::SetBy,
                IssueParentRow::SetAt,
            ])
            .values([
                Expr::val(parent.child.stored()),
                Expr::val(parent.parent.stored()),
                Expr::val(parent.set_by.stored()),
                Expr::val(to_micros(parent.set_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a parent"))?
            .on_conflict(
                OnConflict::column(IssueParentRow::Child)
                    .update_columns([
                        IssueParentRow::Parent,
                        IssueParentRow::SetBy,
                        IssueParentRow::SetAt,
                    ])
                    .to_owned(),
            );
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn clear_issue_parent(
        transaction: &mut Transaction,
        child: IssueId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(IssueParentRow::Table)
            .and_where(Expr::col(IssueParentRow::Child).eq(child.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn lock_issue_parents(transaction: &mut Transaction) -> Result<(), StoreError> {
        transaction.advisory_lock(LockClass::IssueParents, 0).await
    }

    async fn lock_issue_relations(transaction: &mut Transaction) -> Result<(), StoreError> {
        transaction
            .advisory_lock(LockClass::IssueRelations, 0)
            .await
    }
}
