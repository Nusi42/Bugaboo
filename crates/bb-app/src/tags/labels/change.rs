//! Making a label, what renaming one would do, and changing one (TAG-02, TAG-03).

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::{LabelGroupId, LabelId};
use bb_core::own_fields::{ChoiceColor, Edit};
use bb_core::tags::Label;
use bb_core::version::FIRST_VERSION;
use bb_policy::{Authorized, Place};
use jiff::Timestamp;
use serde_json::json;

use super::super::refs::NameRef;
use super::super::rewrite::{Replacement, Rewrite, Sorted};
use super::super::types::{
    ChangeLabel, CreateLabel, LabelChangePreviewQuery, LabelView, TagChangePreview,
};
use super::{Reach, crowd, label_name_taken, retired_state, two_of_in};
use crate::auth::{AppError, AuthStore, Ctx, EventMaker, by, entity, set_to};
use crate::input::clean_tag_name;
use crate::member::label as member;
use crate::projects::Why;

/// Creates a label, at the end of the order, with the next color in turn when none is asked for.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags* where the group was created,
/// `resource.not-found` for a group that is not there, `request.invalid` for a name that cannot be
/// one or a retired group, `name.taken` for a name the group has, `limit.reached` for as many
/// labels as the place of the group may have.
pub async fn create_label<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateLabel,
) -> Result<Authorized<LabelView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let shown = ctx.shown_to(&actor).await?;
    let catalogue = ctx.label_catalogue().await?;
    let held = ctx
        .label_manager(&actor, &shown, &catalogue, input.group)
        .await?;
    let name = clean_tag_name(&input.name)?;
    if !catalogue.has_room_for_label(input.group)? {
        return Err(ErrorCode::LimitReached.into());
    }
    if let Some(group) = input.group
        && catalogue.group(group)?.retired_at.is_some()
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if label_name_taken(&catalogue.labels, &name, input.group, None) {
        return Err(ErrorCode::NameTaken.into());
    }
    let label = Label {
        id: LabelId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        group: input.group,
        name,
        color: input
            .color
            .unwrap_or_else(|| ChoiceColor::nth(catalogue.labels.len())),
        position: catalogue
            .labels
            .iter()
            .map(|found| found.position)
            .max()
            .map_or(0, |last| last + 1),
        retired_at: None,
        version: FIRST_VERSION,
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Label, label.id),
        vec![
            set_to(member::NAME, json!(label.name)),
            set_to(member::COLOR, json!(label.color.code())),
        ],
    )?;
    let (why, stored) = (Why::default(), label.clone());
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_tags_and_labels(transaction).await?;
                let labels = S::labels_in(transaction).await?;
                if label_name_taken(&labels, &stored.name, stored.group, None) {
                    return Err(stopped.stop::<S>(ErrorCode::NameTaken));
                }
                S::insert_label(transaction, &stored).await
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NameTaken))?;
    Ok(held.authorize(LabelView::from(&label)))
}

/// What renaming a label, or combining it with another, would do to the issues the person sees.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags*, `resource.not-found` for a label that is
/// not there, `request.invalid` for a name that cannot be one or for nothing asked, `name.taken`
/// for a name another label of the group has.
pub async fn label_change_preview<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: LabelChangePreviewQuery,
) -> Result<Authorized<TagChangePreview>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let shown = ctx.shown_to(&actor).await?;
    let catalogue = ctx.label_catalogue().await?;
    let label = catalogue.label(input.label)?;
    let held = ctx
        .label_manager(&actor, &shown, &catalogue, label.group)
        .await?;
    let rewrite = match (&input.name, input.into) {
        (Some(name), None) => {
            let name = clean_tag_name(name)?;
            if label_name_taken(&catalogue.labels, &name, label.group, Some(label.id)) {
                return Err(ErrorCode::NameTaken.into());
            }
            Rewrite {
                sorted: Sorted::Label,
                from: label.id.into(),
                rename: Some(name),
                into: None,
            }
        }
        (None, Some(into)) => {
            let into = catalogue.label(into)?;
            if into.id == label.id {
                return Err(ErrorCode::InvalidRequest.into());
            }
            // The issues of both labels change, so the person looks after both.
            ctx.label_manager(&actor, &shown, &catalogue, into.group)
                .await?;
            Rewrite {
                sorted: Sorted::Label,
                from: label.id.into(),
                rename: None,
                into: Some(Replacement {
                    id: into.id.into(),
                    named: NameRef::from(into),
                }),
            }
        }
        _ => return Err(ErrorCode::InvalidRequest.into()),
    };
    ctx.ensure_rewrite_in_reach(&shown, label).await?;
    let preview = ctx.preview_rewrite(&actor, rewrite).await?;
    Ok(held.authorize(preview))
}

/// Changes a label: its name (every issue that has it records the new name), color, group and
/// whether it is retired. What is left out stays as it is.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags*, `resource.not-found` for a label or a group
/// that is not there, `request.invalid` for nothing asked, a name that cannot be one or a retired
/// group, `name.taken`, `label.pick-one-conflict` when joining a *Pick one* group would give some
/// issue two of its labels, `edit.version-changed` when somebody changed the label since it was
/// looked at.
pub async fn change_label<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeLabel,
) -> Result<Authorized<LabelView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let shown = ctx.shown_to(&actor).await?;
    let catalogue = ctx.label_catalogue().await?;
    let label = catalogue.label(input.label)?.clone();
    let held = ctx
        .label_manager(&actor, &shown, &catalogue, label.group)
        .await?;
    if input.name.is_none()
        && input.color.is_none()
        && input.group.is_keep()
        && input.retired.is_none()
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if label.version != input.version {
        return Err(AppError::stale(
            entity(RecordKind::Label, label.id),
            Place::from(catalogue.owner_of(label.group)?),
            actor.person.id,
        ));
    }
    let name = match &input.name {
        Some(name) => clean_tag_name(name)?,
        None => label.name.clone(),
    };
    let color = input.color.unwrap_or(label.color);
    let group = match input.group {
        Edit::Keep => label.group,
        Edit::Clear => None,
        Edit::Set(target) => Some(target),
    };
    if group != label.group {
        // A label that moves into a group is looked after there from now on.
        ctx.label_manager(&actor, &shown, &catalogue, group).await?;
        if let Some(target) = group
            && catalogue.group(target)?.retired_at.is_some()
        {
            return Err(ErrorCode::InvalidRequest.into());
        }
    }
    if label_name_taken(&catalogue.labels, &name, group, Some(label.id)) {
        return Err(ErrorCode::NameTaken.into());
    }
    let now = ctx.now();
    let retired_at = retired_state(label.retired_at, input.retired, now);
    let joins_pick_one = group != label.group
        && group.is_some_and(|target| catalogue.group(target).is_ok_and(|found| found.pick_one));
    let crowd_ids = group
        .filter(|_| joins_pick_one)
        .map(|target| crowd(&catalogue, target, Some(label.id), None))
        .unwrap_or_default();
    if joins_pick_one && ctx.any_issue_has_two(crowd_ids.clone()).await? {
        return Err(ErrorCode::PickOneConflict.into());
    }
    let changes = label_differences(&label, &name, color, group, retired_at);
    if label.name != name {
        ctx.ensure_rewrite_in_reach(&shown, &label).await?;
    }
    let rewrite = (label.name != name).then(|| Rewrite {
        sorted: Sorted::Label,
        from: label.id.into(),
        rename: Some(name.clone()),
        into: None,
    });
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Label, label.id),
        changes,
    )?;
    let events = ctx.event_maker(by(actor.person.id));
    let write = LabelWrite {
        id: label.id,
        version: label.version,
        name: name.clone(),
        color,
        group,
        retired_at,
        joins_pick_one,
        crowd_ids,
        rewrite,
        reach: Reach::of(&shown),
    };
    let (id, why) = (label.id, Why::default());
    let owner = catalogue.owner_of(label.group)?;
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { write.run::<S>(transaction, &events, &stopped).await })
        })
        .await
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                &entity(RecordKind::Label, id),
                Place::from(owner),
                actor.person.id,
            )
        })?;
    let stored = ctx.label_catalogue().await?;
    Ok(held.authorize(LabelView::from(stored.label(id)?)))
}

/// What a change of a label changes, for its event.
fn label_differences(
    label: &Label,
    name: &str,
    color: ChoiceColor,
    group: Option<LabelGroupId>,
    retired_at: Option<Timestamp>,
) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    if label.name != name {
        changes.push(FieldChange {
            field: member::NAME.to_owned(),
            before: Some(json!(label.name)),
            after: Some(json!(name)),
        });
    }
    if label.color != color {
        changes.push(FieldChange {
            field: member::COLOR.to_owned(),
            before: Some(json!(label.color.code())),
            after: Some(json!(color.code())),
        });
    }
    if label.group != group {
        changes.push(FieldChange {
            field: member::GROUP.to_owned(),
            before: label.group.map(|group| json!(group.to_string())),
            after: group.map(|group| json!(group.to_string())),
        });
    }
    if label.retired_at.is_some() != retired_at.is_some() {
        changes.push(FieldChange {
            field: member::RETIRED.to_owned(),
            before: Some(json!(label.retired_at.is_some())),
            after: Some(json!(retired_at.is_some())),
        });
    }
    changes
}

/// What the change of a label writes.
struct LabelWrite {
    id: LabelId,
    version: i64,
    name: String,
    color: ChoiceColor,
    group: Option<LabelGroupId>,
    retired_at: Option<Timestamp>,
    joins_pick_one: bool,
    crowd_ids: Vec<LabelId>,
    rewrite: Option<Rewrite>,
    reach: Reach,
}

impl LabelWrite {
    /// Writes the change inside it, after the lock: judged again on what is stored now (another
    /// label may have taken the name, an issue may have got a second label of the group), with the
    /// event of every issue the new name touches.
    async fn run<S: AuthStore>(
        self,
        transaction: &mut S::Transaction,
        events: &EventMaker,
        stopped: &Why,
    ) -> Result<(), S::Error> {
        S::lock_tags_and_labels(transaction).await?;
        let labels = S::labels_in(transaction).await?;
        if label_name_taken(&labels, &self.name, self.group, Some(self.id)) {
            return Err(stopped.stop::<S>(ErrorCode::NameTaken));
        }
        if self.joins_pick_one && two_of_in::<S>(transaction, &self.crowd_ids).await? {
            return Err(stopped.stop::<S>(ErrorCode::PickOneConflict));
        }
        if let Some(rewrite) = &self.rewrite {
            self.reach
                .ensure_in::<S>(transaction, self.id, stopped)
                .await?;
            rewrite.append_events::<S>(transaction, events).await?;
        }
        if S::update_label(
            transaction,
            self.id,
            &self.name,
            self.color,
            self.group,
            self.retired_at,
            self.version,
        )
        .await?
        {
            Ok(())
        } else {
            Err(stopped.stop::<S>(ErrorCode::VersionChanged))
        }
    }
}
