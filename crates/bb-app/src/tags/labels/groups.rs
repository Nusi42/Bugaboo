//! Making and changing a group of labels (TAG-02).

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::LabelGroupId;
use bb_core::offer::has_room;
use bb_core::tags::{LABEL_GROUPS_MAX, LabelGroup};
use bb_core::version::FIRST_VERSION;
use bb_core::version::Version;
use bb_policy::{Authorized, Permission, Place};
use jiff::Timestamp;
use serde_json::json;

use super::super::types::{ChangeLabelGroup, CreateLabelGroup, LabelGroupView};
use super::{crowd, group_name_taken, retired_state, two_of_in};
use crate::auth::{AppError, AuthStore, Ctx, by, entity, set_to};
use crate::input::clean_tag_name;
use crate::member::label_group as group_member;
use crate::projects::{Why, tree_in};

/// Creates a group of labels, at the end of the order, for the whole organization or in a project.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags* where it is created, `request.invalid` for a
/// name that cannot be one, `limit.reached` for as many groups as the place may have, `name.taken`
/// for a name another group that can meet it has.
pub async fn create_label_group<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateLabelGroup,
) -> Result<Authorized<LabelGroupView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let place = ctx.offer_place(input.project.as_deref()).await?;
    let held = ctx.require(&actor, Permission::ManageLabels, place).await?;
    let owner = place.project();
    let name = clean_tag_name(&input.name)?;
    let catalogue = ctx.label_catalogue().await?;
    let tree = ctx.project_tree_now().await?;
    if !has_room(
        catalogue.groups.iter().map(|found| found.project),
        owner,
        LABEL_GROUPS_MAX,
    ) {
        return Err(ErrorCode::LimitReached.into());
    }
    if group_name_taken(&catalogue.groups, &tree, &name, owner, None) {
        return Err(ErrorCode::NameTaken.into());
    }
    let group = LabelGroup {
        id: LabelGroupId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        name,
        pick_one: input.pick_one,
        project: owner,
        position: catalogue
            .groups
            .iter()
            .map(|found| found.position)
            .max()
            .map_or(0, |last| last + 1),
        retired_at: None,
        version: FIRST_VERSION,
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::LabelGroup, group.id),
        vec![
            set_to(group_member::NAME, json!(group.name)),
            set_to(group_member::PICK_ONE, json!(group.pick_one)),
            set_to(
                group_member::PROJECT,
                json!(owner.map(|project| project.to_string())),
            ),
        ],
    )?;
    let (why, stored) = (Why::default(), group.clone());
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_tags_and_labels(transaction).await?;
                let groups = S::label_groups_in(transaction).await?;
                let tree = tree_in::<S>(transaction).await?;
                if !has_room(
                    groups.iter().map(|found| found.project),
                    stored.project,
                    LABEL_GROUPS_MAX,
                ) {
                    return Err(stopped.stop::<S>(ErrorCode::LimitReached));
                }
                if group_name_taken(&groups, &tree, &stored.name, stored.project, None) {
                    return Err(stopped.stop::<S>(ErrorCode::NameTaken));
                }
                S::insert_label_group(transaction, &stored).await
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NameTaken))?;
    Ok(held.authorize(ctx.label_group_after(&actor, &group).await?))
}

/// Changes a group of labels: its name, whether it is a *Pick one* group, whether it is retired.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags* where the group was made,
/// `resource.not-found` for a group that is not there, `request.invalid` for nothing asked or a
/// name that cannot be one, `name.taken`,
/// `label.pick-one-conflict` when some issue has two labels of the group, and the group is to be
/// a *Pick one* group, `edit.version-changed` when somebody changed the group since it was looked
/// at.
pub async fn change_label_group<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeLabelGroup,
) -> Result<Authorized<LabelGroupView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let shown = ctx.shown_to(&actor).await?;
    let catalogue = ctx.label_catalogue().await?;
    let group = catalogue.group(input.group)?.clone();
    let held = ctx
        .label_manager(&actor, &shown, &catalogue, Some(group.id))
        .await?;
    if input.name.is_none() && input.pick_one.is_none() && input.retired.is_none() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if group.version != input.version {
        return Err(AppError::stale(
            entity(RecordKind::LabelGroup, group.id),
            Place::from(group.project),
            actor.person.id,
        ));
    }
    let name = match &input.name {
        Some(name) => clean_tag_name(name)?,
        None => group.name.clone(),
    };
    let tree = ctx.project_tree_now().await?;
    if group_name_taken(
        &catalogue.groups,
        &tree,
        &name,
        group.project,
        Some(group.id),
    ) {
        return Err(ErrorCode::NameTaken.into());
    }
    let pick_one = input.pick_one.unwrap_or(group.pick_one);
    let now = ctx.now();
    let retired_at = retired_state(group.retired_at, input.retired, now);
    if pick_one
        && !group.pick_one
        && ctx
            .any_issue_has_two(crowd(&catalogue, group.id, None, None))
            .await?
    {
        return Err(ErrorCode::PickOneConflict.into());
    }
    let changes = group_differences(&group, &name, pick_one, retired_at);
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::LabelGroup, group.id),
        changes,
    )?;
    let (id, version, why) = (group.id, group.version, Why::default());
    let (stopped, written, owner) = (why.clone(), name.clone(), group.project);
    let watch_conflict = pick_one && !group.pick_one;
    let crowd_ids = crowd(&catalogue, group.id, None, None);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_tags_and_labels(transaction).await?;
                let groups = S::label_groups_in(transaction).await?;
                let tree = tree_in::<S>(transaction).await?;
                if group_name_taken(&groups, &tree, &written, owner, Some(id)) {
                    return Err(stopped.stop::<S>(ErrorCode::NameTaken));
                }
                if watch_conflict && two_of_in::<S>(transaction, &crowd_ids).await? {
                    return Err(stopped.stop::<S>(ErrorCode::PickOneConflict));
                }
                if S::update_label_group(
                    transaction,
                    id,
                    &written,
                    pick_one,
                    retired_at,
                    Version::new(version),
                )
                .await?
                {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::VersionChanged))
                }
            })
        })
        .await
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                &entity(RecordKind::LabelGroup, id),
                Place::from(group.project),
                actor.person.id,
            )
        })?;
    let stored = ctx.label_catalogue().await?;
    Ok(held.authorize(ctx.label_group_after(&actor, stored.group(id)?).await?))
}

/// What a change of a group of labels changes, for its event.
fn group_differences(
    group: &LabelGroup,
    name: &str,
    pick_one: bool,
    retired_at: Option<Timestamp>,
) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    if group.name != name {
        changes.push(FieldChange {
            field: group_member::NAME.to_owned(),
            before: Some(json!(group.name)),
            after: Some(json!(name)),
        });
    }
    if group.pick_one != pick_one {
        changes.push(FieldChange {
            field: group_member::PICK_ONE.to_owned(),
            before: Some(json!(group.pick_one)),
            after: Some(json!(pick_one)),
        });
    }
    if group.retired_at.is_some() != retired_at.is_some() {
        changes.push(FieldChange {
            field: group_member::RETIRED.to_owned(),
            before: Some(json!(group.retired_at.is_some())),
            after: Some(json!(retired_at.is_some())),
        });
    }
    changes
}
