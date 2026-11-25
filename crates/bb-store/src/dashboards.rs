//! The adapter of the dashboard port (`bb_app::dashboards::DashboardStore`): dashboards, their
//! boxes and whom they are shared with (CFG-03).

use bb_app::dashboards::DashboardStore;
use bb_app::saved_searches::Reach;
use bb_core::dashboard::{
    BoxKind, BoxOptions, BoxPlace, BoxSource, BoxSpec, ChartShape, Dashboard, DashboardBox,
    DashboardShare, TimeSpan,
};
use bb_core::id::{DashboardBoxId, DashboardId, GroupId, ProjectId, SavedSearchId, UserId};
use bb_core::saved_search::{ShareKind, ShareTarget};
use jiff::Timestamp;
use sea_query::{Cond, Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::locks::{LockClass, lock_key};
use crate::schema::{Computed, DashboardBoxRow, DashboardRow, DashboardShareRow};

fn dashboard_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            DashboardRow::Id,
            DashboardRow::Name,
            DashboardRow::Maker,
            DashboardRow::Home,
            DashboardRow::CreatedAt,
            DashboardRow::UpdatedAt,
            DashboardRow::DeletedAt,
            DashboardRow::Version,
        ])
        .from(DashboardRow::Table);
    select
}

fn read_dashboard(row: &Row) -> Result<Dashboard, StoreError> {
    Ok(Dashboard {
        id: row.id::<DashboardId>(DashboardRow::Id)?,
        name: row.get(DashboardRow::Name)?,
        maker: row.id::<UserId>(DashboardRow::Maker)?,
        home: row.get(DashboardRow::Home)?,
        created_at: row.time(DashboardRow::CreatedAt)?,
        updated_at: row.time(DashboardRow::UpdatedAt)?,
        deleted_at: row.optional_time(DashboardRow::DeletedAt)?,
        version: row.get(DashboardRow::Version)?,
    })
}

const BOX_COLUMNS: [DashboardBoxRow; 14] = [
    DashboardBoxRow::Dashboard,
    DashboardBoxRow::Id,
    DashboardBoxRow::Position,
    DashboardBoxRow::Kind,
    DashboardBoxRow::GridColumn,
    DashboardBoxRow::GridRow,
    DashboardBoxRow::Width,
    DashboardBoxRow::Height,
    DashboardBoxRow::Title,
    DashboardBoxRow::SearchText,
    DashboardBoxRow::SavedSearch,
    DashboardBoxRow::ChartBy,
    DashboardBoxRow::ChartShape,
    DashboardBoxRow::TimeSpan,
];

/// A whole number of the grid as it is stored.
fn grid(row: &Row, column: DashboardBoxRow) -> Result<u32, StoreError> {
    u32::try_from(row.get::<i64>(column)?).map_err(|_| StoreError::Corrupt("a place on a grid"))
}

fn read_box(row: &Row) -> Result<DashboardBox, StoreError> {
    let text: Option<String> = row.get(DashboardBoxRow::SearchText)?;
    let saved = row
        .get::<Option<Vec<u8>>>(DashboardBoxRow::SavedSearch)?
        .map(|_| row.id::<SavedSearchId>(DashboardBoxRow::SavedSearch))
        .transpose()?;
    let source = match (text, saved) {
        (Some(text), None) => Some(BoxSource::Text(text)),
        (None, Some(saved)) => Some(BoxSource::Saved(saved)),
        (None, None) => None,
        (Some(_), Some(_)) => return Err(StoreError::Corrupt("the source of a box")),
    };
    let word = |column| -> Result<Option<String>, StoreError> { row.get(column) };
    Ok(DashboardBox {
        dashboard: row.id::<DashboardId>(DashboardBoxRow::Dashboard)?,
        id: row.id::<DashboardBoxId>(DashboardBoxRow::Id)?,
        position: row.get(DashboardBoxRow::Position)?,
        spec: BoxSpec {
            kind: BoxKind::from_code(&row.get::<String>(DashboardBoxRow::Kind)?)
                .ok_or(StoreError::Corrupt("a kind of box"))?,
            place: BoxPlace {
                column: grid(row, DashboardBoxRow::GridColumn)?,
                row: grid(row, DashboardBoxRow::GridRow)?,
                width: grid(row, DashboardBoxRow::Width)?,
                height: grid(row, DashboardBoxRow::Height)?,
            },
            title: row.get(DashboardBoxRow::Title)?,
            source,
            options: BoxOptions {
                by: word(DashboardBoxRow::ChartBy)?,
                shape: word(DashboardBoxRow::ChartShape)?
                    .map(|code| {
                        ChartShape::from_code(&code).ok_or(StoreError::Corrupt("a shape of chart"))
                    })
                    .transpose()?,
                span: word(DashboardBoxRow::TimeSpan)?
                    .map(|code| {
                        TimeSpan::from_code(&code).ok_or(StoreError::Corrupt("a time span"))
                    })
                    .transpose()?,
            },
        },
    })
}

fn share_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            DashboardShareRow::Dashboard,
            DashboardShareRow::Kind,
            DashboardShareRow::Target,
            DashboardShareRow::SharedAt,
        ])
        .from(DashboardShareRow::Table);
    select
}

fn read_share(row: &Row) -> Result<DashboardShare, StoreError> {
    let kind = ShareKind::from_code(&row.get::<String>(DashboardShareRow::Kind)?)
        .ok_or(StoreError::Corrupt("a kind of share"))?;
    let target = match kind {
        ShareKind::Group => ShareTarget::Group(row.id::<GroupId>(DashboardShareRow::Target)?),
        ShareKind::Project => ShareTarget::Project(row.id::<ProjectId>(DashboardShareRow::Target)?),
    };
    Ok(DashboardShare {
        dashboard: row.id::<DashboardId>(DashboardShareRow::Dashboard)?,
        target,
        shared_at: row.time(DashboardShareRow::SharedAt)?,
    })
}

/// The ID of the group or project a share is with, as stored.
fn target_id(target: ShareTarget) -> Vec<u8> {
    match target {
        ShareTarget::Group(group) => group.stored(),
        ShareTarget::Project(project) => project.stored(),
    }
}

fn stored_ids<T: StoredId + Copy>(ids: &[T]) -> Vec<Vec<u8>> {
    ids.iter().map(|id| id.stored()).collect()
}

/// The dashboards shared with a place that reaches a person: any of their groups or projects they
/// see.
fn shares_reaching(reach: &Reach<'_>) -> SelectStatement {
    let mut places = Cond::any();
    if !reach.groups.is_empty() {
        places = places.add(
            Cond::all()
                .add(Expr::col(DashboardShareRow::Kind).eq(ShareKind::Group.code()))
                .add(Expr::col(DashboardShareRow::Target).is_in(stored_ids(reach.groups))),
        );
    }
    if !reach.projects.is_empty() {
        places = places.add(
            Cond::all()
                .add(Expr::col(DashboardShareRow::Kind).eq(ShareKind::Project.code()))
                .add(Expr::col(DashboardShareRow::Target).is_in(stored_ids(reach.projects))),
        );
    }
    let mut select = Query::select();
    select
        .column(DashboardShareRow::Dashboard)
        .from(DashboardShareRow::Table)
        .cond_where(places);
    select
}

/// The dashboards that have a share.
fn shared() -> SelectStatement {
    let mut select = Query::select();
    select
        .column(DashboardShareRow::Dashboard)
        .from(DashboardShareRow::Table);
    select
}

/// The dashboards shared with a group, by ID.
fn shared_with_group(group: GroupId) -> SelectStatement {
    let mut select = Query::select();
    select
        .column(DashboardShareRow::Dashboard)
        .from(DashboardShareRow::Table)
        .and_where(Expr::col(DashboardShareRow::Kind).eq(ShareKind::Group.code()))
        .and_where(Expr::col(DashboardShareRow::Target).eq(group.stored()))
        .order_by(DashboardShareRow::Dashboard, Order::Asc);
    select
}

/// The IDs of the dashboards in rows of shares.
fn dashboard_ids(rows: &[Row]) -> Result<Vec<DashboardId>, StoreError> {
    rows.iter()
        .map(|row| row.id::<DashboardId>(DashboardShareRow::Dashboard))
        .collect()
}

/// How many dashboards of `maker` there are, among those `narrowed` leaves.
async fn count_of(
    transaction: &mut Transaction,
    maker: UserId,
    narrowed: impl FnOnce(&mut SelectStatement),
) -> Result<u64, StoreError> {
    let mut select = Query::select();
    select
        .expr_as(Expr::col(DashboardRow::Id).count(), Computed::Total)
        .from(DashboardRow::Table)
        .and_where(Expr::col(DashboardRow::Maker).eq(maker.stored()));
    narrowed(&mut select);
    let total: i64 = match transaction.fetch_optional(&select).await? {
        Some(row) => row.get(Computed::Total)?,
        None => 0,
    };
    u64::try_from(total).map_err(|_| StoreError::Corrupt("a count"))
}

/// Writes the boxes of a dashboard.
async fn insert_boxes(
    transaction: &mut Transaction,
    boxes: &[DashboardBox],
) -> Result<(), StoreError> {
    for each in boxes {
        let spec = &each.spec;
        let (text, saved) = match &spec.source {
            Some(BoxSource::Text(text)) => (Some(text.clone()), None),
            Some(BoxSource::Saved(saved)) => (None, Some(saved.stored())),
            None => (None, None),
        };
        let mut insert = Query::insert();
        insert
            .into_table(DashboardBoxRow::Table)
            .columns(BOX_COLUMNS)
            .values([
                Expr::val(each.dashboard.stored()),
                Expr::val(each.id.stored()),
                Expr::val(each.position),
                Expr::val(spec.kind.code()),
                Expr::val(i64::from(spec.place.column)),
                Expr::val(i64::from(spec.place.row)),
                Expr::val(i64::from(spec.place.width)),
                Expr::val(i64::from(spec.place.height)),
                Expr::val(spec.title.as_str()),
                Expr::val(text),
                Expr::val(saved),
                Expr::val(spec.options.by.clone()),
                Expr::val(spec.options.shape.map(ChartShape::code)),
                Expr::val(spec.options.span.map(TimeSpan::code)),
            ])
            .map_err(|_| StoreError::Corrupt("a box of a dashboard"))?;
        transaction.execute(&insert).await?;
    }
    Ok(())
}

/// Deletes the boxes of these dashboards.
async fn delete_boxes(
    transaction: &mut Transaction,
    ids: &[DashboardId],
) -> Result<(), StoreError> {
    let mut delete = Query::delete();
    delete
        .from_table(DashboardBoxRow::Table)
        .and_where(Expr::col(DashboardBoxRow::Dashboard).is_in(stored_ids(ids)));
    transaction.execute(&delete).await.map(|_| ())
}

impl DashboardStore for AuthStorage {
    async fn dashboard_in(
        transaction: &mut Transaction,
        id: DashboardId,
    ) -> Result<Option<Dashboard>, StoreError> {
        let mut select = dashboard_select();
        select.and_where(Expr::col(DashboardRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_dashboard)
            .transpose()
    }

    async fn dashboard_boxes_in(
        transaction: &mut Transaction,
        id: DashboardId,
    ) -> Result<Vec<DashboardBox>, StoreError> {
        let mut select = Query::select();
        select
            .columns(BOX_COLUMNS)
            .from(DashboardBoxRow::Table)
            .and_where(Expr::col(DashboardBoxRow::Dashboard).eq(id.stored()))
            .order_by(DashboardBoxRow::Position, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_box)
            .collect()
    }

    async fn dashboard_shares_in(
        transaction: &mut Transaction,
        id: DashboardId,
    ) -> Result<Vec<DashboardShare>, StoreError> {
        let mut select = share_select();
        select
            .and_where(Expr::col(DashboardShareRow::Dashboard).eq(id.stored()))
            .order_by(DashboardShareRow::SharedAt, Order::Asc)
            .order_by(DashboardShareRow::Kind, Order::Asc)
            .order_by(DashboardShareRow::Target, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_share)
            .collect()
    }

    async fn home_of_in(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Option<Dashboard>, StoreError> {
        let mut select = dashboard_select();
        select
            .and_where(Expr::col(DashboardRow::Maker).eq(person.stored()))
            .and_where(Expr::col(DashboardRow::Home).eq(true))
            .order_by(DashboardRow::Id, Order::Asc)
            .limit(1);
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_dashboard)
            .transpose()
    }

    async fn count_dashboards_of_in(
        transaction: &mut Transaction,
        maker: UserId,
    ) -> Result<u64, StoreError> {
        count_of(transaction, maker, |_| ()).await
    }

    async fn count_shared_dashboards_of_in(
        transaction: &mut Transaction,
        maker: UserId,
    ) -> Result<u64, StoreError> {
        count_of(transaction, maker, |select| {
            select.and_where(Expr::col(DashboardRow::Id).in_subquery(shared()));
        })
        .await
    }

    async fn dashboards_made_by(
        &self,
        maker: UserId,
        after: Option<DashboardId>,
        limit: u64,
    ) -> Result<Vec<Dashboard>, StoreError> {
        let mut select = dashboard_select();
        select
            .and_where(Expr::col(DashboardRow::Maker).eq(maker.stored()))
            .and_where(Expr::col(DashboardRow::DeletedAt).is_null())
            .and_where(Expr::col(DashboardRow::Home).eq(false))
            .order_by(DashboardRow::Id, Order::Asc)
            .limit(limit);
        if let Some(after) = after {
            select.and_where(Expr::col(DashboardRow::Id).gt(after.stored()));
        }
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_dashboard)
            .collect()
    }

    async fn dashboards_shared_with(
        &self,
        reach: Reach<'_>,
        after: Option<DashboardId>,
        limit: u64,
    ) -> Result<Vec<Dashboard>, StoreError> {
        if reach.groups.is_empty() && reach.projects.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = dashboard_select();
        select
            .and_where(Expr::col(DashboardRow::Maker).ne(reach.person.stored()))
            .and_where(Expr::col(DashboardRow::DeletedAt).is_null())
            .and_where(Expr::col(DashboardRow::Id).in_subquery(shares_reaching(&reach)))
            .order_by(DashboardRow::Id, Order::Asc)
            .limit(limit);
        if let Some(after) = after {
            select.and_where(Expr::col(DashboardRow::Id).gt(after.stored()));
        }
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_dashboard)
            .collect()
    }

    async fn shares_of_dashboards(
        &self,
        ids: &[DashboardId],
    ) -> Result<Vec<DashboardShare>, StoreError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = share_select();
        select
            .and_where(Expr::col(DashboardShareRow::Dashboard).is_in(stored_ids(ids)))
            .order_by(DashboardShareRow::Dashboard, Order::Asc)
            .order_by(DashboardShareRow::SharedAt, Order::Asc);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_share)
            .collect()
    }

    async fn dashboards_shared_with_group(
        &self,
        group: GroupId,
    ) -> Result<Vec<DashboardId>, StoreError> {
        dashboard_ids(&self.database().fetch_all(&shared_with_group(group)).await?)
    }

    async fn lock_dashboards_of(
        transaction: &mut Transaction,
        maker: UserId,
    ) -> Result<(), StoreError> {
        transaction
            .advisory_lock(LockClass::Dashboards, lock_key(maker.to_bytes()))
            .await
    }

    async fn insert_dashboard(
        transaction: &mut Transaction,
        dashboard: &Dashboard,
        boxes: &[DashboardBox],
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(DashboardRow::Table)
            .columns([
                DashboardRow::Id,
                DashboardRow::Name,
                DashboardRow::Maker,
                DashboardRow::Home,
                DashboardRow::CreatedAt,
                DashboardRow::UpdatedAt,
                DashboardRow::DeletedAt,
                DashboardRow::Version,
            ])
            .values([
                Expr::val(dashboard.id.stored()),
                Expr::val(dashboard.name.as_str()),
                Expr::val(dashboard.maker.stored()),
                Expr::val(dashboard.home),
                Expr::val(to_micros(dashboard.created_at)),
                Expr::val(to_micros(dashboard.updated_at)),
                Expr::val(dashboard.deleted_at.map(to_micros)),
                Expr::val(dashboard.version),
            ])
            .map_err(|_| StoreError::Corrupt("a dashboard"))?;
        transaction.execute(&insert).await?;
        insert_boxes(transaction, boxes).await
    }

    async fn update_dashboard(
        transaction: &mut Transaction,
        dashboard: &Dashboard,
        boxes: &[DashboardBox],
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(DashboardRow::Table)
            .value(DashboardRow::Name, dashboard.name.as_str())
            .value(DashboardRow::UpdatedAt, to_micros(dashboard.updated_at))
            .value(DashboardRow::Version, dashboard.version)
            .and_where(Expr::col(DashboardRow::Id).eq(dashboard.id.stored()))
            .and_where(Expr::col(DashboardRow::Version).eq(version));
        if transaction.execute(&update).await? == 0 {
            return Ok(false);
        }
        delete_boxes(transaction, &[dashboard.id]).await?;
        insert_boxes(transaction, boxes).await?;
        Ok(true)
    }

    async fn set_dashboard_deleted(
        transaction: &mut Transaction,
        id: DashboardId,
        deleted_at: Option<Timestamp>,
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(DashboardRow::Table)
            .value(DashboardRow::DeletedAt, deleted_at.map(to_micros))
            .value(DashboardRow::Version, version.saturating_add(1))
            .and_where(Expr::col(DashboardRow::Id).eq(id.stored()))
            .and_where(Expr::col(DashboardRow::Version).eq(version));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn insert_dashboard_share(
        transaction: &mut Transaction,
        share: &DashboardShare,
    ) -> Result<bool, StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(DashboardShareRow::Table)
            .columns([
                DashboardShareRow::Dashboard,
                DashboardShareRow::Kind,
                DashboardShareRow::Target,
                DashboardShareRow::SharedAt,
            ])
            .values([
                Expr::val(share.dashboard.stored()),
                Expr::val(share.target.kind().code()),
                Expr::val(target_id(share.target)),
                Expr::val(to_micros(share.shared_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a share of a dashboard"))?
            .on_conflict(
                sea_query::OnConflict::columns([
                    DashboardShareRow::Dashboard,
                    DashboardShareRow::Kind,
                    DashboardShareRow::Target,
                ])
                .do_nothing()
                .to_owned(),
            );
        Ok(transaction.execute(&insert).await? > 0)
    }

    async fn delete_dashboard_share(
        transaction: &mut Transaction,
        dashboard: DashboardId,
        target: ShareTarget,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(DashboardShareRow::Table)
            .and_where(Expr::col(DashboardShareRow::Dashboard).eq(dashboard.stored()))
            .and_where(Expr::col(DashboardShareRow::Kind).eq(target.kind().code()))
            .and_where(Expr::col(DashboardShareRow::Target).eq(target_id(target)));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn delete_dashboard_shares_with_group(
        transaction: &mut Transaction,
        group: GroupId,
    ) -> Result<Vec<DashboardId>, StoreError> {
        let gone = dashboard_ids(&transaction.fetch_all(&shared_with_group(group)).await?)?;
        let mut delete = Query::delete();
        delete
            .from_table(DashboardShareRow::Table)
            .and_where(Expr::col(DashboardShareRow::Kind).eq(ShareKind::Group.code()))
            .and_where(Expr::col(DashboardShareRow::Target).eq(group.stored()));
        transaction.execute(&delete).await?;
        Ok(gone)
    }

    async fn delete_private_dashboards_of(
        transaction: &mut Transaction,
        maker: UserId,
    ) -> Result<Vec<DashboardId>, StoreError> {
        let mut select = Query::select();
        select
            .column(DashboardRow::Id)
            .from(DashboardRow::Table)
            .and_where(Expr::col(DashboardRow::Maker).eq(maker.stored()))
            .and_where(Expr::col(DashboardRow::Id).not_in_subquery(shared()))
            .order_by(DashboardRow::Id, Order::Asc);
        let private = transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| row.id::<DashboardId>(DashboardRow::Id))
            .collect::<Result<Vec<_>, _>>()?;
        if private.is_empty() {
            return Ok(private);
        }
        delete_boxes(transaction, &private).await?;
        let mut delete = Query::delete();
        delete
            .from_table(DashboardRow::Table)
            .and_where(Expr::col(DashboardRow::Id).is_in(stored_ids(&private)));
        transaction.execute(&delete).await?;
        Ok(private)
    }
}
