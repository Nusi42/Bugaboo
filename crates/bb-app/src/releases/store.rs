//! The port of the release use cases: what they need from storage beyond the issues port.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::id::{IssueId, ProjectId, ReleaseId};
use bb_core::project_release::{IssueFoundIn, Release};
use bb_core::version::Version;

use crate::issues::IssueStore;
use crate::store_macros::{queries, writes};

/// What the release use cases need from storage.
pub trait ReleaseStore: IssueStore + 'static {
    queries! {
        /// The release with this ID, also one in the trash.
        fn release(id: ReleaseId) -> Option<Release> = release_in;
        /// The releases made in these projects that are not in the trash.
        fn releases_of_projects(projects: Vec<ProjectId>) -> Vec<Release> = releases_of_projects_in;
        /// The releases with these IDs, also those in the trash.
        fn releases_by_ids(ids: Vec<ReleaseId>) -> Vec<Release> = releases_by_ids_in;
        /// The releases these issues were found in, in the order of each issue.
        fn found_in_of_issues(issues: Vec<IssueId>) -> Vec<IssueFoundIn> = found_in_of_issues_in;
        /// Whether any issue names the release, in either field, also an issue in the trash. Says
        /// nothing of which issues or how many (ADR-0049, decision 6).
        fn release_named(release: ReleaseId) -> bool = release_named_in;
    }

    writes! {
        /// Holds the releases until the change ends: a delete or a retire and a change that names a
        /// release are judged one after the other, so a release that an issue names is never
        /// deleted, and two releases of one name are never made at the same moment.
        fn lock_releases() -> ();
        /// Adds a release.
        fn insert_release(release: &Release) -> ();
        /// Writes a change of a release, counts its version up, and says whether it was still at
        /// `version`.
        fn update_release(release: &Release, version: Version) -> bool;
        /// Puts the releases an issue was found in in place of the ones it had.
        fn replace_found_in(issue: IssueId, rows: &[IssueFoundIn]) -> ();
    }
}
