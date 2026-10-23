//! *Follow*, *Following* and *Followers 3* (ISS-07). Anybody who sees an issue may follow it. The
//! names of the followers are shown as the kind of people information *Followers* says.

use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::follow::{FollowReason, Follower};
use bb_core::id::{IssueId, UserId};
use bb_core::paging::window;
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Authorized, PeopleContext};
use serde_json::json;

use super::types::{
    FollowIssue, Followers, FollowersQuery, FollowingQuery, FollowingState, UnfollowIssue,
};
use crate::auth::{AppError, AuthStore, Ctx, Done, by, entity, internal};
use crate::member::follower as member;
use crate::projects::Why;

impl<S: AuthStore> Ctx<'_, S> {
    /// The row of `person` among the followers of `issue`, if they follow it.
    async fn follows(&self, issue: IssueId, person: UserId) -> Result<Option<Follower>, AppError> {
        Ok(self
            .store()
            .followers_of(issue)
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .find(|follower| follower.person == person))
    }
}

/// Creates the person a follower of the issue. Following an issue already followed changes nothing
/// and writes no event.
///
/// # Errors
/// `resource.not-found` for an issue that does not exist, is in the trash or that the person may
/// not see.
pub async fn follow_issue<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: FollowIssue,
) -> Result<Done, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let (issue, _) = ctx.visible_issue(&actor, &input.issue).await?;
    let person = actor.person.id;
    if ctx.follows(issue.id, person).await?.is_some() {
        return Ok(Done);
    }
    let now = ctx.now();
    let event = ctx.event(
        by(person),
        entity(RecordKind::Follower, issue.id),
        vec![FieldChange {
            field: member::PERSON.to_owned(),
            before: None,
            after: Some(json!(person.to_string())),
        }],
    )?;
    let issue_id = issue.id;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::add_followers(
                    transaction,
                    &[Follower {
                        issue: issue_id,
                        person,
                        reason: FollowReason::Chose,
                        since: now,
                    }],
                )
                .await
            })
        })
        .await
        .map_err(|error| crate::auth::change_failed::<S>(&error, crate::auth::Stops::NONE))?;
    Ok(Done)
}

/// Takes the person out of the followers of the issue. Not following changes nothing and writes
/// no event.
///
/// # Errors
/// `resource.not-found` for an issue that does not exist, is in the trash or that the person may
/// not see.
pub async fn unfollow_issue<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: UnfollowIssue,
) -> Result<Done, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let (issue, _) = ctx.visible_issue(&actor, &input.issue).await?;
    let person = actor.person.id;
    if ctx.follows(issue.id, person).await?.is_none() {
        return Ok(Done);
    }
    let event = ctx.event(
        by(person),
        entity(RecordKind::Follower, issue.id),
        vec![FieldChange {
            field: member::PERSON.to_owned(),
            before: Some(json!(person.to_string())),
            after: None,
        }],
    )?;
    let issue_id = issue.id;
    let why = Why::default();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::remove_follower(transaction, issue_id, person)
                    .await
                    .map(|_| ())
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NotFound))?;
    Ok(Done)
}

/// Whether the person follows the issue, and why. It is the person's own state, so only a session
/// reaches it (an access key is turned away, ADR-0025, decision 8).
///
/// # Errors
/// `resource.not-found` for an issue that does not exist, is in the trash or that the person may
/// not see.
pub async fn following<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: FollowingQuery,
) -> Result<Authorized<FollowingState>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let mine = ctx.follows(issue.id, actor.person.id).await?;
    Ok(held.authorize(FollowingState {
        following: mine.is_some(),
        reason: mine.map(|follower| follower.reason),
    }))
}

/// Who follows the issue, a page at a time, each as the kind *Followers* says for the viewer. When
/// the kind is hidden for the viewer the list is left out with its length: no items, no cursor.
///
/// # Errors
/// `resource.not-found` for an issue that does not exist, is in the trash or that the person may
/// not see, `request.invalid` for a cursor that is not one.
pub async fn followers<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: FollowersQuery,
) -> Result<Authorized<Followers>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let hidden =
        held.people_level(PeopleKind::Followers, PeopleContext::Ordinary) == PeopleLevel::Hidden;
    let all = if hidden {
        Vec::new()
    } else {
        ctx.store()
            .followers_of(issue.id)
            .await
            .map_err(|error| internal::<S>(&error))?
    };
    let page = window(all.len(), input.cursor.as_deref(), input.limit)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    let mut items = Vec::with_capacity(page.end - page.start);
    for follower in &all[page.start..page.end] {
        items.push(
            ctx.person_ref(
                &held,
                PeopleKind::Followers,
                PeopleContext::Ordinary,
                follower.person,
            )
            .await?,
        );
    }
    Ok(held.authorize(Followers {
        items,
        next_cursor: page.next,
    }))
}
