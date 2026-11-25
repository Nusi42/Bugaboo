//! The release part of the in-memory store (ISS-08).

use bb_core::id::{IssueId, ProjectId, ReleaseId};
use bb_core::project_release::{IssueFoundIn, Release};
use bb_core::version::Version;

use super::fake::{FakeError, FakeStore, Tx};
use crate::releases::ReleaseStore;

impl ReleaseStore for FakeStore {
    async fn release_in(tx: &mut Tx, id: ReleaseId) -> Result<Option<Release>, FakeError> {
        Ok(tx.state.releases.iter().find(|each| each.id == id).cloned())
    }

    async fn releases_of_projects_in(
        tx: &mut Tx,
        projects: Vec<ProjectId>,
    ) -> Result<Vec<Release>, FakeError> {
        Ok(tx
            .state
            .releases
            .iter()
            .filter(|each| each.deleted_at.is_none() && projects.contains(&each.project))
            .cloned()
            .collect())
    }

    async fn releases_by_ids_in(
        tx: &mut Tx,
        ids: Vec<ReleaseId>,
    ) -> Result<Vec<Release>, FakeError> {
        Ok(tx
            .state
            .releases
            .iter()
            .filter(|each| ids.contains(&each.id))
            .cloned()
            .collect())
    }

    async fn found_in_of_issues_in(
        tx: &mut Tx,
        issues: Vec<IssueId>,
    ) -> Result<Vec<IssueFoundIn>, FakeError> {
        let mut rows: Vec<IssueFoundIn> = tx
            .state
            .found_in
            .iter()
            .filter(|row| issues.contains(&row.issue))
            .cloned()
            .collect();
        rows.sort_by_key(|row| (row.issue, row.position));
        Ok(rows)
    }

    async fn release_named_in(tx: &mut Tx, release: ReleaseId) -> Result<bool, FakeError> {
        Ok(tx.state.found_in.iter().any(|row| row.release == release)
            || tx
                .state
                .issues
                .iter()
                .any(|issue| issue.planned_for_release == Some(release)))
    }

    async fn lock_releases(_tx: &mut Tx) -> Result<(), FakeError> {
        Ok(())
    }

    async fn insert_release(tx: &mut Tx, release: &Release) -> Result<(), FakeError> {
        tx.state.releases.push(release.clone());
        Ok(())
    }

    async fn update_release(
        tx: &mut Tx,
        release: &Release,
        version: Version,
    ) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .releases
            .iter_mut()
            .find(|each| each.id == release.id && each.version == version.get())
        else {
            return Ok(false);
        };
        *found = Release {
            version: version.get() + 1,
            ..release.clone()
        };
        Ok(true)
    }

    async fn replace_found_in(
        tx: &mut Tx,
        issue: IssueId,
        rows: &[IssueFoundIn],
    ) -> Result<(), FakeError> {
        tx.state.found_in.retain(|row| row.issue != issue);
        tx.state.found_in.extend_from_slice(rows);
        Ok(())
    }
}
