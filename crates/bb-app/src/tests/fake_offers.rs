//! The offers part of the in-memory store.

use bb_core::id::{IssueTypeId, LabelGroupId, LabelId, ProjectId};
use bb_core::issue::Issue;
use bb_core::offer::{Offer, OfferSubject};
use bb_core::tags::IssueSight;

use super::fake::{FakeError, FakeStore, Tx};
use crate::offers::OfferStore;

/// Whether the person of the sight reads the issue.
fn reads(sight: &IssueSight, issue: &Issue) -> bool {
    (sight.with_trashed || issue.deleted_at.is_none())
        && (sight.all.contains(&issue.project)
            || (sight.own.contains(&issue.project) && issue.reported_by == sight.person))
}

impl OfferStore for FakeStore {
    async fn offers_in(tx: &mut Tx) -> Result<Vec<Offer>, FakeError> {
        Ok(tx.state.offers.clone())
    }

    async fn subject_offers_in(
        tx: &mut Tx,
        subject: OfferSubject,
    ) -> Result<Vec<Offer>, FakeError> {
        Ok(tx
            .state
            .offers
            .iter()
            .filter(|row| row.subject == subject)
            .copied()
            .collect())
    }

    async fn used_issue_types(&self, sight: &IssueSight) -> Result<Vec<IssueTypeId>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<IssueTypeId> = state
                .issues
                .iter()
                .filter(|issue| reads(sight, issue))
                .map(|issue| issue.issue_type)
                .collect();
            found.sort();
            found.dedup();
            found
        }))
    }

    async fn used_label_groups(&self, sight: &IssueSight) -> Result<Vec<LabelGroupId>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<LabelGroupId> = state
                .issue_labels
                .iter()
                .filter(|link| {
                    state
                        .issues
                        .iter()
                        .any(|issue| issue.id == link.issue && reads(sight, issue))
                })
                .filter_map(|link| {
                    state
                        .labels
                        .iter()
                        .find(|label| label.id == link.label)
                        .and_then(|label| label.group)
                })
                .collect();
            found.sort();
            found.dedup();
            found
        }))
    }

    async fn projects_with_label(&self, label: LabelId) -> Result<Vec<ProjectId>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<ProjectId> = state
                .issue_labels
                .iter()
                .filter(|link| link.label == label)
                .filter_map(|link| {
                    state
                        .issues
                        .iter()
                        .find(|issue| issue.id == link.issue)
                        .map(|issue| issue.project)
                })
                .collect();
            found.sort();
            found.dedup();
            found
        }))
    }

    async fn put_offer(tx: &mut Tx, offer: &Offer) -> Result<(), FakeError> {
        Self::remove_offer(tx, offer.subject, offer.project).await?;
        tx.state.offers.push(*offer);
        Ok(())
    }

    async fn remove_offer(
        tx: &mut Tx,
        subject: OfferSubject,
        project: Option<ProjectId>,
    ) -> Result<(), FakeError> {
        tx.state
            .offers
            .retain(|row| !(row.subject == subject && row.project == project));
        Ok(())
    }
}
