//! The offers part of the in-memory store.

use bb_core::id::ProjectId;
use bb_core::offer::{Offer, OfferSubject};

use super::fake::{FakeError, FakeStore, Tx};
use crate::offers::OfferStore;

impl OfferStore for FakeStore {
    async fn offers_in(tx: &mut Tx) -> Result<Vec<Offer>, FakeError> {
        Ok(tx.state.offers.clone())
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
