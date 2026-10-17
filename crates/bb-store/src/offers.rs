//! The adapter of the offers port (`bb_app::offers::OfferStore`): where issue types and groups of
//! labels are offered (ISS-02, TAG-02).

use bb_app::offers::OfferStore;
use bb_core::id::{IssueTypeId, LabelGroupId, LabelId, ProjectId};
use bb_core::offer::{Offer, OfferKind, OfferSubject};
use bb_core::tags::IssueSight;
use sea_query::{Condition, Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction};
use crate::error::StoreError;
use crate::schema::{IssueLabelRow, IssueRow, LabelRow, OfferRow};
use crate::tags::sight_condition;

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

fn offer_select() -> SelectStatement {
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
    select
}

fn read_offer(row: &Row) -> Result<Offer, StoreError> {
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
}

/// The IDs in one column of the rows a distinct select found.
fn ids_of<T: StoredId>(
    rows: &[Row],
    column: impl sea_query::Iden + Copy,
) -> Result<Vec<T>, StoreError> {
    rows.iter().map(|row| row.id::<T>(column)).collect()
}

impl OfferStore for AuthStorage {
    async fn offers_in(transaction: &mut Transaction) -> Result<Vec<Offer>, StoreError> {
        transaction
            .fetch_all(&offer_select())
            .await?
            .iter()
            .map(read_offer)
            .collect()
    }

    async fn subject_offers_in(
        transaction: &mut Transaction,
        subject: OfferSubject,
    ) -> Result<Vec<Offer>, StoreError> {
        // The kind and the subject are the first two columns of the index `offer_by_subject`.
        let mut select = offer_select();
        select
            .and_where(Expr::col(OfferRow::Kind).eq(subject.kind().code()))
            .and_where(Expr::col(OfferRow::Subject).eq(subject.id_bytes().to_vec()));
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_offer)
            .collect()
    }

    async fn used_issue_types(&self, sight: &IssueSight) -> Result<Vec<IssueTypeId>, StoreError> {
        if sight.all.is_empty() && sight.own.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = Query::select();
        select
            .distinct()
            .column((IssueRow::Table, IssueRow::IssueType))
            .from(IssueRow::Table)
            .cond_where(sight_condition(sight));
        let rows = self.database().fetch_all(&select).await?;
        ids_of(&rows, IssueRow::IssueType)
    }

    async fn used_label_groups(&self, sight: &IssueSight) -> Result<Vec<LabelGroupId>, StoreError> {
        if sight.all.is_empty() && sight.own.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = Query::select();
        select
            .distinct()
            .column((LabelRow::Table, LabelRow::Group))
            .from(LabelRow::Table)
            .inner_join(
                IssueLabelRow::Table,
                Expr::col((IssueLabelRow::Table, IssueLabelRow::Label))
                    .equals((LabelRow::Table, LabelRow::Id)),
            )
            .inner_join(
                IssueRow::Table,
                Expr::col((IssueRow::Table, IssueRow::Id))
                    .equals((IssueLabelRow::Table, IssueLabelRow::Issue)),
            )
            .and_where(Expr::col((LabelRow::Table, LabelRow::Group)).is_not_null())
            .cond_where(sight_condition(sight));
        let rows = self.database().fetch_all(&select).await?;
        ids_of(&rows, LabelRow::Group)
    }

    async fn projects_with_label_in(
        transaction: &mut Transaction,
        label: LabelId,
    ) -> Result<Vec<ProjectId>, StoreError> {
        let mut select = Query::select();
        select
            .distinct()
            .column((IssueRow::Table, IssueRow::Project))
            .from(IssueLabelRow::Table)
            .inner_join(
                IssueRow::Table,
                Expr::col((IssueRow::Table, IssueRow::Id))
                    .equals((IssueLabelRow::Table, IssueLabelRow::Issue)),
            )
            .and_where(Expr::col((IssueLabelRow::Table, IssueLabelRow::Label)).eq(label.stored()));
        let rows = transaction.fetch_all(&select).await?;
        ids_of(&rows, IssueRow::Project)
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
