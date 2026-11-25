//! Creating and changing dashboards (CFG-03, ADR-0050, decisions 2, 3 and 9): create, change,
//! delete into the trash and take back (*Undo*), and *Save a copy*. Only the person who made a
//! dashboard changes or deletes it; a person with *Change organization settings* may delete any.

use bb_core::ErrorCode;
use bb_core::dashboard::{
    BoxSource, BoxSpec, DASHBOARDS_MAX, Dashboard, DashboardBox, boxes_are_valid, pack,
};
use bb_core::event::{DomainEvent, RecordKind};
use bb_core::id::{DashboardBoxId, DashboardId, UserId};
use bb_core::version::{FIRST_VERSION, next};
use bb_policy::{Authorized, Permission, Place};

use super::events;
use super::open::{Opened, Reads, Stored};
use super::types::{
    BoxChange, ChangeDashboard, CopyDashboard, CreateDashboard, DashboardView, DeleteDashboard,
    RestoreDashboard, RestoredDashboard,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Done, Stops, by, change_failed, entity};
use crate::input::clean_name;
use crate::ports::BoxFuture;
use crate::projects::Why;
use crate::saved_searches::reach::{Reached, Reaching};
use crate::saved_searches::write::clean_search_text;

/// A dashboard about to be written, with its boxes and the event of it.
pub(super) struct NewDashboard {
    pub(super) dashboard: Dashboard,
    pub(super) boxes: Vec<DashboardBox>,
    pub(super) event: DomainEvent,
}

/// Whether the maker may have one dashboard more outside the trash; it stops with
/// `limit.reached` (in `why`) when not. Taken under the lock of the maker and counted in the change
/// that writes, so two changes at once cannot both pass: the one rule of the limit.
async fn within_limit<S: AuthStore>(
    transaction: &mut S::Transaction,
    maker: UserId,
    why: &Why,
) -> Result<(), S::Error> {
    S::lock_dashboards_of(transaction, maker).await?;
    let have = S::count_dashboards_of_in(transaction, maker).await?;
    if have >= u64::try_from(DASHBOARDS_MAX).unwrap_or(u64::MAX) {
        return Err(why.stop::<S>(ErrorCode::LimitReached));
    }
    Ok(())
}

/// The change that writes a new dashboard with its boxes. It stops with `limit.reached` (in
/// `why`) when the person has as many dashboards as they may have, and with `edit.version-changed`
/// when it is a *Home* and the person got one meanwhile: both judged in the same change, under the
/// lock of the maker, so two at once cannot pass.
pub(super) fn inserting<S: AuthStore>(
    new: &NewDashboard,
    why: Why,
) -> impl for<'t> FnOnce(&'t mut S::Transaction) -> BoxFuture<'t, Result<(), S::Error>> + Send + 'static
{
    let (dashboard, boxes) = (new.dashboard.clone(), new.boxes.clone());
    move |transaction: &mut S::Transaction| {
        Box::pin(async move {
            within_limit::<S>(transaction, dashboard.maker, &why).await?;
            if dashboard.home && S::home_of_in(transaction, dashboard.maker).await?.is_some() {
                return Err(why.stop::<S>(ErrorCode::VersionChanged));
            }
            S::insert_dashboard(transaction, &dashboard, &boxes).await
        })
    }
}

/// The change that takes a dashboard out of the trash. It stops with `limit.reached` (in `why`)
/// when the maker has as many dashboards as they may have: counted in the same change, under the
/// lock of the maker, as when one is made.
fn restoring<S: AuthStore>(
    dashboard: &Dashboard,
    version: i64,
    why: Why,
) -> impl for<'t> FnOnce(&'t mut S::Transaction) -> BoxFuture<'t, Result<(), S::Error>> + Send + 'static
{
    let maker = dashboard.maker;
    let taken_out = trashing::<S>(dashboard, version, why.clone());
    move |transaction: &mut S::Transaction| {
        Box::pin(async move {
            within_limit::<S>(transaction, maker, &why).await?;
            taken_out(transaction).await
        })
    }
}

/// The change that puts a dashboard into the trash or takes it out. It stops, with the reason in
/// `why`, when the dashboard is gone or was changed meanwhile.
fn trashing<S: AuthStore>(
    dashboard: &Dashboard,
    version: i64,
    why: Why,
) -> impl for<'t> FnOnce(&'t mut S::Transaction) -> BoxFuture<'t, Result<(), S::Error>> + Send + 'static
{
    let (id, deleted_at) = (dashboard.id, dashboard.deleted_at);
    move |transaction: &mut S::Transaction| {
        Box::pin(async move {
            if S::set_dashboard_deleted(transaction, id, deleted_at, version).await? {
                Ok(())
            } else {
                Err(why.stop::<S>(ErrorCode::NotFound))
            }
        })
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The boxes as they are written on `dashboard`, in the order given: each ID named once, the
    /// places packed, and then at most twenty that fit the grid and do not overlap (judged on the
    /// packed places, so boxes kept below others that packing moves up are never refused for the
    /// row they were given). A box keeps its ID when it is one of
    /// `known`; the others get new ones. Nothing is judged for a person here.
    ///
    /// # Errors
    /// `request.invalid` for boxes that cannot be kept together, or an ID named twice.
    pub(super) fn placed(
        &self,
        dashboard: DashboardId,
        given: Vec<BoxChange>,
        known: &[DashboardBoxId],
    ) -> Result<Vec<DashboardBox>, AppError> {
        let named: Vec<DashboardBoxId> = given.iter().filter_map(|each| each.id).collect();
        let repeated = named
            .iter()
            .enumerate()
            .any(|(at, id)| named[..at].contains(id));
        let places: Vec<_> = given.iter().map(|each| each.spec.place).collect();
        let boxes: Vec<DashboardBox> = given
            .into_iter()
            .zip(pack(&places).into_iter().zip(0_i64..))
            .map(|(each, (place, position))| DashboardBox {
                dashboard,
                id: each
                    .id
                    .filter(|id| known.contains(id))
                    .unwrap_or_else(|| DashboardBoxId::generate(&mut self.ids())),
                position,
                spec: BoxSpec { place, ..each.spec },
            })
            .collect();
        let specs: Vec<BoxSpec> = boxes.iter().map(|each| each.spec.clone()).collect();
        if repeated || !boxes_are_valid(&specs) {
            return Err(ErrorCode::InvalidRequest.into());
        }
        Ok(boxes)
    }

    /// The boxes as the actor keeps them on `dashboard`: each title a name, each text trimmed and
    /// in its canonical spelling, every box checking for the actor (a text they may run, a saved
    /// search they may open, a field they may read), placed as [`Self::placed`] places them.
    ///
    /// # Errors
    /// `request.invalid` for a box that cannot be kept or does not check for the actor.
    pub(super) async fn kept_boxes(
        &self,
        (actor, reaching): (&Actor, &Reaching),
        dashboard: DashboardId,
        given: Vec<BoxChange>,
        known: &[DashboardBoxId],
    ) -> Result<Vec<DashboardBox>, AppError> {
        let mut judge = self.judge(actor, reaching).await?;
        judge.lift_budget();
        let mut checked = Vec::with_capacity(given.len());
        for each in given {
            let mut spec = each.spec;
            spec.title = clean_name(&spec.title)?;
            if let Some(BoxSource::Text(text)) = &spec.source {
                spec.source = Some(BoxSource::Text(clean_search_text(text)?));
            }
            let reads = self
                .box_reads(&mut judge, reaching, &spec)
                .await?
                .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
            if let (Some(BoxSource::Text(_)), Reads::Text(canonical)) = (&spec.source, reads) {
                spec.source = Some(BoxSource::Text(canonical));
            }
            checked.push(BoxChange { id: each.id, spec });
        }
        drop(judge);
        self.placed(dashboard, checked, known)
    }

    /// The stored boxes the maker did not name in a change and cannot see now: a change keeps them
    /// (ADR-0050, decision 3), because the maker was never shown them to keep or to remove.
    async fn unseen_boxes(
        &self,
        (actor, reaching): (&Actor, &Reaching),
        stored: &[DashboardBox],
        named: &[DashboardBoxId],
    ) -> Result<Vec<DashboardBox>, AppError> {
        let mut judge = self.judge(actor, reaching).await?;
        judge.lift_budget();
        let mut unseen = Vec::new();
        for each in stored.iter().filter(|each| !named.contains(&each.id)) {
            if self
                .box_reads(&mut judge, reaching, &each.spec)
                .await?
                .is_none()
            {
                unseen.push(each.clone());
            }
        }
        Ok(unseen)
    }

    /// The boxes a change names, then the boxes it keeps unseen, in their packed places below
    /// them.
    fn with_unseen(
        &self,
        dashboard: DashboardId,
        named: Vec<DashboardBox>,
        unseen: Vec<DashboardBox>,
    ) -> Result<Vec<DashboardBox>, AppError> {
        let below = named
            .iter()
            .map(|each| each.spec.place.row + each.spec.place.height)
            .max()
            .unwrap_or(0);
        let top = unseen
            .iter()
            .map(|each| each.spec.place.row)
            .min()
            .unwrap_or(0);
        let known: Vec<DashboardBoxId> = named.iter().chain(&unseen).map(|each| each.id).collect();
        let shifted = unseen.into_iter().map(|mut each| {
            each.spec.place.row = each.spec.place.row - top + below;
            each
        });
        let given = named
            .into_iter()
            .chain(shifted)
            .map(|each| BoxChange {
                id: Some(each.id),
                spec: each.spec,
            })
            .collect();
        self.placed(dashboard, given, &known)
    }

    /// The ID of a dashboard about to be made.
    pub(super) fn new_dashboard_id(&self) -> Result<DashboardId, AppError> {
        DashboardId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    /// A new dashboard of the actor, private, with its boxes and the event that says it was made.
    pub(super) fn new_dashboard(
        &self,
        actor: &Actor,
        (id, name, home): (DashboardId, &str, bool),
        boxes: Vec<DashboardBox>,
    ) -> Result<NewDashboard, AppError> {
        let name = clean_name(name)?;
        let now = self.now();
        let dashboard = Dashboard {
            id,
            name,
            maker: actor.person.id,
            home,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            version: FIRST_VERSION,
        };
        let event = self.event(
            by(actor.person.id),
            entity(RecordKind::Dashboard, id),
            events::made(&dashboard, &boxes),
        )?;
        Ok(NewDashboard {
            dashboard,
            boxes,
            event,
        })
    }

    /// A new private dashboard of the actor with boxes that each check for them.
    async fn checked_dashboard(
        &self,
        (actor, reaching): (&Actor, &Reaching),
        name: &str,
        specs: Vec<BoxSpec>,
    ) -> Result<NewDashboard, AppError> {
        let id = self.new_dashboard_id()?;
        let given = specs
            .into_iter()
            .map(|spec| BoxChange { id: None, spec })
            .collect();
        let boxes = self.kept_boxes((actor, reaching), id, given, &[]).await?;
        self.new_dashboard(actor, (id, name, false), boxes)
    }

    /// A dashboard that was just written, as its maker sees it.
    pub(super) async fn made_dashboard_view(
        &self,
        actor: &Actor,
        reaching: &Reaching,
        new: NewDashboard,
    ) -> Result<Authorized<DashboardView>, AppError> {
        let stored = Stored {
            dashboard: new.dashboard,
            boxes: new.boxes,
            shares: Vec::new(),
        };
        self.authorized_dashboard(actor, reaching, stored).await
    }

    /// The dashboard to put into the trash or take out of it: the person's own, or any one for
    /// somebody with *Change organization settings*; never a *Home*. One the person may not delete
    /// is refused when they see it and not found when they do not.
    async fn dashboard_to_trash(
        &self,
        actor: &Actor,
        id: DashboardId,
        in_trash: bool,
    ) -> Result<(Stored, Reaching), AppError> {
        let stored = self.stored_dashboard(id).await?;
        let reaching = self.reaching(actor).await?;
        let organization = self.evaluation_of(actor, Place::Organization).await?;
        let found = stored.filter(|found| found.dashboard.deleted_at.is_some() == in_trash);
        let Some(found) = found else {
            return Err(ErrorCode::NotFound.into());
        };
        let mine = found.dashboard.maker == actor.person.id;
        if (mine || organization.allows(Permission::ChangeOrganizationSettings))
            && !found.dashboard.home
        {
            return Ok((found, reaching));
        }
        // Whoever does not see it is told nothing about it, and nobody else is told about the trash.
        let sees = found.dashboard.deleted_at.is_none() && found.reached(&reaching) != Reached::Not;
        if sees {
            Err(ErrorCode::Forbidden.into())
        } else {
            Err(ErrorCode::NotFound.into())
        }
    }

    /// The event of putting a dashboard into the trash or taking it out (the version counts up),
    /// and the dashboard as it is afterwards.
    fn dashboard_trash_event(
        &self,
        actor: &Actor,
        found: &Dashboard,
        deleted: bool,
    ) -> Result<(DomainEvent, Dashboard), AppError> {
        let now = self.now();
        let after = Dashboard {
            deleted_at: deleted.then_some(now),
            updated_at: now,
            version: next(found.version),
            ..found.clone()
        };
        let event = self.event(
            by(actor.person.id),
            entity(RecordKind::Dashboard, found.id),
            events::trashed(found, &after),
        )?;
        Ok((event, after))
    }
}

/// Creates a dashboard under a name, private and the person's own.
///
/// # Errors
/// `request.invalid` for a name that cannot be one or a box that cannot be kept or does not check
/// for the person; `limit.reached` when they have as many dashboards as they may have.
pub async fn create_dashboard<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateDashboard,
) -> Result<Authorized<DashboardView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let reaching = ctx.reaching(&actor).await?;
    let new = ctx
        .checked_dashboard((&actor, &reaching), &input.name, input.boxes)
        .await?;
    let why = Why::default();
    ctx.store()
        .record_change(&new.event, inserting::<S>(&new, why.clone()))
        .await
        .map_err(|error| why.said_or_failed::<S>(&error))?;
    ctx.made_dashboard_view(&actor, &reaching, new).await
}

/// Changes the name or the boxes of a dashboard. Only its maker does.
///
/// # Errors
/// `resource.not-found` for a dashboard the person does not see, `access.forbidden` for one they
/// see and did not make, `request.invalid` as for creating, `edit.version-changed` when it changed
/// since the person looked at it.
pub async fn change_dashboard<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeDashboard,
) -> Result<Authorized<DashboardView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let Opened { stored, reaching } = ctx.own_dashboard(&actor, input.dashboard).await?;
    let before = stored.dashboard.clone();
    let record = entity(RecordKind::Dashboard, before.id);
    if before.version != input.version {
        return Err(AppError::stale(
            record,
            Place::Organization,
            actor.person.id,
        ));
    }
    let mut dashboard = before.clone();
    if let Some(name) = &input.name {
        dashboard.name = clean_name(name)?;
    }
    let boxes = match input.boxes {
        Some(given) => {
            let known: Vec<DashboardBoxId> = stored.boxes.iter().map(|each| each.id).collect();
            let named: Vec<DashboardBoxId> = given.iter().filter_map(|each| each.id).collect();
            let kept = ctx
                .kept_boxes((&actor, &reaching), before.id, given, &known)
                .await?;
            let unseen = ctx
                .unseen_boxes((&actor, &reaching), &stored.boxes, &named)
                .await?;
            if unseen.is_empty() {
                kept
            } else {
                ctx.with_unseen(before.id, kept, unseen)?
            }
        }
        None => stored.boxes.clone(),
    };
    let changes = events::changed((&before, &stored.boxes), (&dashboard, &boxes));
    if changes.is_empty() {
        return ctx.authorized_dashboard(&actor, &reaching, stored).await;
    }
    dashboard.updated_at = ctx.now();
    dashboard.version = next(before.version);
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Dashboard, dashboard.id),
        changes,
    )?;
    let (written, written_boxes, was) = (dashboard.clone(), boxes.clone(), before.version);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::update_dashboard(transaction, &written, &written_boxes, was).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| {
            change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)).about(
                &record,
                Place::Organization,
                actor.person.id,
            )
        })?;
    let saved = Stored {
        dashboard,
        boxes,
        shares: stored.shares,
    };
    ctx.authorized_dashboard(&actor, &reaching, saved).await
}

/// Puts a dashboard into the trash. Its maker does, and so does anybody with *Change organization
/// settings*; *Home* is not deleted.
///
/// # Errors
/// `resource.not-found` for a dashboard that does not exist, is in the trash already or that the
/// person does not see, `access.forbidden` for one they see and may not delete.
pub async fn delete_dashboard<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteDashboard,
) -> Result<Done, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let (found, _) = ctx
        .dashboard_to_trash(&actor, input.dashboard, false)
        .await?;
    let (event, after) = ctx.dashboard_trash_event(&actor, &found.dashboard, true)?;
    let why = Why::default();
    ctx.store()
        .record_change(
            &event,
            trashing::<S>(&after, found.dashboard.version, why.clone()),
        )
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NotFound))?;
    Ok(Done)
}

/// Takes a dashboard out of the trash (*Undo*), for whoever may put it there.
///
/// # Errors
/// `resource.not-found` for a dashboard that is not in the trash or that the person may not take
/// out: whoever may not is not told that it is in the trash (SEC-06); `limit.reached` when its
/// maker has as many dashboards as they may have.
pub async fn restore_dashboard<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RestoreDashboard,
) -> Result<Authorized<RestoredDashboard>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let (found, reaching) = ctx
        .dashboard_to_trash(&actor, input.dashboard, true)
        .await?;
    let (event, after) = ctx.dashboard_trash_event(&actor, &found.dashboard, false)?;
    let why = Why::default();
    ctx.store()
        .record_change(
            &event,
            restoring::<S>(&after, found.dashboard.version, why.clone()),
        )
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NotFound))?;
    let restored = Stored {
        dashboard: after,
        ..found
    };
    // Somebody who may take it out of the trash without being its maker is shown it only when it
    // would be theirs to open: what they may do is not what they may read.
    let view = if restored.reached(&reaching) == Reached::Not {
        None
    } else {
        Some(ctx.dashboard_view(&actor, &reaching, restored).await?)
    };
    let organization = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(organization.authorize(RestoredDashboard { dashboard: view }))
}

/// *Save a copy*: a private dashboard of the person with the boxes of one they may open, as they
/// see them (a box left out for them is not copied). Everyone who sees a dashboard may copy it.
///
/// # Errors
/// `resource.not-found` for a dashboard the person does not see, `request.invalid` for a name that
/// cannot be one, `limit.reached` when they have as many dashboards as they may have.
pub async fn copy_dashboard<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CopyDashboard,
) -> Result<Authorized<DashboardView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let Opened { stored, reaching } = ctx.open_dashboard(&actor, input.dashboard).await?;
    let name = input.name.unwrap_or_else(|| stored.dashboard.name.clone());
    let specs = ctx
        .boxes_for(&actor, &reaching, &stored.boxes)
        .await?
        .into_iter()
        .map(|kept| kept.spec)
        .collect();
    let new = ctx
        .checked_dashboard((&actor, &reaching), &name, specs)
        .await?;
    let why = Why::default();
    ctx.store()
        .record_change(&new.event, inserting::<S>(&new, why.clone()))
        .await
        .map_err(|error| why.said_or_failed::<S>(&error))?;
    ctx.made_dashboard_view(&actor, &reaching, new).await
}
