//! The adapter of the saved-search port (`bb_app::saved_searches::SavedSearchStore`): saved
//! searches, their columns, whom they are shared with and the list of a project (QRY-06).

use bb_app::saved_searches::{Reach, SavedSearchStore};
use bb_core::id::{GroupId, ProjectId, SavedSearchId, UserId};
use bb_core::person::Person;
use bb_core::saved_search::{
    ProjectDefaultList, SavedSearch, SearchColumn, SearchShare, ShareKind, ShareTarget, Spacing,
    ViewKind,
};
use jiff::Timestamp;
use sea_query::{Cond, Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::auth::{person_columns, read_person};
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::locks::{LockClass, lock_key};
use crate::schema::{
    PersonRow, ProjectDefaultListRow, SavedSearchRow, SearchColumnRow, SearchShareRow,
};

fn search_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            SavedSearchRow::Id,
            SavedSearchRow::Name,
            SavedSearchRow::Text,
            SavedSearchRow::Maker,
            SavedSearchRow::View,
            SavedSearchRow::Spacing,
            SavedSearchRow::GroupBy,
            SavedSearchRow::CreatedAt,
            SavedSearchRow::UpdatedAt,
            SavedSearchRow::DeletedAt,
            SavedSearchRow::Version,
        ])
        .from(SavedSearchRow::Table);
    select
}

fn read_search(row: &Row) -> Result<SavedSearch, StoreError> {
    Ok(SavedSearch {
        id: row.id::<SavedSearchId>(SavedSearchRow::Id)?,
        name: row.get(SavedSearchRow::Name)?,
        text: row.get(SavedSearchRow::Text)?,
        maker: row.id::<UserId>(SavedSearchRow::Maker)?,
        view: ViewKind::from_code(&row.get::<String>(SavedSearchRow::View)?)
            .ok_or(StoreError::Corrupt("a kind of view"))?,
        spacing: Spacing::of(&row.get::<String>(SavedSearchRow::Spacing)?)
            .ok_or(StoreError::Corrupt("a spacing"))?,
        group_by: row.get(SavedSearchRow::GroupBy)?,
        created_at: row.time(SavedSearchRow::CreatedAt)?,
        updated_at: row.time(SavedSearchRow::UpdatedAt)?,
        deleted_at: row.optional_time(SavedSearchRow::DeletedAt)?,
        version: row.get(SavedSearchRow::Version)?,
    })
}

fn column_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            SearchColumnRow::Search,
            SearchColumnRow::Position,
            SearchColumnRow::Field,
            SearchColumnRow::Width,
        ])
        .from(SearchColumnRow::Table);
    select
}

fn read_column(row: &Row) -> Result<SearchColumn, StoreError> {
    Ok(SearchColumn {
        search: row.id::<SavedSearchId>(SearchColumnRow::Search)?,
        position: row.get(SearchColumnRow::Position)?,
        field: row.get(SearchColumnRow::Field)?,
        width: row.get(SearchColumnRow::Width)?,
    })
}

fn share_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            SearchShareRow::Search,
            SearchShareRow::Kind,
            SearchShareRow::Target,
            SearchShareRow::SharedAt,
        ])
        .from(SearchShareRow::Table);
    select
}

fn read_share(row: &Row) -> Result<SearchShare, StoreError> {
    let kind = ShareKind::from_code(&row.get::<String>(SearchShareRow::Kind)?)
        .ok_or(StoreError::Corrupt("a kind of share"))?;
    let target = match kind {
        ShareKind::Group => ShareTarget::Group(row.id::<GroupId>(SearchShareRow::Target)?),
        ShareKind::Project => ShareTarget::Project(row.id::<ProjectId>(SearchShareRow::Target)?),
    };
    Ok(SearchShare {
        search: row.id::<SavedSearchId>(SearchShareRow::Search)?,
        target,
        shared_at: row.time(SearchShareRow::SharedAt)?,
    })
}

/// The ID of the group or project a share is with, as stored.
fn target_id(target: ShareTarget) -> [u8; 16] {
    match target {
        ShareTarget::Group(group) => group.to_stored(),
        ShareTarget::Project(project) => project.to_stored(),
    }
}

fn stored_ids<T: StoredId + Copy>(ids: &[T]) -> Vec<Vec<u8>> {
    ids.iter().map(|id| id.stored()).collect()
}

/// The shares with a place that reaches a person: any of their groups or projects they see.
fn shares_reaching(reach: &Reach<'_>) -> SelectStatement {
    let mut places = Cond::any();
    if !reach.groups.is_empty() {
        places = places.add(
            Cond::all()
                .add(Expr::col(SearchShareRow::Kind).eq(ShareKind::Group.code()))
                .add(Expr::col(SearchShareRow::Target).is_in(stored_ids(reach.groups))),
        );
    }
    if !reach.projects.is_empty() {
        places = places.add(
            Cond::all()
                .add(Expr::col(SearchShareRow::Kind).eq(ShareKind::Project.code()))
                .add(Expr::col(SearchShareRow::Target).is_in(stored_ids(reach.projects))),
        );
    }
    let mut select = Query::select();
    select
        .column(SearchShareRow::Search)
        .from(SearchShareRow::Table)
        .cond_where(places);
    select
}

impl SavedSearchStore for AuthStorage {
    async fn saved_search_in(
        transaction: &mut Transaction,
        id: SavedSearchId,
    ) -> Result<Option<SavedSearch>, StoreError> {
        let mut select = search_select();
        select.and_where(Expr::col(SavedSearchRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_search)
            .transpose()
    }

    async fn search_columns_in(
        transaction: &mut Transaction,
        id: SavedSearchId,
    ) -> Result<Vec<SearchColumn>, StoreError> {
        let mut select = column_select();
        select
            .and_where(Expr::col(SearchColumnRow::Search).eq(id.stored()))
            .order_by(SearchColumnRow::Position, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_column)
            .collect()
    }

    async fn search_shares_in(
        transaction: &mut Transaction,
        id: SavedSearchId,
    ) -> Result<Vec<SearchShare>, StoreError> {
        let mut select = share_select();
        select
            .and_where(Expr::col(SearchShareRow::Search).eq(id.stored()))
            .order_by(SearchShareRow::SharedAt, Order::Asc)
            .order_by(SearchShareRow::Kind, Order::Asc)
            .order_by(SearchShareRow::Target, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_share)
            .collect()
    }

    async fn project_default_list_in(
        transaction: &mut Transaction,
        project: ProjectId,
    ) -> Result<Option<ProjectDefaultList>, StoreError> {
        let mut select = Query::select();
        select
            .columns([
                ProjectDefaultListRow::Project,
                ProjectDefaultListRow::Search,
            ])
            .from(ProjectDefaultListRow::Table)
            .and_where(Expr::col(ProjectDefaultListRow::Project).eq(project.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(|row| {
                Ok(ProjectDefaultList {
                    project: row.id::<ProjectId>(ProjectDefaultListRow::Project)?,
                    search: row.id::<SavedSearchId>(ProjectDefaultListRow::Search)?,
                })
            })
            .transpose()
    }

    async fn saved_searches_made_by(
        &self,
        maker: UserId,
        after: Option<SavedSearchId>,
        limit: u64,
    ) -> Result<Vec<SavedSearch>, StoreError> {
        let mut select = search_select();
        select
            .and_where(Expr::col(SavedSearchRow::Maker).eq(maker.stored()))
            .and_where(Expr::col(SavedSearchRow::DeletedAt).is_null())
            .order_by(SavedSearchRow::Id, Order::Asc)
            .limit(limit);
        if let Some(after) = after {
            select.and_where(Expr::col(SavedSearchRow::Id).gt(after.stored()));
        }
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_search)
            .collect()
    }

    async fn saved_searches_shared_with(
        &self,
        reach: Reach<'_>,
        after: Option<SavedSearchId>,
        skip: u64,
        limit: u64,
    ) -> Result<Vec<SavedSearch>, StoreError> {
        if reach.groups.is_empty() && reach.projects.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = search_select();
        select
            .and_where(Expr::col(SavedSearchRow::Maker).ne(reach.person.stored()))
            .and_where(Expr::col(SavedSearchRow::DeletedAt).is_null())
            .and_where(Expr::col(SavedSearchRow::Id).in_subquery(shares_reaching(&reach)))
            .order_by(SavedSearchRow::Id, Order::Asc)
            .offset(skip)
            .limit(limit);
        if let Some(after) = after {
            select.and_where(Expr::col(SavedSearchRow::Id).gt(after.stored()));
        }
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_search)
            .collect()
    }

    async fn count_saved_searches_of_in(
        transaction: &mut Transaction,
        maker: UserId,
    ) -> Result<u64, StoreError> {
        let mut select = Query::select();
        select
            .expr_as(
                Expr::col(SavedSearchRow::Id).count(),
                crate::schema::Computed::Total,
            )
            .from(SavedSearchRow::Table)
            .and_where(Expr::col(SavedSearchRow::Maker).eq(maker.stored()));
        let total: i64 = match transaction.fetch_optional(&select).await? {
            Some(row) => row.get(crate::schema::Computed::Total)?,
            None => 0,
        };
        u64::try_from(total).map_err(|_| StoreError::Corrupt("a count"))
    }

    async fn columns_of_searches(
        &self,
        ids: &[SavedSearchId],
    ) -> Result<Vec<SearchColumn>, StoreError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = column_select();
        select
            .and_where(Expr::col(SearchColumnRow::Search).is_in(stored_ids(ids)))
            .order_by(SearchColumnRow::Search, Order::Asc)
            .order_by(SearchColumnRow::Position, Order::Asc);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_column)
            .collect()
    }

    async fn shares_of_searches(
        &self,
        ids: &[SavedSearchId],
    ) -> Result<Vec<SearchShare>, StoreError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = share_select();
        select
            .and_where(Expr::col(SearchShareRow::Search).is_in(stored_ids(ids)))
            .order_by(SearchShareRow::Search, Order::Asc)
            .order_by(SearchShareRow::SharedAt, Order::Asc)
            .order_by(SearchShareRow::Kind, Order::Asc)
            .order_by(SearchShareRow::Target, Order::Asc);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_share)
            .collect()
    }

    async fn searches_shared_with_group(
        &self,
        group: GroupId,
    ) -> Result<Vec<SavedSearchId>, StoreError> {
        let mut select = Query::select();
        select
            .column(SearchShareRow::Search)
            .from(SearchShareRow::Table)
            .and_where(Expr::col(SearchShareRow::Kind).eq(ShareKind::Group.code()))
            .and_where(Expr::col(SearchShareRow::Target).eq(group.stored()))
            .order_by(SearchShareRow::Search, Order::Asc);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| row.id::<SavedSearchId>(SearchShareRow::Search))
            .collect()
    }

    async fn makers_of(&self, ids: &[UserId]) -> Result<Vec<Person>, StoreError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = Query::select();
        select
            .columns(person_columns())
            .from(PersonRow::Table)
            .and_where(Expr::col(PersonRow::Id).is_in(stored_ids(ids)));
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_person)
            .collect()
    }

    async fn lock_saved_searches_of(
        transaction: &mut Transaction,
        maker: UserId,
    ) -> Result<(), StoreError> {
        transaction
            .advisory_lock(LockClass::SavedSearches, lock_key(maker.to_bytes()))
            .await
    }

    async fn insert_saved_search(
        transaction: &mut Transaction,
        search: &SavedSearch,
        columns: &[SearchColumn],
    ) -> Result<(), StoreError> {
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
                SavedSearchRow::GroupBy,
                SavedSearchRow::CreatedAt,
                SavedSearchRow::UpdatedAt,
                SavedSearchRow::DeletedAt,
                SavedSearchRow::Version,
            ])
            .values([
                Expr::val(search.id.stored()),
                Expr::val(search.name.as_str()),
                Expr::val(search.text.as_str()),
                Expr::val(search.maker.stored()),
                Expr::val(search.view.code()),
                Expr::val(search.spacing.code()),
                Expr::val(search.group_by.clone()),
                Expr::val(to_micros(search.created_at)),
                Expr::val(to_micros(search.updated_at)),
                Expr::val(search.deleted_at.map(to_micros)),
                Expr::val(search.version),
            ])
            .map_err(|_| StoreError::Corrupt("a saved search"))?;
        transaction.execute(&insert).await?;
        insert_columns(transaction, columns).await
    }

    async fn update_saved_search(
        transaction: &mut Transaction,
        search: &SavedSearch,
        columns: &[SearchColumn],
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(SavedSearchRow::Table)
            .value(SavedSearchRow::Name, search.name.as_str())
            .value(SavedSearchRow::Text, search.text.as_str())
            .value(SavedSearchRow::View, search.view.code())
            .value(SavedSearchRow::Spacing, search.spacing.code())
            .value(SavedSearchRow::GroupBy, search.group_by.clone())
            .value(SavedSearchRow::UpdatedAt, to_micros(search.updated_at))
            .value(SavedSearchRow::Version, search.version)
            .and_where(Expr::col(SavedSearchRow::Id).eq(search.id.stored()))
            .and_where(Expr::col(SavedSearchRow::Version).eq(version));
        if transaction.execute(&update).await? == 0 {
            return Ok(false);
        }
        let mut delete = Query::delete();
        delete
            .from_table(SearchColumnRow::Table)
            .and_where(Expr::col(SearchColumnRow::Search).eq(search.id.stored()));
        transaction.execute(&delete).await?;
        insert_columns(transaction, columns).await?;
        Ok(true)
    }

    async fn set_saved_search_deleted(
        transaction: &mut Transaction,
        id: SavedSearchId,
        deleted_at: Option<Timestamp>,
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(SavedSearchRow::Table)
            .value(SavedSearchRow::DeletedAt, deleted_at.map(to_micros))
            .value(SavedSearchRow::Version, version.saturating_add(1))
            .and_where(Expr::col(SavedSearchRow::Id).eq(id.stored()))
            .and_where(Expr::col(SavedSearchRow::Version).eq(version));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn insert_search_share(
        transaction: &mut Transaction,
        share: &SearchShare,
    ) -> Result<bool, StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(SearchShareRow::Table)
            .columns([
                SearchShareRow::Search,
                SearchShareRow::Kind,
                SearchShareRow::Target,
                SearchShareRow::SharedAt,
            ])
            .values([
                Expr::val(share.search.stored()),
                Expr::val(share.target.kind().code()),
                Expr::val(target_id(share.target).to_vec()),
                Expr::val(to_micros(share.shared_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a share of a saved search"))?
            .on_conflict(
                sea_query::OnConflict::columns([
                    SearchShareRow::Search,
                    SearchShareRow::Kind,
                    SearchShareRow::Target,
                ])
                .do_nothing()
                .to_owned(),
            );
        Ok(transaction.execute(&insert).await? > 0)
    }

    async fn delete_search_share(
        transaction: &mut Transaction,
        search: SavedSearchId,
        target: ShareTarget,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(SearchShareRow::Table)
            .and_where(Expr::col(SearchShareRow::Search).eq(search.stored()))
            .and_where(Expr::col(SearchShareRow::Kind).eq(target.kind().code()))
            .and_where(Expr::col(SearchShareRow::Target).eq(target_id(target).to_vec()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn put_project_default_list(
        transaction: &mut Transaction,
        list: &ProjectDefaultList,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(ProjectDefaultListRow::Table)
            .columns([
                ProjectDefaultListRow::Project,
                ProjectDefaultListRow::Search,
            ])
            .values([
                Expr::val(list.project.stored()),
                Expr::val(list.search.stored()),
            ])
            .map_err(|_| StoreError::Corrupt("the list of a project"))?
            .on_conflict(
                sea_query::OnConflict::column(ProjectDefaultListRow::Project)
                    .update_column(ProjectDefaultListRow::Search)
                    .to_owned(),
            );
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn clear_project_default_list(
        transaction: &mut Transaction,
        project: ProjectId,
        search: Option<SavedSearchId>,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(ProjectDefaultListRow::Table)
            .and_where(Expr::col(ProjectDefaultListRow::Project).eq(project.stored()));
        if let Some(search) = search {
            delete.and_where(Expr::col(ProjectDefaultListRow::Search).eq(search.stored()));
        }
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn delete_shares_with_group(
        transaction: &mut Transaction,
        group: GroupId,
    ) -> Result<Vec<SavedSearchId>, StoreError> {
        let mut select = Query::select();
        select
            .column(SearchShareRow::Search)
            .from(SearchShareRow::Table)
            .and_where(Expr::col(SearchShareRow::Kind).eq(ShareKind::Group.code()))
            .and_where(Expr::col(SearchShareRow::Target).eq(group.stored()))
            .order_by(SearchShareRow::Search, Order::Asc);
        let gone = transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| row.id::<SavedSearchId>(SearchShareRow::Search))
            .collect::<Result<Vec<_>, _>>()?;
        let mut delete = Query::delete();
        delete
            .from_table(SearchShareRow::Table)
            .and_where(Expr::col(SearchShareRow::Kind).eq(ShareKind::Group.code()))
            .and_where(Expr::col(SearchShareRow::Target).eq(group.stored()));
        transaction.execute(&delete).await?;
        Ok(gone)
    }

    async fn delete_private_searches_of(
        transaction: &mut Transaction,
        maker: UserId,
    ) -> Result<Vec<SavedSearchId>, StoreError> {
        let mut shared = Query::select();
        shared
            .column(SearchShareRow::Search)
            .from(SearchShareRow::Table);
        let mut select = Query::select();
        select
            .column(SavedSearchRow::Id)
            .from(SavedSearchRow::Table)
            .and_where(Expr::col(SavedSearchRow::Maker).eq(maker.stored()))
            .and_where(Expr::col(SavedSearchRow::Id).not_in_subquery(shared))
            .order_by(SavedSearchRow::Id, Order::Asc);
        let private = transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| row.id::<SavedSearchId>(SavedSearchRow::Id))
            .collect::<Result<Vec<_>, _>>()?;
        if private.is_empty() {
            return Ok(private);
        }
        let mut columns = Query::delete();
        columns
            .from_table(SearchColumnRow::Table)
            .and_where(Expr::col(SearchColumnRow::Search).is_in(stored_ids(&private)));
        transaction.execute(&columns).await?;
        let mut searches = Query::delete();
        searches
            .from_table(SavedSearchRow::Table)
            .and_where(Expr::col(SavedSearchRow::Id).is_in(stored_ids(&private)));
        transaction.execute(&searches).await?;
        Ok(private)
    }
}

/// Writes the columns of a saved search.
async fn insert_columns(
    transaction: &mut Transaction,
    columns: &[SearchColumn],
) -> Result<(), StoreError> {
    for column in columns {
        let mut insert = Query::insert();
        insert
            .into_table(SearchColumnRow::Table)
            .columns([
                SearchColumnRow::Search,
                SearchColumnRow::Position,
                SearchColumnRow::Field,
                SearchColumnRow::Width,
            ])
            .values([
                Expr::val(column.search.stored()),
                Expr::val(column.position),
                Expr::val(column.field.as_str()),
                Expr::val(column.width),
            ])
            .map_err(|_| StoreError::Corrupt("a column of a saved search"))?;
        transaction.execute(&insert).await?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/saved_searches.rs"]
mod tests;
