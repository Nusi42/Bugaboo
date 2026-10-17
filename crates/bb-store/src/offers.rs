//! The adapter of the offers port (`bb_app::offers::OfferStore`): where issue types and groups of
//! labels are offered (ISS-02, TAG-02).

use bb_app::offers::OfferStore;
use bb_core::id::ProjectId;
use bb_core::offer::{Offer, OfferKind, OfferSubject};
use sea_query::{Condition, Expr, ExprTrait as _, Order, Query};

use crate::auth::AuthStorage;
use crate::db::{StoredId, Transaction};
use crate::error::StoreError;
use crate::schema::OfferRow;

/// The condition that picks the row of exactly this subject and place: a missing project is
/// `NULL`, which `=` never matches.
fn at(subject: OfferSubject, project: Option<ProjectId>) -> Condition {
    let place = match project {
        Some(project) => Expr::col(OfferRow::Project).eq(project.stored()),
        None => Expr::col(OfferRow::Project).is_null(),
    };
    Condition::all()
        .add(Expr::col(OfferRow::Kind).eq(subject.kind().code()))
        .add(Expr::col(OfferRow::Subject).eq(subject.id_bytes().to_vec()))
        .add(place)
}

impl OfferStore for AuthStorage {
    async fn offers_in(transaction: &mut Transaction) -> Result<Vec<Offer>, StoreError> {
        let mut select = Query::select();
        select
            .columns([
                OfferRow::Kind,
                OfferRow::Subject,
                OfferRow::Project,
                OfferRow::Offered,
            ])
            .from(OfferRow::Table)
            .order_by(OfferRow::Subject, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                let kind = OfferKind::from_code(&row.get::<String>(OfferRow::Kind)?)
                    .ok_or(StoreError::Corrupt("an offer kind"))?;
                let bytes = <[u8; 16]>::try_from(row.get::<Vec<u8>>(OfferRow::Subject)?)
                    .map_err(|_| StoreError::Corrupt("an offer subject"))?;
                Ok(Offer {
                    subject: OfferSubject::from_parts(kind, bytes)
                        .ok_or(StoreError::Corrupt("an offer subject"))?,
                    project: row.optional_id(OfferRow::Project)?,
                    offered: row.get(OfferRow::Offered)?,
                })
            })
            .collect()
    }

    async fn put_offer(transaction: &mut Transaction, offer: &Offer) -> Result<(), StoreError> {
        Self::remove_offer(transaction, offer.subject, offer.project).await?;
        let mut insert = Query::insert();
        insert
            .into_table(OfferRow::Table)
            .columns([
                OfferRow::Kind,
                OfferRow::Subject,
                OfferRow::Project,
                OfferRow::Offered,
            ])
            .values([
                Expr::val(offer.subject.kind().code()),
                Expr::val(offer.subject.id_bytes().to_vec()),
                Expr::val(offer.project.map(StoredId::stored)),
                Expr::val(offer.offered),
            ])
            .map_err(|_| StoreError::Corrupt("an offer"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn remove_offer(
        transaction: &mut Transaction,
        subject: OfferSubject,
        project: Option<ProjectId>,
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(OfferRow::Table)
            .cond_where(at(subject, project));
        transaction.execute(&delete).await?;
        Ok(())
    }
}
