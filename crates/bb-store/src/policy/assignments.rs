//! The rows of *Who has which role*.

use bb_core::access::{AppliesTo, Assignment};
use bb_core::id::{AssignmentId, GroupId, ProjectId, RoleId, UserId};
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::db::{Reader, Row as DbRow, StoredId as _, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::AssignmentRow;

pub(crate) fn read_assignment(row: &DbRow) -> Result<Assignment, StoreError> {
    Ok(Assignment {
        id: row.id::<AssignmentId>(AssignmentRow::Id)?,
        person: row.optional_id::<UserId>(AssignmentRow::Person)?,
        group: row.optional_id::<GroupId>(AssignmentRow::Group)?,
        role: row.optional_id::<RoleId>(AssignmentRow::Role)?,
        project: row.optional_id::<ProjectId>(AssignmentRow::Project)?,
        applies_to: AppliesTo::from_code(&row.get::<String>(AssignmentRow::AppliesTo)?)
            .ok_or(StoreError::Corrupt("a place of a role"))?,
        given_at: row.time(AssignmentRow::GivenAt)?,
    })
}

/// Every row, in the order they were made; the reads narrow it.
pub(crate) fn assignment_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            AssignmentRow::Id,
            AssignmentRow::Person,
            AssignmentRow::Group,
            AssignmentRow::Role,
            AssignmentRow::Project,
            AssignmentRow::AppliesTo,
            AssignmentRow::GivenAt,
        ])
        .from(AssignmentRow::Table)
        .order_by(AssignmentRow::GivenAt, Order::Asc)
        .order_by(AssignmentRow::Id, Order::Asc);
    select
}

/// The rows of a query made from [`assignment_select`].
pub(crate) async fn assignments_of(
    reader: &mut impl Reader,
    select: &SelectStatement,
) -> Result<Vec<Assignment>, StoreError> {
    reader
        .fetch_all(select)
        .await?
        .iter()
        .map(read_assignment)
        .collect()
}

/// The rows on a project (`Some`) or for the whole organization (`None`).
pub(crate) async fn assignments_on(
    reader: &mut impl Reader,
    project: Option<ProjectId>,
) -> Result<Vec<Assignment>, StoreError> {
    let mut select = assignment_select();
    match project {
        Some(project) => {
            select.and_where(Expr::col(AssignmentRow::Project).eq(project.stored()));
        }
        None => {
            select.and_where(Expr::col(AssignmentRow::Project).is_null());
        }
    }
    assignments_of(reader, &select).await
}

/// The rows of a group, wherever they are.
pub(crate) async fn assignments_of_group(
    reader: &mut impl Reader,
    group: GroupId,
) -> Result<Vec<Assignment>, StoreError> {
    let mut select = assignment_select();
    select.and_where(Expr::col(AssignmentRow::Group).eq(group.stored()));
    assignments_of(reader, &select).await
}

/// The rows of a person: their own, not their groups'.
pub(crate) async fn assignments_of_person(
    reader: &mut impl Reader,
    person: UserId,
) -> Result<Vec<Assignment>, StoreError> {
    let mut select = assignment_select();
    select.and_where(Expr::col(AssignmentRow::Person).eq(person.stored()));
    assignments_of(reader, &select).await
}

/// The rows that give a role.
pub(crate) async fn assignments_of_role(
    reader: &mut impl Reader,
    role: RoleId,
) -> Result<Vec<Assignment>, StoreError> {
    let mut select = assignment_select();
    select.and_where(Expr::col(AssignmentRow::Role).eq(role.stored()));
    assignments_of(reader, &select).await
}

/// One row.
pub(crate) async fn assignment_of(
    reader: &mut impl Reader,
    id: AssignmentId,
) -> Result<Option<Assignment>, StoreError> {
    let mut select = assignment_select();
    select.and_where(Expr::col(AssignmentRow::Id).eq(id.stored()));
    reader
        .fetch_optional(&select)
        .await?
        .as_ref()
        .map(read_assignment)
        .transpose()
}

/// Adds a row.
pub(crate) async fn insert_assignment(
    transaction: &mut Transaction,
    assignment: &Assignment,
) -> Result<(), StoreError> {
    let mut insert = Query::insert();
    insert
        .into_table(AssignmentRow::Table)
        .columns([
            AssignmentRow::Id,
            AssignmentRow::Person,
            AssignmentRow::Group,
            AssignmentRow::Role,
            AssignmentRow::Project,
            AssignmentRow::AppliesTo,
            AssignmentRow::GivenAt,
        ])
        .values([
            Expr::val(assignment.id.stored()),
            Expr::val(assignment.person.map(UserId::stored)),
            Expr::val(assignment.group.map(GroupId::stored)),
            Expr::val(assignment.role.map(RoleId::stored)),
            Expr::val(assignment.project.map(ProjectId::stored)),
            Expr::val(assignment.applies_to.code()),
            Expr::val(to_micros(assignment.given_at)),
        ])
        .map_err(|_| StoreError::Corrupt("an assignment"))?;
    transaction.execute(&insert).await.map(|_| ())
}

/// Removes a row; `false` if there was none.
pub(crate) async fn delete_assignment(
    transaction: &mut Transaction,
    id: AssignmentId,
) -> Result<bool, StoreError> {
    let mut delete = Query::delete();
    delete
        .from_table(AssignmentRow::Table)
        .and_where(Expr::col(AssignmentRow::Id).eq(id.stored()));
    Ok(transaction.execute(&delete).await? > 0)
}
