//! Sharing a saved search with a group or a project, and stopping (ADR-0034, decisions 2 and 5).
//!
//! Only the person who made a search shares it. With a project they need *Share searches* there;
//! with a group they need to be in it and hold *Share searches* for the whole organization, or
//! *Manage groups*. Stopping needs nothing but being the maker, so a person who lost the right to
//! share can still take a share back. Being shared with never adds a right to a receiver.

use bb_core::ErrorCode;
use bb_core::event::{DomainEvent, FieldChange, RecordKind};
use bb_core::id::GroupId;
use bb_core::saved_search::{SHARES_MAX, SearchShare, ShareTarget};
use bb_policy::{Authorized, Permission, Place};
use serde_json::Value;

use super::events::{default_list_change, target_json};
use super::reach::{Opened, may_share_with_group};
use super::types::{SavedSearchView, ShareSearch, StopSharingSearch};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Stops, by, change_failed, entity};
use crate::member::saved_search as member;
use crate::projects::Why;

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether the actor may share with the group or the project: *Share searches* in the
    /// project; for a group, being in it with *Share searches* for the whole organization, or
    /// *Manage groups*.
    ///
    /// # Errors
    /// `resource.not-found` for a project the person does not see, `access.forbidden` for a place
    /// they may not share with, also a group that is not there.
    async fn require_to_share(
        &self,
        actor: &Actor,
        in_group: impl Fn(GroupId) -> bool,
        target: ShareTarget,
    ) -> Result<(), AppError> {
        match target {
            ShareTarget::Project(project) => {
                let held = self.evaluation_of(actor, Place::Project(project)).await?;
                // What is inside a project is not told to a person who does not see it.
                if !held.sees_project() {
                    return Err(ErrorCode::NotFound.into());
                }
                if !held.allows(Permission::ShareSearches) {
                    return Err(ErrorCode::Forbidden.into());
                }
            }
            ShareTarget::Group(group) => {
                let held = self.evaluation_of(actor, Place::Organization).await?;
                if !may_share_with_group(&held, in_group(group)) {
                    return Err(ErrorCode::Forbidden.into());
                }
                // Somebody who manages groups is told when the group is not there.
                self.store()
                    .group(group)
                    .await
                    .map_err(|error| crate::auth::internal::<S>(&error))?
                    .ok_or(AppError::Refused(ErrorCode::NotFound))?;
            }
        }
        Ok(())
    }
}

/// The share event: the group or project the search is shared with, before and after.
fn share_change(target: ShareTarget, shared: bool) -> FieldChange {
    let value: Option<Value> = Some(target_json(target));
    let (before, after) = if shared { (None, value) } else { (value, None) };
    FieldChange {
        field: member::SHARED_WITH.to_owned(),
        before,
        after,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The events of the saved searches that stop being shared because the group is deleted: one
    /// for each search, so that each one's history says it (ADR-0034, decision 7).
    /// The shares are read here and not in the change: a share made at the same moment can be
    /// missing from the events, never from the change, which deletes them all.
    pub(crate) async fn events_of_a_deleted_group(
        &self,
        by_person: bb_core::id::UserId,
        group: GroupId,
    ) -> Result<Vec<DomainEvent>, AppError> {
        let searches = self
            .store()
            .searches_shared_with_group(group)
            .await
            .map_err(|error| crate::auth::internal::<S>(&error))?;
        searches
            .into_iter()
            .map(|search| {
                self.event(
                    by(by_person),
                    entity(RecordKind::SavedSearch, search),
                    vec![share_change(ShareTarget::Group(group), false)],
                )
            })
            .collect()
    }
}

/// Shares a saved search with a group or a project. Sharing it again changes nothing.
///
/// # Errors
/// `resource.not-found` for a search the person does not see, `access.forbidden` for one they did
/// not make or a place they may not share with, `limit.reached` when it is shared with as many
/// places as it can be.
pub async fn share_search<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ShareSearch,
) -> Result<Authorized<SavedSearchView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let Opened {
        mut stored,
        reaching,
    } = ctx.own_search(&actor, input.search).await?;
    ctx.require_to_share(&actor, |group| reaching.is_in(group), input.target)
        .await?;
    if stored
        .shares
        .iter()
        .any(|share| share.target == input.target)
    {
        return ctx.authorized_view(&actor, &reaching, stored).await;
    }
    let share = SearchShare {
        search: stored.search.id,
        target: input.target,
        shared_at: ctx.now(),
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::SavedSearch, share.search),
        vec![share_change(input.target, true)],
    )?;
    let (written, maker) = (share.clone(), stored.search.maker);
    let why = Why::default();
    let full = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // Counted in the change, under the lock of the maker, so two shares at once cannot
                // pass the limit.
                S::lock_saved_searches_of(transaction, maker).await?;
                let shared = S::search_shares_in(transaction, written.search).await?;
                if shared.len() >= SHARES_MAX {
                    return Err(full.stop::<S>(ErrorCode::LimitReached));
                }
                S::insert_search_share(transaction, &written)
                    .await
                    .map(|_| ())
            })
        })
        .await
        .map_err(|error| why.said_or_failed::<S>(&error))?;
    stored.shares.push(share);
    ctx.authorized_view(&actor, &reaching, stored).await
}

/// Stops sharing a saved search with a group or a project. A project that had it as its list
/// shows the built-in default again. Stopping what is not shared changes nothing.
///
/// # Errors
/// `resource.not-found` for a search the person does not see, `access.forbidden` for one they did
/// not make.
pub async fn stop_sharing_search<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: StopSharingSearch,
) -> Result<Authorized<SavedSearchView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let Opened {
        mut stored,
        reaching,
    } = ctx.own_search(&actor, input.search).await?;
    if !stored
        .shares
        .iter()
        .any(|share| share.target == input.target)
    {
        return ctx.authorized_view(&actor, &reaching, stored).await;
    }
    let (search, target) = (stored.search.id, input.target);
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::SavedSearch, search),
        vec![share_change(target, false)],
    )?;
    // A project that had the search as its list shows the built-in default from now on, and its
    // history says why. Whether it had is read here and not in the change: a choice of the list at
    // the same moment can make this event differ from what the change does, and the history then
    // shows the one that was true when the person asked (the list itself is always right, because
    // the change clears it by the search).
    let mut events = vec![event];
    if let ShareTarget::Project(project) = target {
        let list = ctx
            .store()
            .project_default_list(project)
            .await
            .map_err(|error| crate::auth::internal::<S>(&error))?;
        if list.is_some_and(|row| row.search == search) {
            events.push(ctx.event(
                by(actor.person.id),
                entity(RecordKind::Project, project),
                vec![default_list_change(Some(search), None)],
            )?);
        }
    }
    ctx.store()
        .record_changes(&events, move |transaction| {
            Box::pin(async move {
                S::delete_search_share(transaction, search, target).await?;
                match target {
                    ShareTarget::Project(project) => {
                        S::clear_project_default_list(transaction, project, Some(search))
                            .await
                            .map(|_| ())
                    }
                    ShareTarget::Group(_) => Ok(()),
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    stored.shares.retain(|share| share.target != input.target);
    ctx.authorized_view(&actor, &reaching, stored).await
}
