//! Sharing a dashboard with a group or a project, and stopping (ADR-0050, decision 2): the rules of
//! saved searches (ADR-0034, decisions 2 and 5) with *Share dashboards* in place of *Share
//! searches*, through the one check of both (`Ctx::require_to_share`). Only the maker shares;
//! stopping needs nothing but being the maker. *Home* is never shared.

use bb_core::ErrorCode;
use bb_core::dashboard::DashboardShare;
use bb_core::event::{DomainEvent, RecordKind};
use bb_core::id::{GroupId, UserId};
use bb_core::saved_search::{SHARES_MAX, ShareTarget};
use bb_policy::Authorized;

use super::events::share_change;
use super::open::Opened;
use super::types::{DashboardView, ShareDashboard, StopSharingDashboard};
use crate::auth::{AppError, AuthStore, Ctx, Stops, by, change_failed, entity};
use crate::projects::Why;
use crate::saved_searches::Shareable;

impl<S: AuthStore> Ctx<'_, S> {
    /// The events of the dashboards that stop being shared because the group is deleted: one for
    /// each, so that each one's history says it. Read here and not in the change, like those of
    /// saved searches (`events_of_a_deleted_group`).
    pub(crate) async fn dashboard_events_of_a_deleted_group(
        &self,
        by_person: UserId,
        group: GroupId,
    ) -> Result<Vec<DomainEvent>, AppError> {
        self.store()
            .dashboards_shared_with_group(group)
            .await
            .map_err(|error| crate::auth::internal::<S>(&error))?
            .into_iter()
            .map(|dashboard| {
                self.event(
                    by(by_person),
                    entity(RecordKind::Dashboard, dashboard),
                    vec![share_change(ShareTarget::Group(group), false)],
                )
            })
            .collect()
    }
}

/// Shares a dashboard with a group or a project. Sharing it again changes nothing.
///
/// # Errors
/// `resource.not-found` for a dashboard the person does not see, `access.forbidden` for one they
/// did not make, their *Home*, or a place they may not share with, `limit.reached` when it is
/// shared with as many places as it can be.
pub async fn share_dashboard<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ShareDashboard,
) -> Result<Authorized<DashboardView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let Opened {
        mut stored,
        reaching,
    } = ctx.own_dashboard(&actor, input.dashboard).await?;
    if stored.dashboard.home {
        return Err(ErrorCode::Forbidden.into());
    }
    ctx.require_to_share(
        &actor,
        |group| reaching.is_in(group),
        (Shareable::Dashboards, input.target),
    )
    .await?;
    if stored
        .shares
        .iter()
        .any(|share| share.target == input.target)
    {
        return ctx.authorized_dashboard(&actor, &reaching, stored).await;
    }
    let share = DashboardShare {
        dashboard: stored.dashboard.id,
        target: input.target,
        shared_at: ctx.now(),
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Dashboard, share.dashboard),
        vec![share_change(input.target, true)],
    )?;
    let (written, maker) = (share.clone(), stored.dashboard.maker);
    let why = Why::default();
    let full = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // Counted in the change, under the lock of the maker, so two shares at once cannot
                // pass the limit.
                S::lock_dashboards_of(transaction, maker).await?;
                let shared = S::dashboard_shares_in(transaction, written.dashboard).await?;
                if shared.len() >= SHARES_MAX {
                    return Err(full.stop::<S>(ErrorCode::LimitReached));
                }
                S::insert_dashboard_share(transaction, &written)
                    .await
                    .map(|_| ())
            })
        })
        .await
        .map_err(|error| why.said_or_failed::<S>(&error))?;
    stored.shares.push(share);
    ctx.authorized_dashboard(&actor, &reaching, stored).await
}

/// Stops sharing a dashboard with a group or a project. Stopping what is not shared changes
/// nothing.
///
/// # Errors
/// `resource.not-found` for a dashboard the person does not see, `access.forbidden` for one they
/// did not make.
pub async fn stop_sharing_dashboard<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: StopSharingDashboard,
) -> Result<Authorized<DashboardView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let Opened {
        mut stored,
        reaching,
    } = ctx.own_dashboard(&actor, input.dashboard).await?;
    if !stored
        .shares
        .iter()
        .any(|share| share.target == input.target)
    {
        return ctx.authorized_dashboard(&actor, &reaching, stored).await;
    }
    let (dashboard, target) = (stored.dashboard.id, input.target);
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Dashboard, dashboard),
        vec![share_change(target, false)],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::delete_dashboard_share(transaction, dashboard, target)
                    .await
                    .map(|_| ())
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    stored.shares.retain(|share| share.target != input.target);
    ctx.authorized_dashboard(&actor, &reaching, stored).await
}
