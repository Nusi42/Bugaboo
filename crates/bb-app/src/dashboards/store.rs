//! The port of the dashboards: what the use cases need from storage.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::dashboard::{Dashboard, DashboardBox, DashboardShare};
use bb_core::id::{DashboardId, GroupId, UserId};
use bb_core::saved_search::ShareTarget;
use jiff::Timestamp;

use crate::ports::ChangeStore;
use crate::saved_searches::Reach;
use crate::store_macros::{queries, reads, writes};

/// What the dashboard use cases need from storage.
pub trait DashboardStore: ChangeStore + 'static {
    queries! {
        /// The dashboard with this ID, also one in the trash.
        fn dashboard(id: DashboardId) -> Option<Dashboard> = dashboard_in;
        /// The boxes of one dashboard, in order.
        fn dashboard_boxes(id: DashboardId) -> Vec<DashboardBox> = dashboard_boxes_in;
        /// The groups and projects one dashboard is shared with, in the order they were added.
        fn dashboard_shares(id: DashboardId) -> Vec<DashboardShare> = dashboard_shares_in;
        /// The *Home* of a person, if it was made.
        fn home_of(person: UserId) -> Option<Dashboard> = home_of_in;
        /// How many dashboards a person has, the trash and *Home* included.
        fn count_dashboards_of(maker: UserId) -> u64 = count_dashboards_of_in;
        /// How many dashboards a person made that are shared with a group or a project: the ones
        /// that stay, shown with *Former user* as their maker, when the person is removed.
        fn count_shared_dashboards_of(maker: UserId) -> u64 = count_shared_dashboards_of_in;
    }

    reads! {
        /// The dashboards a person made that are not in the trash, *Home* left out, by ID (oldest
        /// first), after `after`, at most `limit`.
        fn dashboards_made_by(
            maker: UserId,
            after: Option<DashboardId>,
            limit: u64
        ) -> Vec<Dashboard>;
        /// The dashboards of other people that are shared with a place that reaches the person and
        /// are not in the trash, by ID, after `after`, at most `limit`.
        fn dashboards_shared_with(
            reach: Reach<'_>,
            after: Option<DashboardId>,
            limit: u64
        ) -> Vec<Dashboard>;
        /// The shares of these dashboards.
        fn shares_of_dashboards(ids: &[DashboardId]) -> Vec<DashboardShare>;
        /// The dashboards that are shared with a group, by ID.
        fn dashboards_shared_with_group(group: GroupId) -> Vec<DashboardId>;
    }

    writes! {
        /// Takes the lock of one person's dashboards, so that two changes of theirs at the same
        /// moment cannot both pass the limits of dashboards and of shares, or both make their
        /// *Home*; does nothing where writes cannot overlap.
        fn lock_dashboards_of(maker: UserId) -> ();
        /// Adds a dashboard with its boxes.
        fn insert_dashboard(dashboard: &Dashboard, boxes: &[DashboardBox]) -> ();
        /// Writes the name and the boxes of a dashboard, counts the version up to the one in
        /// `dashboard`, and says whether the dashboard was still at `version`.
        fn update_dashboard(
            dashboard: &Dashboard,
            boxes: &[DashboardBox],
            version: i64
        ) -> bool;
        /// Puts a dashboard into the trash (a date) or takes it out (`None`), writes the version
        /// after `version`, and says whether the dashboard was at `version`.
        fn set_dashboard_deleted(
            id: DashboardId,
            deleted_at: Option<Timestamp>,
            version: i64
        ) -> bool;
        /// Shares a dashboard; `false` when it was shared with the place already.
        fn insert_dashboard_share(share: &DashboardShare) -> bool;
        /// Stops sharing; `false` when it was not shared with the place.
        fn delete_dashboard_share(dashboard: DashboardId, target: ShareTarget) -> bool;
        /// Takes a group out of every share of a dashboard (the group is deleted); the IDs of the
        /// dashboards that were shared with it.
        fn delete_dashboard_shares_with_group(group: GroupId) -> Vec<DashboardId>;
        /// Deletes the dashboards a person made that are shared with nobody, *Home* among them,
        /// with their boxes (the person is removed, ADR-0050, decision 11); the shared ones stay.
        /// The IDs of those that went.
        fn delete_private_dashboards_of(maker: UserId) -> Vec<DashboardId>;
    }
}
