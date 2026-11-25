//! The dashboard part of the in-memory store (CFG-03).

use bb_core::dashboard::{Dashboard, DashboardBox, DashboardShare};
use bb_core::id::{DashboardId, GroupId, UserId};
use bb_core::saved_search::ShareTarget;
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, Tx};
use crate::dashboards::DashboardStore;
use crate::saved_searches::Reach;

/// The dashboards of the in-memory store, with their boxes and shares.
#[derive(Clone, Default)]
pub(crate) struct Dashboards {
    pub(crate) records: Vec<Dashboard>,
    pub(crate) boxes: Vec<DashboardBox>,
    pub(crate) shares: Vec<DashboardShare>,
}

impl Dashboards {
    fn shared(&self, id: DashboardId) -> bool {
        self.shares.iter().any(|share| share.dashboard == id)
    }

    fn reaches(&self, id: DashboardId, reach: &Reach<'_>) -> bool {
        self.shares
            .iter()
            .filter(|share| share.dashboard == id)
            .any(|share| match share.target {
                ShareTarget::Group(group) => reach.groups.contains(&group),
                ShareTarget::Project(project) => reach.projects.contains(&project),
            })
    }
}

/// The dashboards of `found` after `after`, by ID, at most `limit`.
fn page(mut found: Vec<Dashboard>, after: Option<DashboardId>, limit: u64) -> Vec<Dashboard> {
    found.sort_by_key(|dashboard| dashboard.id);
    found
        .into_iter()
        .filter(|dashboard| after.is_none_or(|after| dashboard.id > after))
        .take(usize::try_from(limit).unwrap_or(usize::MAX))
        .collect()
}

fn count(rows: impl Iterator<Item = ()>) -> u64 {
    u64::try_from(rows.count()).unwrap_or(u64::MAX)
}

impl DashboardStore for FakeStore {
    async fn dashboard_in(tx: &mut Tx, id: DashboardId) -> Result<Option<Dashboard>, FakeError> {
        Ok(tx
            .state
            .dashboards
            .records
            .iter()
            .find(|dashboard| dashboard.id == id)
            .cloned())
    }

    async fn dashboard_boxes_in(
        tx: &mut Tx,
        id: DashboardId,
    ) -> Result<Vec<DashboardBox>, FakeError> {
        let mut found: Vec<DashboardBox> = tx
            .state
            .dashboards
            .boxes
            .iter()
            .filter(|each| each.dashboard == id)
            .cloned()
            .collect();
        found.sort_by_key(|each| each.position);
        Ok(found)
    }

    async fn dashboard_shares_in(
        tx: &mut Tx,
        id: DashboardId,
    ) -> Result<Vec<DashboardShare>, FakeError> {
        let mut found: Vec<DashboardShare> = tx
            .state
            .dashboards
            .shares
            .iter()
            .filter(|share| share.dashboard == id)
            .cloned()
            .collect();
        found.sort_by_key(|share| (share.shared_at, share.target));
        Ok(found)
    }

    async fn home_of_in(tx: &mut Tx, person: UserId) -> Result<Option<Dashboard>, FakeError> {
        Ok(tx
            .state
            .dashboards
            .records
            .iter()
            .find(|dashboard| dashboard.home && dashboard.maker == person)
            .cloned())
    }

    async fn count_dashboards_of_in(tx: &mut Tx, maker: UserId) -> Result<u64, FakeError> {
        Ok(count(
            tx.state
                .dashboards
                .records
                .iter()
                .filter(|dashboard| dashboard.maker == maker && dashboard.deleted_at.is_none())
                .map(|_| ()),
        ))
    }

    async fn count_shared_dashboards_of_in(tx: &mut Tx, maker: UserId) -> Result<u64, FakeError> {
        let all = &tx.state.dashboards;
        Ok(count(
            all.records
                .iter()
                .filter(|dashboard| dashboard.maker == maker && all.shared(dashboard.id))
                .map(|_| ()),
        ))
    }

    async fn dashboards_made_by(
        &self,
        maker: UserId,
        after: Option<DashboardId>,
        limit: u64,
    ) -> Result<Vec<Dashboard>, FakeError> {
        Ok(self.read(|state| {
            let made = state
                .dashboards
                .records
                .iter()
                .filter(|dashboard| {
                    dashboard.maker == maker && dashboard.deleted_at.is_none() && !dashboard.home
                })
                .cloned()
                .collect();
            page(made, after, limit)
        }))
    }

    async fn dashboards_shared_with(
        &self,
        reach: Reach<'_>,
        after: Option<DashboardId>,
        limit: u64,
    ) -> Result<Vec<Dashboard>, FakeError> {
        Ok(self.read(|state| {
            let all = &state.dashboards;
            let shared = all
                .records
                .iter()
                .filter(|dashboard| {
                    dashboard.maker != reach.person
                        && dashboard.deleted_at.is_none()
                        && all.reaches(dashboard.id, &reach)
                })
                .cloned()
                .collect();
            page(shared, after, limit)
        }))
    }

    async fn shares_of_dashboards(
        &self,
        ids: &[DashboardId],
    ) -> Result<Vec<DashboardShare>, FakeError> {
        Ok(self.read(|state| {
            state
                .dashboards
                .shares
                .iter()
                .filter(|share| ids.contains(&share.dashboard))
                .cloned()
                .collect()
        }))
    }

    async fn dashboards_shared_with_group(
        &self,
        group: GroupId,
    ) -> Result<Vec<DashboardId>, FakeError> {
        Ok(self.read(|state| {
            state
                .dashboards
                .shares
                .iter()
                .filter(|share| share.target == ShareTarget::Group(group))
                .map(|share| share.dashboard)
                .collect()
        }))
    }

    async fn lock_dashboards_of(_tx: &mut Tx, _maker: UserId) -> Result<(), FakeError> {
        Ok(())
    }

    async fn insert_dashboard(
        tx: &mut Tx,
        dashboard: &Dashboard,
        boxes: &[DashboardBox],
    ) -> Result<(), FakeError> {
        tx.state.dashboards.records.push(dashboard.clone());
        tx.state.dashboards.boxes.extend(boxes.iter().cloned());
        Ok(())
    }

    async fn update_dashboard(
        tx: &mut Tx,
        dashboard: &Dashboard,
        boxes: &[DashboardBox],
        version: i64,
    ) -> Result<bool, FakeError> {
        let all = &mut tx.state.dashboards;
        let Some(stored) = all
            .records
            .iter_mut()
            .find(|stored| stored.id == dashboard.id && stored.version == version)
        else {
            return Ok(false);
        };
        *stored = dashboard.clone();
        all.boxes.retain(|each| each.dashboard != dashboard.id);
        all.boxes.extend(boxes.iter().cloned());
        Ok(true)
    }

    async fn set_dashboard_deleted(
        tx: &mut Tx,
        id: DashboardId,
        deleted_at: Option<Timestamp>,
        version: i64,
    ) -> Result<bool, FakeError> {
        let Some(stored) = tx
            .state
            .dashboards
            .records
            .iter_mut()
            .find(|stored| stored.id == id && stored.version == version)
        else {
            return Ok(false);
        };
        stored.deleted_at = deleted_at;
        stored.version = version.saturating_add(1);
        Ok(true)
    }

    async fn insert_dashboard_share(
        tx: &mut Tx,
        share: &DashboardShare,
    ) -> Result<bool, FakeError> {
        let shares = &mut tx.state.dashboards.shares;
        if shares
            .iter()
            .any(|each| each.dashboard == share.dashboard && each.target == share.target)
        {
            return Ok(false);
        }
        shares.push(share.clone());
        Ok(true)
    }

    async fn delete_dashboard_share(
        tx: &mut Tx,
        dashboard: DashboardId,
        target: ShareTarget,
    ) -> Result<bool, FakeError> {
        let shares = &mut tx.state.dashboards.shares;
        let before = shares.len();
        shares.retain(|each| !(each.dashboard == dashboard && each.target == target));
        Ok(shares.len() < before)
    }

    async fn delete_dashboard_shares_with_group(
        tx: &mut Tx,
        group: GroupId,
    ) -> Result<Vec<DashboardId>, FakeError> {
        let shares = &mut tx.state.dashboards.shares;
        let gone = shares
            .iter()
            .filter(|share| share.target == ShareTarget::Group(group))
            .map(|share| share.dashboard)
            .collect();
        shares.retain(|share| share.target != ShareTarget::Group(group));
        Ok(gone)
    }

    async fn delete_private_dashboards_of(
        tx: &mut Tx,
        maker: UserId,
    ) -> Result<Vec<DashboardId>, FakeError> {
        let all = &mut tx.state.dashboards;
        let private: Vec<DashboardId> = all
            .records
            .iter()
            .filter(|dashboard| dashboard.maker == maker && !all.shared(dashboard.id))
            .map(|dashboard| dashboard.id)
            .collect();
        all.records
            .retain(|dashboard| !private.contains(&dashboard.id));
        all.boxes.retain(|each| !private.contains(&each.dashboard));
        Ok(private)
    }
}
