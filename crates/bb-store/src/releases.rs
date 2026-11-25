//! The adapter of the release port (`bb_app::releases::ReleaseStore`): the releases of the
//! projects and the releases an issue was found in (ISS-08, ADR-0049).

use bb_app::releases::ReleaseStore;
use bb_core::id::{IssueId, ProjectId, ReleaseId, UserId};
use bb_core::project_release::{IssueFoundIn, Release, ReleaseState};
use bb_core::version::Version;
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction, sorted_chunks, to_micros};
use crate::error::StoreError;
use crate::issues::{read_day, stored_day};
use crate::locks::LockClass;
use crate::schema::{IssueFoundInRow, IssueRow, ReleaseRow};

fn select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            ReleaseRow::Id,
            ReleaseRow::Project,
            ReleaseRow::Name,
            ReleaseRow::ReleaseDate,
            ReleaseRow::State,
            ReleaseRow::ReleasedOn,
            ReleaseRow::MadeBy,
            ReleaseRow::MadeAt,
            ReleaseRow::ChangedBy,
            ReleaseRow::ChangedAt,
            ReleaseRow::DeletedAt,
            ReleaseRow::Version,
        ])
        .from(ReleaseRow::Table);
    select
}

fn read_release(row: &Row) -> Result<Release, StoreError> {
    Ok(Release {
        id: row.id::<ReleaseId>(ReleaseRow::Id)?,
        project: row.id::<ProjectId>(ReleaseRow::Project)?,
        name: row.get(ReleaseRow::Name)?,
        release_date: read_day(row, ReleaseRow::ReleaseDate)?,
        state: ReleaseState::from_code(&row.get::<String>(ReleaseRow::State)?)
            .ok_or(StoreError::Corrupt("the state of a release"))?,
        released_on: read_day(row, ReleaseRow::ReleasedOn)?,
        made_by: row.id::<UserId>(ReleaseRow::MadeBy)?,
        made_at: row.time(ReleaseRow::MadeAt)?,
        changed_by: row.id::<UserId>(ReleaseRow::ChangedBy)?,
        changed_at: row.time(ReleaseRow::ChangedAt)?,
        deleted_at: row.optional_time(ReleaseRow::DeletedAt)?,
        version: row.get(ReleaseRow::Version)?,
    })
}

async fn releases_where(
    transaction: &mut Transaction,
    select: &SelectStatement,
) -> Result<Vec<Release>, StoreError> {
    transaction
        .fetch_all(select)
        .await?
        .iter()
        .map(read_release)
        .collect()
}

/// Whether the select finds a row; it asks for one at most, so nothing is counted
/// (ARCH-NO-COUNTERS).
async fn any_row(
    transaction: &mut Transaction,
    select: &mut SelectStatement,
) -> Result<bool, StoreError> {
    select.limit(1);
    Ok(transaction.fetch_optional(&*select).await?.is_some())
}

impl ReleaseStore for AuthStorage {
    async fn release_in(
        transaction: &mut Transaction,
        id: ReleaseId,
    ) -> Result<Option<Release>, StoreError> {
        let mut select = select();
        select.and_where(Expr::col(ReleaseRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_release)
            .transpose()
    }

    async fn releases_of_projects_in(
        transaction: &mut Transaction,
        projects: Vec<ProjectId>,
    ) -> Result<Vec<Release>, StoreError> {
        let mut found = Vec::new();
        for chunk in sorted_chunks(&projects) {
            let mut select = select();
            select
                .and_where(Expr::col(ReleaseRow::Project).is_in(chunk))
                .and_where(Expr::col(ReleaseRow::DeletedAt).is_null())
                .order_by(ReleaseRow::Id, Order::Asc);
            found.extend(releases_where(transaction, &select).await?);
        }
        Ok(found)
    }

    async fn releases_by_ids_in(
        transaction: &mut Transaction,
        ids: Vec<ReleaseId>,
    ) -> Result<Vec<Release>, StoreError> {
        let mut found = Vec::new();
        for chunk in sorted_chunks(&ids) {
            let mut select = select();
            select
                .and_where(Expr::col(ReleaseRow::Id).is_in(chunk))
                .order_by(ReleaseRow::Id, Order::Asc);
            found.extend(releases_where(transaction, &select).await?);
        }
        Ok(found)
    }

    async fn found_in_of_issues_in(
        transaction: &mut Transaction,
        issues: Vec<IssueId>,
    ) -> Result<Vec<IssueFoundIn>, StoreError> {
        let mut found = Vec::new();
        for chunk in sorted_chunks(&issues) {
            let mut select = Query::select();
            select
                .columns([
                    IssueFoundInRow::Issue,
                    IssueFoundInRow::Release,
                    IssueFoundInRow::Position,
                ])
                .from(IssueFoundInRow::Table)
                .and_where(Expr::col(IssueFoundInRow::Issue).is_in(chunk))
                .order_by(IssueFoundInRow::Issue, Order::Asc)
                .order_by(IssueFoundInRow::Position, Order::Asc);
            for row in transaction.fetch_all(&select).await? {
                found.push(IssueFoundIn {
                    issue: row.id::<IssueId>(IssueFoundInRow::Issue)?,
                    release: row.id::<ReleaseId>(IssueFoundInRow::Release)?,
                    position: row.get(IssueFoundInRow::Position)?,
                });
            }
        }
        Ok(found)
    }

    async fn release_named_in(
        transaction: &mut Transaction,
        release: ReleaseId,
    ) -> Result<bool, StoreError> {
        let mut found_in = Query::select();
        found_in
            .expr(Expr::val(1))
            .from(IssueFoundInRow::Table)
            .and_where(Expr::col(IssueFoundInRow::Release).eq(release.stored()));
        if any_row(transaction, &mut found_in).await? {
            return Ok(true);
        }
        let mut planned = Query::select();
        planned
            .expr(Expr::val(1))
            .from(IssueRow::Table)
            .and_where(Expr::col(IssueRow::PlannedForRelease).eq(release.stored()));
        any_row(transaction, &mut planned).await
    }

    async fn lock_releases(transaction: &mut Transaction) -> Result<(), StoreError> {
        transaction.advisory_lock(LockClass::Releases, 0).await
    }

    async fn insert_release(
        transaction: &mut Transaction,
        release: &Release,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(ReleaseRow::Table)
            .columns([
                ReleaseRow::Id,
                ReleaseRow::Project,
                ReleaseRow::Name,
                ReleaseRow::ReleaseDate,
                ReleaseRow::State,
                ReleaseRow::ReleasedOn,
                ReleaseRow::MadeBy,
                ReleaseRow::MadeAt,
                ReleaseRow::ChangedBy,
                ReleaseRow::ChangedAt,
                ReleaseRow::DeletedAt,
                ReleaseRow::Version,
            ])
            .values([
                Expr::val(release.id.stored()),
                Expr::val(release.project.stored()),
                Expr::val(release.name.as_str()),
                Expr::val(stored_day(release.release_date)),
                Expr::val(release.state.code()),
                Expr::val(stored_day(release.released_on)),
                Expr::val(release.made_by.stored()),
                Expr::val(to_micros(release.made_at)),
                Expr::val(release.changed_by.stored()),
                Expr::val(to_micros(release.changed_at)),
                Expr::val(release.deleted_at.map(to_micros)),
                Expr::val(release.version),
            ])
            .map_err(|_| StoreError::Corrupt("a release"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn update_release(
        transaction: &mut Transaction,
        release: &Release,
        version: Version,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(ReleaseRow::Table)
            .value(ReleaseRow::Name, release.name.as_str())
            .value(ReleaseRow::ReleaseDate, stored_day(release.release_date))
            .value(ReleaseRow::State, release.state.code())
            .value(ReleaseRow::ReleasedOn, stored_day(release.released_on))
            .value(ReleaseRow::ChangedBy, release.changed_by.stored())
            .value(ReleaseRow::ChangedAt, to_micros(release.changed_at))
            .value(ReleaseRow::DeletedAt, release.deleted_at.map(to_micros))
            .value(ReleaseRow::Version, version.get() + 1)
            .and_where(Expr::col(ReleaseRow::Id).eq(release.id.stored()))
            .and_where(Expr::col(ReleaseRow::Version).eq(version.get()));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn replace_found_in(
        transaction: &mut Transaction,
        issue: IssueId,
        rows: &[IssueFoundIn],
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(IssueFoundInRow::Table)
            .and_where(Expr::col(IssueFoundInRow::Issue).eq(issue.stored()));
        transaction.execute(&delete).await?;
        if rows.is_empty() {
            return Ok(());
        }
        let mut insert = Query::insert();
        insert.into_table(IssueFoundInRow::Table).columns([
            IssueFoundInRow::Issue,
            IssueFoundInRow::Release,
            IssueFoundInRow::Position,
        ]);
        for row in rows {
            insert
                .values([
                    Expr::val(row.issue.stored()),
                    Expr::val(row.release.stored()),
                    Expr::val(row.position),
                ])
                .map_err(|_| StoreError::Corrupt("a release of an issue"))?;
        }
        transaction.execute(&insert).await?;
        Ok(())
    }
}
