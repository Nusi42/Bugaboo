//! The adapter of the issues port (`bb_app::issues::IssueStore`): issue types, issues, the people
//! they are assigned to and the codes of their readable IDs (ISS-01, ISS-01a, ISS-02).
//!
//! Like the projects adapter, reads run in one snapshot, and writes are associated functions that
//! run inside the transaction of a change. The counting number of a project is the one counter
//! of the product: `next_counting_number` raises the stored last number by one in the
//! transaction, so two writers wait for each other and no number is skipped or given twice.

use bb_app::issues::{IssueFound, IssueStore};
use bb_core::event::DomainEvent;
use bb_core::id::{IssueId, IssueTypeId, ProjectId, UserId};
use bb_core::issue::{Issue, IssueAlias, IssueAssignee, IssueIcon, IssueType, Priority, Status};
use bb_core::issue_key::IssueCode;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, OnConflict, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Reader, Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{IssueAliasRow, IssueAssigneeRow, IssueCounterRow, IssueRow, IssueTypeRow};

/// Apart from the other advisory locks of the program (`MIGRATION_LOCK`, `POLICY_LOCK`): the lock
/// that serializes the codes of one project is this number xor eight bytes of the project's ID.
const ISSUE_CODES_LOCK: i64 = 0x4242_4953_5545_0001;

fn type_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            IssueTypeRow::Id,
            IssueTypeRow::Name,
            IssueTypeRow::Icon,
            IssueTypeRow::Position,
            IssueTypeRow::RetiredAt,
            IssueTypeRow::Version,
        ])
        .from(IssueTypeRow::Table);
    select
}

fn read_type(row: &Row) -> Result<IssueType, StoreError> {
    Ok(IssueType {
        id: row.id::<IssueTypeId>(IssueTypeRow::Id)?,
        name: row.get(IssueTypeRow::Name)?,
        icon: IssueIcon::from_code(&row.get::<String>(IssueTypeRow::Icon)?)
            .ok_or(StoreError::Corrupt("an issue icon"))?,
        position: row.get(IssueTypeRow::Position)?,
        retired_at: row.optional_time(IssueTypeRow::RetiredAt)?,
        version: row.get(IssueTypeRow::Version)?,
    })
}

fn issue_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            IssueRow::Id,
            IssueRow::Project,
            IssueRow::IssueType,
            IssueRow::Code,
            IssueRow::Title,
            IssueRow::Description,
            IssueRow::Status,
            IssueRow::Priority,
            IssueRow::ReportedBy,
            IssueRow::CreatedAt,
            IssueRow::UpdatedAt,
            IssueRow::ClosedAt,
            IssueRow::DeletedAt,
            IssueRow::Version,
        ])
        .from(IssueRow::Table);
    select
}

fn read_code(text: &str) -> Result<IssueCode, StoreError> {
    IssueCode::new(text).map_err(|_| StoreError::Corrupt("an issue code"))
}

fn read_issue(row: &Row) -> Result<Issue, StoreError> {
    Ok(Issue {
        id: row.id::<IssueId>(IssueRow::Id)?,
        project: row.id::<ProjectId>(IssueRow::Project)?,
        issue_type: row.id::<IssueTypeId>(IssueRow::IssueType)?,
        code: read_code(&row.get::<String>(IssueRow::Code)?)?,
        title: row.get(IssueRow::Title)?,
        description: row.get(IssueRow::Description)?,
        status: Status::from_code(&row.get::<String>(IssueRow::Status)?)
            .ok_or(StoreError::Corrupt("a status"))?,
        priority: Priority::from_code(&row.get::<String>(IssueRow::Priority)?)
            .ok_or(StoreError::Corrupt("a priority"))?,
        reported_by: row.id::<UserId>(IssueRow::ReportedBy)?,
        created_at: row.time(IssueRow::CreatedAt)?,
        updated_at: row.time(IssueRow::UpdatedAt)?,
        closed_at: row.optional_time(IssueRow::ClosedAt)?,
        deleted_at: row.optional_time(IssueRow::DeletedAt)?,
        version: row.get(IssueRow::Version)?,
    })
}

fn read_assignee(row: &Row) -> Result<IssueAssignee, StoreError> {
    Ok(IssueAssignee {
        issue: row.id::<IssueId>(IssueAssigneeRow::Issue)?,
        person: row.id::<UserId>(IssueAssigneeRow::Person)?,
        added_at: row.time(IssueAssigneeRow::AddedAt)?,
    })
}

async fn issue_in(reader: &mut impl Reader, id: IssueId) -> Result<Option<Issue>, StoreError> {
    let mut select = issue_select();
    select.and_where(Expr::col(IssueRow::Id).eq(id.stored()));
    reader
        .fetch_optional(&select)
        .await?
        .as_ref()
        .map(read_issue)
        .transpose()
}

async fn issue_at_in(
    reader: &mut impl Reader,
    project: ProjectId,
    code: &IssueCode,
) -> Result<Option<IssueFound>, StoreError> {
    let mut select = issue_select();
    select
        .and_where(Expr::col(IssueRow::Project).eq(project.stored()))
        .and_where(Expr::col(IssueRow::Code).eq(code.as_str()));
    if let Some(row) = reader.fetch_optional(&select).await? {
        return Ok(Some(IssueFound {
            issue: read_issue(&row)?,
            old_code: false,
        }));
    }
    let mut select = Query::select();
    select
        .column(IssueAliasRow::Issue)
        .from(IssueAliasRow::Table)
        .and_where(Expr::col(IssueAliasRow::Project).eq(project.stored()))
        .and_where(Expr::col(IssueAliasRow::Code).eq(code.as_str()));
    let Some(row) = reader.fetch_optional(&select).await? else {
        return Ok(None);
    };
    let issue = row.id::<IssueId>(IssueAliasRow::Issue)?;
    Ok(issue_in(reader, issue).await?.map(|issue| IssueFound {
        issue,
        old_code: true,
    }))
}

async fn code_taken_in(
    reader: &mut impl Reader,
    project: ProjectId,
    code: &IssueCode,
) -> Result<bool, StoreError> {
    let mut select = Query::select();
    select
        .column(IssueRow::Id)
        .from(IssueRow::Table)
        .and_where(Expr::col(IssueRow::Project).eq(project.stored()))
        .and_where(Expr::col(IssueRow::Code).eq(code.as_str()));
    if reader.fetch_optional(&select).await?.is_some() {
        return Ok(true);
    }
    let mut select = Query::select();
    select
        .column(IssueAliasRow::Issue)
        .from(IssueAliasRow::Table)
        .and_where(Expr::col(IssueAliasRow::Project).eq(project.stored()))
        .and_where(Expr::col(IssueAliasRow::Code).eq(code.as_str()));
    Ok(reader.fetch_optional(&select).await?.is_some())
}

async fn assignees_of_in(
    reader: &mut impl Reader,
    issues: &[IssueId],
) -> Result<Vec<IssueAssignee>, StoreError> {
    if issues.is_empty() {
        return Ok(Vec::new());
    }
    let mut select = Query::select();
    select
        .columns([
            IssueAssigneeRow::Issue,
            IssueAssigneeRow::Person,
            IssueAssigneeRow::AddedAt,
        ])
        .from(IssueAssigneeRow::Table)
        .and_where(
            Expr::col(IssueAssigneeRow::Issue).is_in(
                issues
                    .iter()
                    .map(|issue| issue.stored())
                    .collect::<Vec<_>>(),
            ),
        )
        .order_by(IssueAssigneeRow::AddedAt, Order::Asc)
        .order_by(IssueAssigneeRow::Person, Order::Asc);
    reader
        .fetch_all(&select)
        .await?
        .iter()
        .map(read_assignee)
        .collect()
}

async fn insert_assignees(
    transaction: &mut Transaction,
    assignees: &[IssueAssignee],
) -> Result<(), StoreError> {
    for assignee in assignees {
        let mut insert = Query::insert();
        insert
            .into_table(IssueAssigneeRow::Table)
            .columns([
                IssueAssigneeRow::Issue,
                IssueAssigneeRow::Person,
                IssueAssigneeRow::AddedAt,
            ])
            .values([
                Expr::val(assignee.issue.stored()),
                Expr::val(assignee.person.stored()),
                Expr::val(to_micros(assignee.added_at)),
            ])
            .map_err(|_| StoreError::Corrupt("an assignee"))?;
        transaction.execute(&insert).await?;
    }
    Ok(())
}

impl IssueStore for AuthStorage {
    async fn issue_types_in(transaction: &mut Transaction) -> Result<Vec<IssueType>, StoreError> {
        let mut select = type_select();
        select
            .order_by(IssueTypeRow::Position, Order::Asc)
            .order_by(IssueTypeRow::Id, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_type)
            .collect()
    }

    async fn issue_in(
        transaction: &mut Transaction,
        id: IssueId,
    ) -> Result<Option<Issue>, StoreError> {
        issue_in(transaction, id).await
    }

    async fn issue_at_in(
        transaction: &mut Transaction,
        project: ProjectId,
        code: IssueCode,
    ) -> Result<Option<IssueFound>, StoreError> {
        issue_at_in(transaction, project, &code).await
    }

    async fn issue_code_taken_in(
        transaction: &mut Transaction,
        project: ProjectId,
        code: IssueCode,
    ) -> Result<bool, StoreError> {
        code_taken_in(transaction, project, &code).await
    }

    async fn assignees_in(
        transaction: &mut Transaction,
        issue: IssueId,
    ) -> Result<Vec<IssueAssignee>, StoreError> {
        assignees_of_in(transaction, &[issue]).await
    }

    async fn issue_codes_in(
        transaction: &mut Transaction,
        project: ProjectId,
    ) -> Result<Vec<(IssueId, IssueCode)>, StoreError> {
        let mut select = Query::select();
        select
            .columns([IssueRow::Id, IssueRow::Code])
            .from(IssueRow::Table)
            .and_where(Expr::col(IssueRow::Project).eq(project.stored()))
            .order_by(IssueRow::Id, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                Ok((
                    row.id::<IssueId>(IssueRow::Id)?,
                    read_code(&row.get::<String>(IssueRow::Code)?)?,
                ))
            })
            .collect()
    }

    async fn issue_type(&self, id: IssueTypeId) -> Result<Option<IssueType>, StoreError> {
        let mut select = type_select();
        select.and_where(Expr::col(IssueTypeRow::Id).eq(id.stored()));
        self.database()
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_type)
            .transpose()
    }

    async fn issues_page(
        &self,
        project: ProjectId,
        reported_by: Option<UserId>,
        before: Option<IssueId>,
        limit: u64,
    ) -> Result<Vec<Issue>, StoreError> {
        let mut select = issue_select();
        select
            .and_where(Expr::col(IssueRow::Project).eq(project.stored()))
            .and_where(Expr::col(IssueRow::DeletedAt).is_null())
            .order_by(IssueRow::Id, Order::Desc)
            .limit(limit);
        if let Some(person) = reported_by {
            select.and_where(Expr::col(IssueRow::ReportedBy).eq(person.stored()));
        }
        if let Some(before) = before {
            select.and_where(Expr::col(IssueRow::Id).lt(before.stored()));
        }
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_issue)
            .collect()
    }

    async fn assignees_of(&self, issues: &[IssueId]) -> Result<Vec<IssueAssignee>, StoreError> {
        assignees_of_in(&mut self.database(), issues).await
    }

    async fn insert_issue_type(
        transaction: &mut Transaction,
        issue_type: &IssueType,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(IssueTypeRow::Table)
            .columns([
                IssueTypeRow::Id,
                IssueTypeRow::Name,
                IssueTypeRow::Icon,
                IssueTypeRow::Position,
                IssueTypeRow::RetiredAt,
                IssueTypeRow::Version,
            ])
            .values([
                Expr::val(issue_type.id.stored()),
                Expr::val(issue_type.name.as_str()),
                Expr::val(issue_type.icon.code()),
                Expr::val(issue_type.position),
                Expr::val(issue_type.retired_at.map(to_micros)),
                Expr::val(issue_type.version),
            ])
            .map_err(|_| StoreError::Corrupt("an issue type"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn update_issue_type(
        transaction: &mut Transaction,
        id: IssueTypeId,
        name: &str,
        icon: IssueIcon,
        retired_at: Option<Timestamp>,
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(IssueTypeRow::Table)
            .value(IssueTypeRow::Name, name)
            .value(IssueTypeRow::Icon, icon.code())
            .value(IssueTypeRow::RetiredAt, retired_at.map(to_micros))
            .value(
                IssueTypeRow::Version,
                Expr::col(IssueTypeRow::Version).add(1),
            )
            .and_where(Expr::col(IssueTypeRow::Id).eq(id.stored()))
            .and_where(Expr::col(IssueTypeRow::Version).eq(version));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn insert_issue(
        transaction: &mut Transaction,
        issue: &Issue,
        assignees: &[IssueAssignee],
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(IssueRow::Table)
            .columns([
                IssueRow::Id,
                IssueRow::Project,
                IssueRow::IssueType,
                IssueRow::Code,
                IssueRow::Title,
                IssueRow::Description,
                IssueRow::Status,
                IssueRow::Priority,
                IssueRow::ReportedBy,
                IssueRow::CreatedAt,
                IssueRow::UpdatedAt,
                IssueRow::ClosedAt,
                IssueRow::DeletedAt,
                IssueRow::Version,
            ])
            .values([
                Expr::val(issue.id.stored()),
                Expr::val(issue.project.stored()),
                Expr::val(issue.issue_type.stored()),
                Expr::val(issue.code.as_str()),
                Expr::val(issue.title.as_str()),
                Expr::val(issue.description.as_str()),
                Expr::val(issue.status.code()),
                Expr::val(issue.priority.code()),
                Expr::val(issue.reported_by.stored()),
                Expr::val(to_micros(issue.created_at)),
                Expr::val(to_micros(issue.updated_at)),
                Expr::val(issue.closed_at.map(to_micros)),
                Expr::val(issue.deleted_at.map(to_micros)),
                Expr::val(issue.version),
            ])
            .map_err(|_| StoreError::Corrupt("an issue"))?;
        transaction.execute(&insert).await?;
        insert_assignees(transaction, assignees).await
    }

    async fn update_issue(
        transaction: &mut Transaction,
        issue: &Issue,
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(IssueRow::Table)
            .value(IssueRow::IssueType, issue.issue_type.stored())
            .value(IssueRow::Title, issue.title.as_str())
            .value(IssueRow::Description, issue.description.as_str())
            .value(IssueRow::Status, issue.status.code())
            .value(IssueRow::Priority, issue.priority.code())
            .value(IssueRow::UpdatedAt, to_micros(issue.updated_at))
            .value(IssueRow::ClosedAt, issue.closed_at.map(to_micros))
            .value(IssueRow::Version, Expr::col(IssueRow::Version).add(1))
            .and_where(Expr::col(IssueRow::Id).eq(issue.id.stored()))
            .and_where(Expr::col(IssueRow::Version).eq(version));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn replace_assignees(
        transaction: &mut Transaction,
        issue: IssueId,
        assignees: &[IssueAssignee],
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(IssueAssigneeRow::Table)
            .and_where(Expr::col(IssueAssigneeRow::Issue).eq(issue.stored()));
        transaction.execute(&delete).await?;
        insert_assignees(transaction, assignees).await
    }

    async fn set_issue_deleted(
        transaction: &mut Transaction,
        id: IssueId,
        deleted_at: Option<Timestamp>,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(IssueRow::Table)
            .value(IssueRow::DeletedAt, deleted_at.map(to_micros))
            .value(IssueRow::Version, Expr::col(IssueRow::Version).add(1))
            .and_where(Expr::col(IssueRow::Id).eq(id.stored()));
        // Only an issue that is where the change expects it: not in the trash to put it there,
        // in the trash to take it out.
        update.and_where(if deleted_at.is_some() {
            Expr::col(IssueRow::DeletedAt).is_null()
        } else {
            Expr::col(IssueRow::DeletedAt).is_not_null()
        });
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn next_counting_number(
        transaction: &mut Transaction,
        project: ProjectId,
    ) -> Result<i64, StoreError> {
        let mut start = Query::insert();
        start
            .into_table(IssueCounterRow::Table)
            .columns([IssueCounterRow::Project, IssueCounterRow::Last])
            .values([Expr::val(project.stored()), Expr::val(0_i64)])
            .map_err(|_| StoreError::Corrupt("a counter"))?
            .on_conflict(
                OnConflict::column(IssueCounterRow::Project)
                    .do_nothing()
                    .to_owned(),
            );
        transaction.execute(&start).await?;
        // The update takes the lock of the row until the change ends, so the number read next is
        // this change's own.
        let mut raise = Query::update();
        raise
            .table(IssueCounterRow::Table)
            .value(
                IssueCounterRow::Last,
                Expr::col(IssueCounterRow::Last).add(1),
            )
            .and_where(Expr::col(IssueCounterRow::Project).eq(project.stored()));
        transaction.execute(&raise).await?;
        let mut read = Query::select();
        read.column(IssueCounterRow::Last)
            .from(IssueCounterRow::Table)
            .and_where(Expr::col(IssueCounterRow::Project).eq(project.stored()));
        transaction
            .fetch_optional(&read)
            .await?
            .ok_or(StoreError::Corrupt("a counter"))?
            .get(IssueCounterRow::Last)
    }

    async fn set_issue_code(
        transaction: &mut Transaction,
        id: IssueId,
        code: &IssueCode,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(IssueRow::Table)
            .value(IssueRow::Code, code.as_str())
            .value(IssueRow::Version, Expr::col(IssueRow::Version).add(1))
            .and_where(Expr::col(IssueRow::Id).eq(id.stored()));
        transaction.execute(&update).await.map(|_| ())
    }

    async fn lock_issue_codes(
        transaction: &mut Transaction,
        project: ProjectId,
    ) -> Result<(), StoreError> {
        // The lock is named by the project: eight bytes of its ID, apart from the other locks.
        let bytes = project.to_bytes();
        let mut tail = [0_u8; 8];
        tail.copy_from_slice(&bytes[8..]);
        transaction
            .advisory_lock(i64::from_be_bytes(tail) ^ ISSUE_CODES_LOCK)
            .await
    }

    async fn append_follow_up_event(
        transaction: &mut Transaction,
        event: &DomainEvent,
    ) -> Result<(), StoreError> {
        crate::events::append_event(transaction, event).await
    }

    async fn insert_issue_alias(
        transaction: &mut Transaction,
        alias: &IssueAlias,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(IssueAliasRow::Table)
            .columns([
                IssueAliasRow::Project,
                IssueAliasRow::Code,
                IssueAliasRow::Issue,
                IssueAliasRow::RetiredAt,
            ])
            .values([
                Expr::val(alias.project.stored()),
                Expr::val(alias.code.as_str()),
                Expr::val(alias.issue.stored()),
                Expr::val(to_micros(alias.retired_at)),
            ])
            .map_err(|_| StoreError::Corrupt("an old issue code"))?;
        transaction.execute(&insert).await.map(|_| ())
    }
}
