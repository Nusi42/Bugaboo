//! Seeing dashboards (CFG-03): the person's own, the ones shared with them, one of them as its
//! viewer sees it, the person's *Home*, and the ready-made boxes.

use std::collections::HashMap;

use bb_core::ErrorCode;
use bb_core::dashboard::{HOME_BOXES, READY_MADE_BOXES, ReadyMadeBox, ready_made};
use bb_core::id::{DashboardId, GroupId, UserId};
use bb_core::message::{self, Message};
use bb_core::paging::{Cut, page_len, read_len};
use bb_core::people_kinds::PeopleKind;
use bb_core::saved_search::{SearchSet, ShareTarget};
use bb_policy::{Authorized, Evaluation, PeopleContext, Place};

use super::open::{Opened, Stored};
use super::types::{
    BoxChange, BoxView, DashboardItem, DashboardPage, DashboardQuery, DashboardView,
    DashboardsQuery, OpenHome, ReadyMadeList, ReadyMadeQuery, ReadyMadeView,
};
use super::write::inserting;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::input::cursor_or_invalid;
use crate::projects::Why;
use crate::saved_searches::reach::Reaching;
use crate::saved_searches::view::{shares_shown, viewer_of};

/// The box of a ready-made entry with its title in the words of the catalog, at `row` of the first
/// column.
fn ready_spec(entry: &ReadyMadeBox, row: u32) -> bb_core::dashboard::BoxSpec {
    entry.spec(Message::from(entry.which.title()).render(), 0, row)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// How the viewer is shown the maker of each dashboard, read for all of them at once: the
    /// rights where a dashboard reaches them through a project they see, else for the organization.
    async fn viewers_of(
        &self,
        actor: &Actor,
        reaching: &Reaching,
        found: &[(UserId, Vec<ShareTarget>)],
    ) -> Result<Vec<Evaluation>, AppError> {
        let makers: Vec<UserId> = found.iter().map(|(maker, _)| *maker).collect();
        self.preload_makers(&makers).await?;
        let organization = self.evaluation_of(actor, Place::Organization).await?;
        let viewers: Vec<Evaluation> = found
            .iter()
            .map(|(_, targets)| viewer_of(&organization, reaching, targets.iter().copied()))
            .collect();
        let by_role: Vec<(UserId, Place)> = found
            .iter()
            .zip(&viewers)
            .filter(|((maker, _), viewer)| {
                viewer.shows_role_label(PeopleKind::Authors, PeopleContext::Ordinary, *maker)
            })
            .map(|((maker, _), viewer)| (*maker, viewer.place()))
            .collect();
        self.preload_role_labels(&reaching.shown, &by_role).await?;
        Ok(viewers)
    }

    /// One dashboard as the actor sees it: the boxes that check for them, packed.
    pub(crate) async fn dashboard_view(
        &self,
        actor: &Actor,
        reaching: &Reaching,
        stored: Stored,
    ) -> Result<DashboardView, AppError> {
        let targets: Vec<ShareTarget> = stored.shares.iter().map(|share| share.target).collect();
        let maker = stored.dashboard.maker;
        let viewer = self
            .viewers_of(actor, reaching, &[(maker, targets.clone())])
            .await?
            .pop()
            .ok_or_else(|| AppError::Internal("the viewer of a dashboard is missing".to_owned()))?;
        let maker_ref = self
            .person_ref(&viewer, PeopleKind::Authors, PeopleContext::Ordinary, maker)
            .await?;
        let mine = maker == actor.person.id;
        let shared_with = if mine {
            let group_names: HashMap<GroupId, String> = if targets
                .iter()
                .any(|target| matches!(target, ShareTarget::Group(_)))
            {
                self.store()
                    .groups()
                    .await
                    .map_err(|error| internal::<S>(&error))?
                    .into_iter()
                    .map(|group| (group.id, group.name))
                    .collect()
            } else {
                HashMap::new()
            };
            shares_shown(targets, reaching, &group_names)
        } else {
            Vec::new()
        };
        let boxes = self
            .boxes_for(actor, reaching, &stored.boxes)
            .await?
            .into_iter()
            .map(|kept| BoxView {
                id: kept.id,
                spec: kept.spec,
            })
            .collect();
        Ok(DashboardView {
            id: stored.dashboard.id,
            name: stored.dashboard.name,
            maker: maker_ref,
            mine,
            home: stored.dashboard.home,
            shared_with,
            boxes,
            updated_at: stored.dashboard.updated_at,
            version: stored.dashboard.version,
        })
    }

    /// A dashboard as the actor sees it, wrapped as the organization's answer.
    pub(crate) async fn authorized_dashboard(
        &self,
        actor: &Actor,
        reaching: &Reaching,
        stored: Stored,
    ) -> Result<Authorized<DashboardView>, AppError> {
        let view = self.dashboard_view(actor, reaching, stored).await?;
        Ok(self
            .evaluation_of(actor, Place::Organization)
            .await?
            .authorize(view))
    }
}

/// The dashboards of one scope, a page at a time: the ones the person made (not *Home*), or the
/// ones other people shared with a group of theirs or a project they see. Oldest first.
///
/// # Errors
/// `request.invalid` for a cursor that is not one.
pub async fn dashboards<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DashboardsQuery,
) -> Result<Authorized<DashboardPage>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let len = page_len(Some(input.limit));
    let after = cursor_or_invalid(input.cursor.as_deref(), |text| {
        text.parse::<DashboardId>().ok()
    })?;
    let reaching = ctx.reaching(&actor).await?;
    let store = ctx.store();
    let found = match input.scope {
        SearchSet::Mine => {
            store
                .dashboards_made_by(actor.person.id, after, read_len(len))
                .await
        }
        SearchSet::Shared => {
            store
                .dashboards_shared_with(reaching.reach(), after, read_len(len))
                .await
        }
    }
    .map_err(|error| internal::<S>(&error))?;
    let cut = Cut::of(found, len);
    let next_cursor = cut.next_cursor(|dashboard| dashboard.id.to_string());
    let ids: Vec<DashboardId> = cut.items.iter().map(|dashboard| dashboard.id).collect();
    let mut shares = store
        .shares_of_dashboards(&ids)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let found: Vec<(UserId, Vec<ShareTarget>)> = cut
        .items
        .iter()
        .map(|dashboard| {
            let (taken, kept) = std::mem::take(&mut shares)
                .into_iter()
                .partition(|share| share.dashboard == dashboard.id);
            shares = kept;
            let taken: Vec<_> = taken;
            (
                dashboard.maker,
                taken.into_iter().map(|share| share.target).collect(),
            )
        })
        .collect();
    let viewers = ctx.viewers_of(&actor, &reaching, &found).await?;
    let mut items = Vec::with_capacity(cut.items.len());
    for (dashboard, viewer) in cut.items.into_iter().zip(viewers) {
        let maker = ctx
            .person_ref(
                &viewer,
                PeopleKind::Authors,
                PeopleContext::Ordinary,
                dashboard.maker,
            )
            .await?;
        items.push(DashboardItem {
            id: dashboard.id,
            name: dashboard.name,
            maker,
            mine: dashboard.maker == actor.person.id,
            updated_at: dashboard.updated_at,
            version: dashboard.version,
        });
    }
    let organization = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(organization.authorize(DashboardPage { items, next_cursor }))
}

/// One dashboard, opened: the person's own, or one shared with them, with the boxes that check for
/// them, packed.
///
/// # Errors
/// `resource.not-found` for a dashboard that does not exist, is in the trash, or is neither theirs
/// nor shared with them: one answer for all three (ADR-0050, decision 2).
pub async fn dashboard<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DashboardQuery,
) -> Result<Authorized<DashboardView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let Opened { stored, reaching } = ctx.open_dashboard(&actor, input.dashboard).await?;
    ctx.authorized_dashboard(&actor, &reaching, stored).await
}

/// The person's *Home* (ADR-0050, decision 6): their own dashboard, made at first use from the
/// blocks *Home* showed before, as the ready-made boxes "My timer", "My open issues" and "Due
/// soon". Nobody else reaches it. The first visit writes it, with its event.
///
/// # Errors
/// `limit.reached` when it has to be made and the person has as many dashboards as they may have.
pub async fn home<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: OpenHome,
) -> Result<Authorized<DashboardView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let reaching = ctx.reaching(&actor).await?;
    if let Some(stored) = ctx.stored_home(actor.person.id).await? {
        return ctx.authorized_dashboard(&actor, &reaching, stored).await;
    }
    // The boxes are kept as the ready-made list writes them, whatever the person may see now: a box
    // they cannot see is left out of the frame like any other, and shows once they may.
    let given: Vec<BoxChange> = HOME_BOXES
        .iter()
        .filter_map(|(which, place)| ready_made(*which).map(|entry| (entry, *place)))
        .map(|(entry, place)| BoxChange {
            id: None,
            spec: bb_core::dashboard::BoxSpec {
                place,
                ..ready_spec(entry, place.row)
            },
        })
        .collect();
    let id = ctx.new_dashboard_id()?;
    let boxes = ctx.placed(id, given, &[])?;
    let name = Message::from(message::DASHBOARD_HOME_NAME).render();
    let new = ctx.new_dashboard(&actor, (id, &name, true), boxes)?;
    let why = Why::default();
    let written = ctx
        .store()
        .record_change(&new.event, inserting::<S>(&new, why.clone()))
        .await
        .map_err(|error| why.said_or_failed::<S>(&error));
    match written {
        Ok(()) => ctx.made_dashboard_view(&actor, &reaching, new).await,
        // Made at the same moment by another request of the person: that one is their *Home*.
        Err(AppError::Refused(ErrorCode::VersionChanged)) => {
            let stored = ctx
                .stored_home(actor.person.id)
                .await?
                .ok_or(AppError::Refused(ErrorCode::NotFound))?;
            ctx.authorized_dashboard(&actor, &reaching, stored).await
        }
        Err(error) => Err(error),
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The *Home* of a person with its boxes, if it was made.
    async fn stored_home(&self, person: UserId) -> Result<Option<Stored>, AppError> {
        let home = self
            .store()
            .home_of(person)
            .await
            .map_err(|error| internal::<S>(&error))?;
        match home {
            Some(home) => self.stored_dashboard(home.id).await,
            None => Ok(None),
        }
    }
}

/// The ready-made boxes *Add box* offers, in order, each with its title in the words of the
/// catalog (ADR-0050, decision 4).
///
/// # Errors
/// As every read of a person's own data: an access key is turned away.
pub async fn ready_made_boxes<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ReadyMadeQuery,
) -> Result<Authorized<ReadyMadeList>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let items = READY_MADE_BOXES
        .iter()
        .map(|entry| ReadyMadeView {
            which: entry.which,
            spec: ready_spec(entry, 0),
        })
        .collect();
    Ok(ctx
        .evaluation_of(&actor, Place::Organization)
        .await?
        .authorize(ReadyMadeList { items }))
}
