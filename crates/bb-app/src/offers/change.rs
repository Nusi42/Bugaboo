//! Changing where an issue type or a group of labels is offered (ISS-02, TAG-02).
//!
//! Every row is judged at the place it is for: the permission to look after issue types (or
//! labels) in that project, or for the whole organization. A subject made in a project takes rows
//! only for that project and below. The change counts the version of the subject up, like any other
//! change of it, so a person who looked at an old state is stopped.

use std::collections::BTreeSet;

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::{EntityId, ProjectId};
use bb_core::offer::{Offer, OfferSubject};
use bb_core::paging::window;
use bb_core::version::counted_up;
use bb_policy::{Authorized, Permission, Place, Tree};
use serde_json::{Value, json};

use super::place::place_of;
use super::types::{ChangeOffers, OfferChange, OfferRowPage, OfferRowView, Offers, OffersQuery};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal};
use crate::projects::Why;

/// What a subject is, as far as the offers are concerned.
pub(crate) struct Subject {
    pub(crate) id: OfferSubject,
    /// The project it was made in; none for the whole organization.
    pub(crate) owner: Option<ProjectId>,
    /// The version it is at.
    pub(crate) version: i64,
    /// What a person needs to hold to look after it.
    pub(crate) permission: Permission,
    /// The kind of record its events are about.
    pub(crate) record: RecordKind,
}

impl Subject {
    /// The record of its events.
    fn entity(&self) -> bb_core::event::EntityRef {
        let id: EntityId = match self.id {
            OfferSubject::IssueType(id) => id.into(),
            OfferSubject::LabelGroup(id) => id.into(),
        };
        entity(self.record, id)
    }

    /// The place its owner is judged at.
    pub(crate) fn place(&self) -> Place {
        place_of(self.owner)
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The subject, or `resource.not_found`.
    pub(crate) async fn offer_subject(&self, id: OfferSubject) -> Result<Subject, AppError> {
        let missing = || AppError::Refused(ErrorCode::NotFound);
        match id {
            OfferSubject::IssueType(issue_type) => {
                let found = self
                    .store()
                    .issue_type(issue_type)
                    .await
                    .map_err(|error| internal::<S>(&error))?
                    .ok_or_else(missing)?;
                Ok(Subject {
                    id,
                    owner: found.project,
                    version: found.version,
                    permission: Permission::ManageIssueTypes,
                    record: RecordKind::IssueType,
                })
            }
            OfferSubject::LabelGroup(group) => {
                let found = self
                    .store()
                    .label_groups()
                    .await
                    .map_err(|error| internal::<S>(&error))?
                    .into_iter()
                    .find(|each| each.id == group)
                    .ok_or_else(missing)?;
                Ok(Subject {
                    id,
                    owner: found.project,
                    version: found.version,
                    permission: Permission::ManageLabels,
                    record: RecordKind::LabelGroup,
                })
            }
        }
    }

    /// Whether the person holds `permission` at `place`; a place they may not know is as good as
    /// one where they lack it.
    pub(crate) async fn holds_at(
        &self,
        actor: &Actor,
        permission: Permission,
        place: Place,
    ) -> Result<bool, AppError> {
        match self.evaluation_of(actor, place).await {
            Ok(held) => Ok(held.allows(permission)),
            Err(AppError::Refused(ErrorCode::NotFound)) => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// The place a project address names: the project, or the whole organization for none.
    pub(crate) async fn offer_place(&self, project: Option<&str>) -> Result<Place, AppError> {
        match project {
            None => Ok(Place::Organization),
            Some(address) => Ok(Place::Project(
                self.find_project(address)
                    .await?
                    .ok_or(AppError::Refused(ErrorCode::NotFound))?
                    .project
                    .id,
            )),
        }
    }

    /// The tree of the projects, read once for a request.
    pub(crate) async fn project_tree_now(&self) -> Result<Tree, AppError> {
        Ok(Tree::of(
            &self
                .store()
                .project_tree()
                .await
                .map_err(|error| internal::<S>(&error))?,
        ))
    }

    /// The rows of a subject as the person sees them.
    async fn offer_rows(
        &self,
        actor: &Actor,
        subject: &Subject,
    ) -> Result<Vec<OfferRowView>, AppError> {
        let rows = self
            .store()
            .offers()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let tree = self.project_tree_now().await?;
        let mut views = Vec::new();
        for row in rows.iter().filter(|row| row.subject == subject.id) {
            views.push(OfferRowView {
                place: self.place_label(actor, place_of(row.project)).await?,
                offered: row.offered,
                may_change: self
                    .may_offer_at(actor, subject, row.project, &tree)
                    .await?,
            });
        }
        Ok(views)
    }

    /// Whether a row at this place may be set for the subject by the person: the subject reaches
    /// the place, and the person looks after such subjects there.
    pub(crate) async fn may_offer_at(
        &self,
        actor: &Actor,
        subject: &Subject,
        project: Option<ProjectId>,
        tree: &Tree,
    ) -> Result<bool, AppError> {
        if !reaches(subject.owner, project, tree) {
            return Ok(false);
        }
        self.holds_at(actor, subject.permission, place_of(project))
            .await
    }
}

/// Whether a subject made in `owner` takes a row at `project`: one made for the whole organization
/// takes a row anywhere; one made in a project, only for that project and below.
pub(crate) fn reaches(owner: Option<ProjectId>, project: Option<ProjectId>, tree: &Tree) -> bool {
    match (owner, project) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(owner), Some(project)) => tree.is_within(project, owner),
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

/// The rows that say where a subject is offered, a page at a time. Anybody who is signed in may
/// read them: the names of the types and the groups are not hidden from anybody who works here, and
/// a project the person may not see is not named.
///
/// # Errors
/// `resource.not_found` for a subject that is not there, `request.invalid` for a cursor that is not
/// one.
pub async fn offers<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: OffersQuery,
) -> Result<Authorized<OfferRowPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let subject = ctx.offer_subject(input.subject).await?;
    let rows = ctx.offer_rows(&actor, &subject).await?;
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
    let subject = ctx.offer_subject(input.subject).await?;
    if input.changes.is_empty() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let tree = ctx.project_tree_now().await?;
    // Every place is judged before anything is written; the first one the person may not change
    // answers for all.
    let mut places: Places = Vec::new();
    let mut seen = BTreeSet::new();
    for OfferChange { project, offered } in &input.changes {
        let place = ctx.offer_place(project.as_deref()).await?;
        let project = match place {
            Place::Organization => None,
            Place::Project(project) => Some(project),
        };
        if !seen.insert(project) {
            return Err(ErrorCode::InvalidRequest.into());
        }
        // A person who may not know the place is told it is not there; one who knows it and lacks
        // the permission is refused.
        ctx.require(&actor, subject.permission, place).await?;
        if !reaches(subject.owner, project, &tree) {
            return Err(ErrorCode::InvalidRequest.into());
        }
        places.push((project, *offered));
    }
    let record = subject.entity();
    if subject.version != input.version {
        return Err(AppError::stale(record, subject.place(), actor.person.id));
    }
    let before: Vec<Offer> = ctx
        .store()
        .offers()
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .filter(|row| row.subject == subject.id)
        .collect();
    let after = after_rows(subject.id, &before, &places);
    let change = FieldChange {
        field: member_of(subject.record).to_owned(),
        before: Some(rows_json(&before)),
        after: Some(rows_json(&after)),
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
    let now = ctx.offer_subject(input.subject).await?;
    Ok(held.authorize(Offers {
        rows: ctx.offer_rows(&actor, &now).await?,
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
