//! Groups and who is in them.

use bb_core::access::{Group, GroupMember};
use bb_core::id::{GroupId, UserId};
use sea_query::{Expr, ExprTrait as _, Order, Query};

use super::delete_by;
use crate::db::{Reader, Row as DbRow, StoredId as _, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{AssignmentRow, GroupMemberRow, GroupRow};

fn read_group(row: &DbRow) -> Result<Group, StoreError> {
    Ok(Group {
        id: row.id::<GroupId>(GroupRow::Id)?,
        name: row.get(GroupRow::Name)?,
    })
}

/// The members a query made from [`member_select`] found.
pub(crate) fn read_member(row: &DbRow) -> Result<GroupMember, StoreError> {
    Ok(GroupMember {
        group: row.id::<GroupId>(GroupMemberRow::Group)?,
        person: row.id::<UserId>(GroupMemberRow::Person)?,
        added_at: row.time(GroupMemberRow::AddedAt)?,
    })
}

/// Every membership, in the order they were made.
pub(crate) fn member_select() -> sea_query::SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            GroupMemberRow::Group,
            GroupMemberRow::Person,
            GroupMemberRow::AddedAt,
        ])
        .from(GroupMemberRow::Table)
        .order_by(GroupMemberRow::AddedAt, Order::Asc)
        .order_by(GroupMemberRow::Person, Order::Asc);
    select
}

/// Every group, by name.
pub(crate) async fn groups(reader: &mut impl Reader) -> Result<Vec<Group>, StoreError> {
    let mut select = Query::select();
    select
        .columns([GroupRow::Id, GroupRow::Name])
        .from(GroupRow::Table)
        .order_by(GroupRow::Name, Order::Asc)
        .order_by(GroupRow::Id, Order::Asc);
    reader
        .fetch_all(&select)
        .await?
        .iter()
        .map(read_group)
        .collect()
}

/// One group.
pub(crate) async fn group(
    reader: &mut impl Reader,
    id: GroupId,
) -> Result<Option<Group>, StoreError> {
    let mut select = Query::select();
    select
        .columns([GroupRow::Id, GroupRow::Name])
        .from(GroupRow::Table)
        .and_where(Expr::col(GroupRow::Id).eq(id.stored()));
    reader
        .fetch_optional(&select)
        .await?
        .as_ref()
        .map(read_group)
        .transpose()
}

/// The people in a group, in the order they were put there.
pub(crate) async fn members(
    reader: &mut impl Reader,
    group: GroupId,
) -> Result<Vec<GroupMember>, StoreError> {
    let mut select = member_select();
    select.and_where(Expr::col(GroupMemberRow::Group).eq(group.stored()));
    reader
        .fetch_all(&select)
        .await?
        .iter()
        .map(read_member)
        .collect()
}

/// The groups a person is in, by name.
pub(crate) async fn groups_of(
    reader: &mut impl Reader,
    person: UserId,
) -> Result<Vec<Group>, StoreError> {
    let mut member_of = Query::select();
    member_of
        .column(GroupMemberRow::Group)
        .from(GroupMemberRow::Table)
        .and_where(Expr::col(GroupMemberRow::Person).eq(person.stored()));
    let mut select = Query::select();
    select
        .columns([GroupRow::Id, GroupRow::Name])
        .from(GroupRow::Table)
        .and_where(Expr::col(GroupRow::Id).in_subquery(member_of))
        .order_by(GroupRow::Name, Order::Asc)
        .order_by(GroupRow::Id, Order::Asc);
    reader
        .fetch_all(&select)
        .await?
        .iter()
        .map(read_group)
        .collect()
}

/// Adds a group.
pub(crate) async fn insert_group(
    transaction: &mut Transaction,
    group: &Group,
) -> Result<(), StoreError> {
    let mut insert = Query::insert();
    insert
        .into_table(GroupRow::Table)
        .columns([GroupRow::Id, GroupRow::Name])
        .values([Expr::val(group.id.stored()), Expr::val(group.name.as_str())])
        .map_err(|_| StoreError::Corrupt("a group"))?;
    transaction.execute(&insert).await.map(|_| ())
}

/// Renames a group; `false` if there is no such group.
pub(crate) async fn rename_group(
    transaction: &mut Transaction,
    id: GroupId,
    name: &str,
) -> Result<bool, StoreError> {
    let mut update = Query::update();
    update
        .table(GroupRow::Table)
        .value(GroupRow::Name, name)
        .and_where(Expr::col(GroupRow::Id).eq(id.stored()));
    Ok(transaction.execute(&update).await? > 0)
}

/// Removes a group, with its members and its rows.
pub(crate) async fn delete_group(
    transaction: &mut Transaction,
    id: GroupId,
) -> Result<bool, StoreError> {
    let group = id.stored();
    delete_by(
        transaction,
        AssignmentRow::Table,
        AssignmentRow::Group,
        &group,
    )
    .await?;
    delete_by(
        transaction,
        GroupMemberRow::Table,
        GroupMemberRow::Group,
        &group,
    )
    .await?;
    Ok(delete_by(transaction, GroupRow::Table, GroupRow::Id, &group).await? > 0)
}

/// Puts a person into a group.
pub(crate) async fn insert_group_member(
    transaction: &mut Transaction,
    member: &GroupMember,
) -> Result<(), StoreError> {
    let mut insert = Query::insert();
    insert
        .into_table(GroupMemberRow::Table)
        .columns([
            GroupMemberRow::Group,
            GroupMemberRow::Person,
            GroupMemberRow::AddedAt,
        ])
        .values([
            Expr::val(member.group.stored()),
            Expr::val(member.person.stored()),
            Expr::val(to_micros(member.added_at)),
        ])
        .map_err(|_| StoreError::Corrupt("a group member"))?;
    transaction.execute(&insert).await.map(|_| ())
}

/// Takes a person out of a group; `false` if they were not in it.
pub(crate) async fn delete_group_member(
    transaction: &mut Transaction,
    group: GroupId,
    person: UserId,
) -> Result<bool, StoreError> {
    let mut delete = Query::delete();
    delete
        .from_table(GroupMemberRow::Table)
        .and_where(Expr::col(GroupMemberRow::Group).eq(group.stored()))
        .and_where(Expr::col(GroupMemberRow::Person).eq(person.stored()));
    Ok(transaction.execute(&delete).await? > 0)
}
