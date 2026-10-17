//! Changing where an issue type or a group of labels is offered (ISS-02, TAG-02).
//!
//! Every row is judged at the place it is for: the permission to look after issue types (or
//! labels) in that project, or for the whole organization. A subject made in a project takes rows
//! only for that project and below. The change counts the version of the subject up, like any other
//! change of it, so a person who looked at an old state is stopped.

use std::collections::BTreeSet;

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::ProjectId;
use bb_core::offer::{Offer, OfferSubject, reaches};
use bb_core::paging::window;
use bb_core::version::counted_up;
use bb_policy::{Authorized, Place};
use serde_json::{Value, json};

use super::subject::Subject;
use super::types::{ChangeOffers, OfferChange, OfferRowPage, OfferRowView, Offers, OffersQuery};
use crate::auth::{AppError, AuthStore, Ctx, by, internal};
use crate::projects::{Shown, Why};

impl<S: AuthStore> Ctx<'_, S> {
    /// The rows of a subject as the person sees them: a row at a place they may not know is left
    /// out, so the answer never names or counts a hidden project.
    async fn offer_rows(
        &self,
        shown: &Shown,
        subject: &Subject,
    ) -> Result<Vec<OfferRowView>, AppError> {
        let rows = self
            .store()
            .subject_offers(subject.id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(rows
            .iter()
            .filter(|row| row.project.is_none_or(|project| shown.knows(project)))
            .map(|row| {
                let place = Place::from(row.project);
                OfferRowView {
                    place: shown.label(place),
                    offered: row.offered,
                    may_change: subject.may_offer_at(shown, place),
                }
            })
            .collect())
    }
}

/// The rows of a subject, for the event: the places and what they say.
fn rows_json(rows: &[Offer]) -> Value {
    json!(
        rows.iter()
            .map(|row| json!({
                "project": row.project.map(|project| project.to_string()),
                "offered": row.offered,
            }))
            .collect::<Vec<_>>()
    )
}

/// The name of the member of an event that holds where a subject is offered.
const fn member_of(record: RecordKind) -> &'static str {
    match record {
        RecordKind::LabelGroup => crate::member::label_group::OFFERED,
        _ => crate::issues::member::issue_type::OFFERED,
    }
}

/// The rows that say where a subject is offered, a page at a time. Anybody who is signed in and may
/// know the subject (`known.rs`) may read them; a row at a project the person may not see is left
/// out.
///
/// # Errors
/// `resource.not_found` for a subject that is not there or that the person may not know,
/// `request.invalid` for a cursor that is not one.
pub async fn offers<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: OffersQuery,
) -> Result<Authorized<OfferRowPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let shown = ctx.shown_to(&actor).await?;
    let subject = ctx.offer_subject(&shown, input.subject).await?;
    let rows = ctx.offer_rows(&shown, &subject).await?;
    let page = window(rows.len(), input.cursor.as_deref(), input.limit)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    Ok(held.authorize(OfferRowPage {
        items: rows[page.start..page.end].to_vec(),
        next_cursor: page.next,
    }))
}

/// The places to change as `(project, what it says)`: a missing project is the whole organization.
type Places = Vec<(Option<ProjectId>, Option<bool>)>;

/// Changes where a subject is offered.
///
/// # Errors
/// `resource.not_found` for a subject or a project that is not there for the person,
/// `access.forbidden` without the permission at one of the places, `request.invalid` for a place the
/// subject does not reach, a place named twice or nothing asked, `edit.version_changed` when
/// somebody changed the subject since it was looked at.
pub async fn change_offers<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeOffers,
) -> Result<Authorized<Offers>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let shown = ctx.shown_to(&actor).await?;
    let subject = ctx.offer_subject(&shown, input.subject).await?;
    if input.changes.is_empty() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    // Every place is judged before anything is written; the first one the person may not change
    // answers for all.
    let mut places: Places = Vec::new();
    let mut seen = BTreeSet::new();
    for OfferChange { project, offered } in &input.changes {
        let place = ctx.offer_place(project.as_deref()).await?;
        let project = place.project();
        if !seen.insert(project) {
            return Err(ErrorCode::InvalidRequest.into());
        }
        // A person who may not know the place is told it is not there; one who knows it and lacks
        // the permission is refused.
        ctx.require(&actor, subject.permission, place).await?;
        if !reaches(subject.owner, &shown.chain(place)) {
            return Err(ErrorCode::InvalidRequest.into());
        }
        places.push((project, *offered));
    }
    let record = subject.entity();
    if subject.version != input.version {
        return Err(AppError::stale(record, subject.place(), actor.person.id));
    }
    let before = ctx
        .store()
        .subject_offers(subject.id)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let after = after_rows(subject.id, &before, &places);
    // The event names the places that were changed, which the person knows, and no other.
    let touched = |rows: &[Offer]| {
        let kept: Vec<Offer> = rows
            .iter()
            .filter(|row| places.iter().any(|(project, _)| *project == row.project))
            .copied()
            .collect();
        rows_json(&kept)
    };
    let change = FieldChange {
        field: member_of(subject.record).to_owned(),
        before: Some(touched(&before)),
        after: Some(touched(&after)),
    };
    let event = ctx.event(
        by(actor.person.id),
        record.clone(),
        vec![change, counted_up(subject.version)],
    )?;
    let why = Why::default();
    let (stopped, id, version) = (why.clone(), subject.id, subject.version);
    let (writes, removes) = split(id, &places);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_workflows(transaction).await?;
                // The version is counted up whatever the rows say: the subject changed.
                if !bump_in::<S>(transaction, id, version).await? {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                for row in &writes {
                    S::put_offer(transaction, row).await?;
                }
                for project in &removes {
                    S::remove_offer(transaction, id, *project).await?;
                }
                Ok(())
            })
        })
        .await
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                &record,
                subject.place(),
                actor.person.id,
            )
        })?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let now = ctx.offer_subject(&shown, input.subject).await?;
    Ok(held.authorize(Offers {
        rows: ctx.offer_rows(&shown, &now).await?,
        version: now.version,
    }))
}

/// The rows of the subject after the changes: a place that says `Some` has that row, a place that
/// says `None` has none.
fn after_rows(subject: OfferSubject, before: &[Offer], places: &Places) -> Vec<Offer> {
    let mut rows: Vec<Offer> = before
        .iter()
        .filter(|row| places.iter().all(|(project, _)| *project != row.project))
        .copied()
        .collect();
    rows.extend(places.iter().filter_map(|(project, offered)| {
        offered.map(|offered| Offer {
            subject,
            project: *project,
            offered,
        })
    }));
    rows
}

/// The rows to write and the places whose row is to go.
fn split(subject: OfferSubject, places: &Places) -> (Vec<Offer>, Vec<Option<ProjectId>>) {
    let mut writes = Vec::new();
    let mut removes = Vec::new();
    for (project, offered) in places {
        match offered {
            Some(offered) => writes.push(Offer {
                subject,
                project: *project,
                offered: *offered,
            }),
            None => removes.push(*project),
        }
    }
    (writes, removes)
}

/// Counts the version of the subject up without changing it, and says whether it was still at
/// `version`.
async fn bump_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    subject: OfferSubject,
    version: i64,
) -> Result<bool, S::Error> {
    match subject {
        OfferSubject::IssueType(id) => {
            let Some(current) = S::issue_types_in(transaction)
                .await?
                .into_iter()
                .find(|each| each.id == id)
            else {
                return Ok(false);
            };
            S::update_issue_type(
                transaction,
                id,
                &current.name,
                current.icon,
                current.retired_at,
                version,
            )
            .await
        }
        OfferSubject::LabelGroup(id) => {
            let Some(current) = S::label_groups_in(transaction)
                .await?
                .into_iter()
                .find(|each| each.id == id)
            else {
                return Ok(false);
            };
            S::update_label_group(
                transaction,
                id,
                &current.name,
                current.pick_one,
                current.retired_at,
                version,
            )
            .await
        }
    }
}
