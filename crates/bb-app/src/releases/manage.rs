//! Making, changing, marking, retiring and deleting the releases of a project (ISS-08, ADR-0049,
//! decisions 2, 5 and 6). Every one needs *Manage releases* in the project the release was made
//! in; each is an event of the release with the old and the new values (ARCH-EVENTS), on the
//! version the person looked at (ARCH-VERSIONED).
//!
//! A name is used once among the releases of a project that are not in the trash, by the one rule
//! of names; that, and whether an issue names a release that is deleted, is judged again inside
//! the change after the lock of the releases, so two people at the same moment cannot both pass.

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::given::SetTo;
use bb_core::id::{ProjectId, ReleaseId, UserId};
use bb_core::names::{is_new_or_changed, same_name};
use bb_core::project::Project;
use bb_core::project_release::{Release, ReleaseState};
use bb_core::version::{FIRST_VERSION, Version, counted_up, next};
use bb_policy::{Authorized, Evaluation, Permission, Place};
use serde_json::{Value, json};

use super::types::{ChangeRelease, CreateRelease, DeleteRelease, ReleaseView};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::input::{clean_name, given_reason};
use crate::projects::{ProjectRef, Why};
use std::sync::Arc;

/// The names of the members of a release in its events: defined once, here.
pub(crate) mod member {
    pub(crate) const NAME: &str = "name";
    pub(crate) const PROJECT: &str = "project";
    pub(crate) const RELEASE_DATE: &str = "release-date";
    pub(crate) const STATE: &str = "state";
    pub(crate) const RELEASED_ON: &str = "released-on";
    /// Why a release went back from *Released* to *Planned*.
    pub(crate) const REASON: &str = "reason";
    pub(crate) const DELETED_AT: &str = "deleted-at";
}

/// Whether another release of the same project than `except`, not in the trash, has the name.
fn name_taken(releases: &[Release], name: &str, except: Option<ReleaseId>) -> bool {
    releases
        .iter()
        .any(|found| Some(found.id) != except && same_name(&found.name, name))
}

/// A change from `before` to `after`, when they differ.
fn difference(field: &str, before: Value, after: Value) -> Option<FieldChange> {
    (before != after).then(|| FieldChange {
        field: field.to_owned(),
        before: Some(before).filter(|value| !value.is_null()),
        after: Some(after).filter(|value| !value.is_null()),
    })
}

/// A release as a change writes it, judged again inside the change after the lock of the
/// releases: the name, and for a delete whether an issue names it.
#[derive(Clone)]
struct ReleaseWrite {
    /// The release as it will be.
    written: Release,
    /// The version the change is made on.
    version: i64,
    /// Whether the change is refused while an issue names the release (a delete).
    unused_only: bool,
}

impl ReleaseWrite {
    /// The write of `written` on `version`.
    fn of(written: &Release, version: i64, unused_only: bool) -> Self {
        Self {
            written: written.clone(),
            version,
            unused_only,
        }
    }

    async fn write_in<S: AuthStore>(
        &self,
        transaction: &mut S::Transaction,
        stopped: &Why,
    ) -> Result<(), S::Error> {
        let written = &self.written;
        S::lock_releases(transaction).await?;
        if written.deleted_at.is_none() {
            let others = S::releases_of_projects_in(transaction, vec![written.project]).await?;
            if name_taken(&others, &written.name, Some(written.id)) {
                return Err(stopped.stop::<S>(ErrorCode::ReleaseNameTaken));
            }
        }
        if self.unused_only && S::release_named_in(transaction, written.id).await? {
            return Err(stopped.stop::<S>(ErrorCode::ReleaseInUse));
        }
        if S::update_release(transaction, written, Version::new(self.version)).await? {
            Ok(())
        } else {
            Err(stopped.stop::<S>(ErrorCode::VersionChanged))
        }
    }

    /// The change that writes it, for `record_change`.
    fn change<S: AuthStore>(
        &self,
        stopped: &Why,
    ) -> impl for<'t> FnOnce(
        &'t mut S::Transaction,
    ) -> crate::ports::BoxFuture<'t, Result<(), S::Error>>
    + Send
    + 'static {
        let (writing, stopped) = (self.clone(), stopped.clone());
        move |transaction| {
            Box::pin(async move { writing.write_in::<S>(transaction, &stopped).await })
        }
    }

    /// What the person is told when the write was refused or failed.
    fn refused<S: AuthStore>(&self, why: &Why, error: &S::Error, editor: UserId) -> AppError {
        why.answer::<S>(error, ErrorCode::VersionChanged).about(
            &entity(RecordKind::Release, self.written.id),
            Place::Project(self.written.project),
            editor,
        )
    }

    /// The event of the change, by `actor`.
    fn event<S: AuthStore>(
        &self,
        ctx: &Ctx<'_, S>,
        actor: &Actor,
        changes: Vec<FieldChange>,
    ) -> Result<bb_core::event::DomainEvent, AppError> {
        ctx.event(
            by(actor.person.id),
            entity(RecordKind::Release, self.written.id),
            changes,
        )
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The day it is now for the person, in the zone of their *Region*: the day a release they
    /// mark *Released* keeps as *Released on*.
    async fn today_of(&self, actor: &Actor) -> Result<jiff::civil::Date, AppError> {
        let layers = self.setting_layers_of(Some(actor.person.id)).await?;
        let region = bb_core::wording::Region::of(&layers);
        Ok(self.now().to_zoned(region.zone().clone()).date())
    }

    /// The project a person names, when they see it, with what they hold there: one they do not
    /// see answers like one that does not exist.
    pub(crate) async fn release_project(
        &self,
        actor: &Actor,
        project: &str,
    ) -> Result<(Project, Arc<Evaluation>), AppError> {
        let project = self
            .find_project(project)
            .await?
            .map(|found| found.project)
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let held = self
            .evaluation_of(actor, Place::Project(project.id))
            .await?;
        if !held.sees_project() {
            return Err(ErrorCode::NotFound.into());
        }
        Ok((project, held))
    }

    /// The project with *Manage releases*, or `access.forbidden` (`resource.not-found` for one the
    /// person does not see).
    async fn managed_project(
        &self,
        actor: &Actor,
        project: &str,
    ) -> Result<(Project, Arc<Evaluation>), AppError> {
        let (project, _) = self.release_project(actor, project).await?;
        let held = self
            .require(
                actor,
                Permission::ManageReleases,
                Place::Project(project.id),
            )
            .await?;
        Ok((project, held))
    }

    /// The release of the project, also one in the trash when `in_trash` says so; one of another
    /// project answers like one that does not exist.
    async fn release_of(
        &self,
        project: ProjectId,
        release: ReleaseId,
        in_trash: bool,
    ) -> Result<Release, AppError> {
        self.store()
            .release(release)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|found| found.project == project && found.deleted_at.is_some() == in_trash)
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }
}

/// Makes a release in a project, in the state *Planned*.
///
/// # Errors
/// `resource.not-found` for a project the person does not see, `access.forbidden` without *Manage
/// releases* there, `request.invalid` for a name that cannot be one, `release.name-taken` for a
/// name another release of the project has.
pub async fn create_release<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateRelease,
) -> Result<Authorized<ReleaseView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (project, held) = ctx.managed_project(&actor, &input.project).await?;
    let name = clean_name(&input.name)?;
    let now = ctx.now();
    let release = Release {
        id: ReleaseId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        project: project.id,
        name,
        release_date: input.release_date,
        state: ReleaseState::Planned,
        released_on: None,
        made_by: actor.person.id,
        made_at: now,
        changed_by: actor.person.id,
        changed_at: now,
        deleted_at: None,
        version: FIRST_VERSION,
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Release, release.id),
        vec![
            set_to(member::NAME, json!(release.name)),
            set_to(member::PROJECT, json!(project.id.to_string())),
            set_to(member::STATE, json!(release.state.code())),
        ]
        .into_iter()
        .chain(
            release
                .release_date
                .map(|day| set_to(member::RELEASE_DATE, json!(day))),
        )
        .collect(),
    )?;
    let why = Why::default();
    let (stopped, stored) = (why.clone(), release.clone());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_releases(transaction).await?;
                let others = S::releases_of_projects_in(transaction, vec![stored.project]).await?;
                if name_taken(&others, &stored.name, None) {
                    return Err(stopped.stop::<S>(ErrorCode::ReleaseNameTaken));
                }
                S::insert_release(transaction, &stored).await
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::ReleaseNameTaken))?;
    Ok(held.authorize(ReleaseView::of(&release, ProjectRef::from(&project), true)))
}

/// What a change asks of a release: the release as it will be, and the changes of its event.
fn planned_change(
    current: &Release,
    input: &ChangeRelease,
    actor: &Actor,
    now: jiff::Timestamp,
    today: jiff::civil::Date,
) -> Result<(Release, Vec<FieldChange>), AppError> {
    let name = input.name.as_deref().map(clean_name).transpose()?;
    let name = name.unwrap_or_else(|| current.name.clone());
    let release_date = input
        .release_date
        .map_or(current.release_date, SetTo::into_option);
    let state = input.state.unwrap_or(current.state);
    let reason = if current.state.needs_reason(state) {
        Some(given_reason(
            Some(input.reason.as_deref().unwrap_or_default()),
            ErrorCode::ReleaseReasonNeeded,
        )?)
    } else {
        None
    };
    let released_on = if state == current.state {
        current.released_on
    } else {
        current
            .state
            .released_on_after(state, current.released_on, today)
    };
    let after = Release {
        name,
        release_date,
        state,
        released_on,
        changed_by: actor.person.id,
        changed_at: now,
        version: next(current.version),
        ..current.clone()
    };
    let mut changes: Vec<FieldChange> = [
        difference(member::NAME, json!(current.name), json!(after.name)),
        difference(
            member::RELEASE_DATE,
            json!(current.release_date),
            json!(after.release_date),
        ),
        difference(
            member::STATE,
            json!(current.state.code()),
            json!(after.state.code()),
        ),
        difference(
            member::RELEASED_ON,
            json!(current.released_on),
            json!(after.released_on),
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !changes.is_empty() {
        changes.extend(reason.map(|reason| set_to(member::REASON, json!(reason))));
        changes.push(counted_up(current.version));
    }
    Ok((after, changes))
}

/// Changes a release: renames it, gives it another *Release date*, marks it *Released* (the day is
/// kept as *Released on*), takes it back to *Planned* with a reason, or retires it.
///
/// # Errors
/// `resource.not-found` for a project the person does not see or a release that is not one of the
/// project (or is in the trash), `access.forbidden` without *Manage releases*, `request.invalid`
/// when nothing is asked or for a name that cannot be one, `release.reason-needed` for a release
/// that goes back to *Planned* without a reason, `release.name-taken`, `edit.version-changed` when
/// somebody changed the release since it was looked at.
pub async fn change_release<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeRelease,
) -> Result<Authorized<ReleaseView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (project, held) = ctx.managed_project(&actor, &input.project).await?;
    let current = ctx.release_of(project.id, input.release, false).await?;
    if input.name.is_none() && input.release_date.is_none() && input.state.is_none() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let record = entity(RecordKind::Release, current.id);
    if current.version != input.version {
        return Err(AppError::stale(
            record,
            Place::Project(project.id),
            actor.person.id,
        ));
    }
    let now = ctx.now();
    let today = ctx.today_of(&actor).await?;
    let (after, changes) = planned_change(&current, &input, &actor, now, today)?;
    if changes.is_empty() {
        return Ok(held.authorize(ReleaseView::of(&current, ProjectRef::from(&project), true)));
    }
    if is_new_or_changed(Some(&current.name), &after.name) {
        let others = ctx
            .store()
            .releases_of_projects(vec![project.id])
            .await
            .map_err(|error| internal::<S>(&error))?;
        if name_taken(&others, &after.name, Some(after.id)) {
            return Err(ErrorCode::ReleaseNameTaken.into());
        }
    }
    let write = ReleaseWrite::of(&after, current.version, false);
    let (event, why) = (write.event(ctx, &actor, changes)?, Why::default());
    ctx.store()
        .record_change(&event, write.change::<S>(&why))
        .await
        .map_err(|error| write.refused::<S>(&why, &error, actor.person.id))?;
    Ok(held.authorize(ReleaseView::of(&after, ProjectRef::from(&project), true)))
}

/// The release as a move into the trash (`deleted_at` a moment) or out of it (none) leaves it.
fn in_trash(
    current: &Release,
    actor: &Actor,
    now: jiff::Timestamp,
    deleted_at: Option<jiff::Timestamp>,
) -> Release {
    Release {
        deleted_at,
        changed_by: actor.person.id,
        changed_at: now,
        version: next(current.version),
        ..current.clone()
    }
}

/// Puts a release that no issue names into the trash; *Undo* ([`restore_release`]) takes it out.
/// A release that an issue names is refused with `release.in-use`, which says nothing of which
/// issues or how many, also not of an issue the person does not see (ADR-0049, decision 6).
///
/// # Errors
/// `resource.not-found` for a project the person does not see or a release that is not one of the
/// project (or is in the trash already), `access.forbidden` without *Manage releases*,
/// `release.in-use` while an issue names it.
pub async fn delete_release<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteRelease,
) -> Result<Authorized<ReleaseView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (project, held) = ctx.managed_project(&actor, &input.project).await?;
    let current = ctx.release_of(project.id, input.release, false).await?;
    let now = ctx.now();
    let after = in_trash(&current, &actor, now, Some(now));
    let changes = vec![
        set_to(member::DELETED_AT, json!(now.to_string())),
        counted_up(current.version),
    ];
    let write = ReleaseWrite::of(&after, current.version, true);
    let (event, why) = (write.event(ctx, &actor, changes)?, Why::default());
    ctx.store()
        .record_change(&event, write.change::<S>(&why))
        .await
        .map_err(|error| write.refused::<S>(&why, &error, actor.person.id))?;
    Ok(held.authorize(ReleaseView::of(&after, ProjectRef::from(&project), true)))
}

/// Takes a release out of the trash (*Undo*). Refused with `release.name-taken` when another
/// release of the project took its name meanwhile.
///
/// # Errors
/// `resource.not-found` for a project the person does not see or a release of the project that
/// is not in the trash, `access.forbidden` without *Manage releases*, `release.name-taken`.
pub async fn restore_release<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteRelease,
) -> Result<Authorized<ReleaseView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (project, held) = ctx.managed_project(&actor, &input.project).await?;
    let current = ctx.release_of(project.id, input.release, true).await?;
    let after = in_trash(&current, &actor, ctx.now(), None);
    let changes = vec![
        FieldChange {
            field: member::DELETED_AT.to_owned(),
            before: current.deleted_at.map(|at| json!(at.to_string())),
            after: None,
        },
        counted_up(current.version),
    ];
    let write = ReleaseWrite::of(&after, current.version, false);
    let (event, why) = (write.event(ctx, &actor, changes)?, Why::default());
    ctx.store()
        .record_change(&event, write.change::<S>(&why))
        .await
        .map_err(|error| write.refused::<S>(&why, &error, actor.person.id))?;
    Ok(held.authorize(ReleaseView::of(&after, ProjectRef::from(&project), true)))
}
