//! Combining two labels into one (TAG-03).

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_policy::{Authorized, Place};

use super::super::refs::NameRef;
use super::super::rewrite::{Combining, Replacement, Rewrite, Sorted};
use super::super::types::{CombineLabels, LabelView};
use super::{Reach, crowd, two_of_in};
use crate::auth::{AppError, AuthStore, Ctx, entity};

/// Moves every issue from a label to another one, then takes the first label away.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags*, `resource.not_found` for a label that is
/// not there, `request.invalid` for the same label twice, `label.pick_one_conflict` when some issue
/// would then have two labels of a *Pick one* group, `edit.version_changed` when somebody changed
/// one of the labels since it was looked at.
pub async fn combine_labels<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CombineLabels,
) -> Result<Authorized<LabelView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let shown = ctx.shown_to(&actor).await?;
    let catalogue = ctx.label_catalogue().await?;
    let (from, into) = (
        catalogue.label(input.label)?.clone(),
        catalogue.label(input.into)?.clone(),
    );
    // The issues of both labels change, so the person looks after both.
    let held = ctx
        .label_manager(&actor, &shown, &catalogue, from.group)
        .await?;
    ctx.label_manager(&actor, &shown, &catalogue, into.group)
        .await?;
    let owner = catalogue.owner_of(from.group)?;
    if from.id == into.id {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if from.version != input.version || into.version != input.into_version {
        // The record that changed is the one to tell about: *Use mine* sends its version next.
        let changed = if into.version == input.into_version {
            from.id
        } else {
            into.id
        };
        return Err(AppError::stale(
            entity(RecordKind::Label, changed),
            Place::from(catalogue.owner_of(from.group)?),
            actor.person.id,
        ));
    }
    ctx.ensure_rewrite_in_reach(&shown, &from).await?;
    let into_pick_one = into
        .group
        .is_some_and(|group| catalogue.group(group).is_ok_and(|found| found.pick_one));
    let crowd_ids = into
        .group
        .filter(|_| into_pick_one)
        .map(|group| crowd(&catalogue, group, Some(from.id), Some(into.id)))
        .unwrap_or_default();
    if into_pick_one && ctx.any_issue_has_two(crowd_ids.clone()).await? {
        return Err(ErrorCode::PickOneConflict.into());
    }
    let rewrite = Rewrite {
        sorted: Sorted::Label,
        from: from.id.into(),
        rename: None,
        into: Some(Replacement {
            id: into.id.into(),
            named: NameRef {
                name: into.name.clone(),
                group: into.group,
            },
        }),
    };
    let Combining { event, events, why } =
        ctx.combine_events(&actor, Sorted::Label, from.id.into(), into.id.into())?;
    let (ids, versions) = ((from.id, into.id), (from.version, into.version));
    let (stopped, reach) = (why.clone(), Reach::of(&shown));
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_tags_and_labels(transaction).await?;
                let now = S::labels_in(transaction).await?;
                let current = |id| {
                    now.iter()
                        .find(|label| label.id == id)
                        .map(|label| label.version)
                };
                if current(ids.0) != Some(versions.0) || current(ids.1) != Some(versions.1) {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                if into_pick_one && two_of_in::<S>(transaction, &crowd_ids).await? {
                    return Err(stopped.stop::<S>(ErrorCode::PickOneConflict));
                }
                reach.ensure_in::<S>(transaction, ids.0, &stopped).await?;
                rewrite.append_events::<S>(transaction, &events).await?;
                S::retarget_issue_labels(transaction, ids.0, ids.1).await?;
                S::delete_label(transaction, ids.0).await?;
                Ok(())
            })
        })
        .await
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                &entity(RecordKind::Label, ids.0),
                Place::from(owner),
                actor.person.id,
            )
        })?;
    let stored = ctx.label_catalogue().await?;
    Ok(held.authorize(LabelView::from(stored.label(into.id)?)))
}
