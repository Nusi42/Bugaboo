//! The organization's labels (TAG-02, TAG-03): the list, groups of labels with *Pick one*, making
//! and changing them, renaming and combining labels across all issues.
//!
//! Everybody who is signed in reads the list (the people who put labels on issues choose from it);
//! the people with *Manage labels and tags* for the whole organization keep it. A name is used once
//! in a group (and once among the labels that sit in no group), without regard to capitals. A label
//! is retired, never deleted, except by combining it with another one: old issues keep it and new
//! ones are not offered it. A group can only be a *Pick one* group, and a label can only join one,
//! when no issue would then hold two labels of it.

use std::collections::HashMap;

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::{EntityId, IssueId, LabelGroupId, LabelId};
use bb_core::own_fields::{ChoiceColor, Edit};
use bb_core::tags::{LABEL_GROUPS_MAX, LABELS_MAX, Label, LabelGroup, TAG_NAME_MAX, name_key};
use bb_core::version::FIRST_VERSION;
use bb_policy::{Authorized, Place};
use jiff::Timestamp;
use serde_json::json;

use super::rewrite::{Replacement, Rewrite, Sorted};
use super::types::{
    ChangeLabel, ChangeLabelGroup, CombineLabels, LabelChangePreviewQuery, LabelGroupView,
    LabelView, Labels, LabelsQuery, MakeLabel, MakeLabelGroup, TagChangePreview,
};
use crate::auth::{AppError, AuthStore, Ctx, EventMaker, by, entity, internal, set_to};
use crate::input::clean_text;
use crate::member::{label as member, label_group as group_member};
use crate::projects::Why;

/// How many issues are read at a time while the labels of a group are counted per issue.
const CHUNK: u64 = 200;

/// The groups and the labels as they are stored.
struct Catalogue {
    groups: Vec<LabelGroup>,
    labels: Vec<Label>,
}

impl Catalogue {
    fn label(&self, id: LabelId) -> Result<&Label, ErrorCode> {
        self.labels
            .iter()
            .find(|label| label.id == id)
            .ok_or(ErrorCode::NotFound)
    }

    fn group(&self, id: LabelGroupId) -> Result<&LabelGroup, ErrorCode> {
        self.groups
            .iter()
            .find(|group| group.id == id)
            .ok_or(ErrorCode::NotFound)
    }

    /// Whether another label has this name in the group (or among those in no group).
    fn label_name_taken(
        &self,
        name: &str,
        group: Option<LabelGroupId>,
        except: Option<LabelId>,
    ) -> bool {
        let key = name_key(name);
        self.labels.iter().any(|other| {
            Some(other.id) != except && other.group == group && name_key(&other.name) == key
        })
    }

    fn group_name_taken(&self, name: &str, except: Option<LabelGroupId>) -> bool {
        let key = name_key(name);
        self.groups
            .iter()
            .any(|other| Some(other.id) != except && name_key(&other.name) == key)
    }
}

/// Whether some issue has two of these labels, read inside a change after the lock.
async fn two_of_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    labels: &[LabelId],
) -> Result<bool, S::Error> {
    let mut counted: HashMap<IssueId, usize> = HashMap::new();
    for label in labels {
        let mut after = None;
        loop {
            let issues = S::issues_with_label_in(transaction, *label, after, CHUNK).await?;
            for issue in &issues {
                let seen = counted.entry(*issue).or_insert(0);
                *seen += 1;
                if *seen >= 2 {
                    return Ok(true);
                }
            }
            if issues.len() < usize::try_from(CHUNK).unwrap_or(usize::MAX) {
                break;
            }
            after = issues.last().copied();
        }
    }
    Ok(false)
}

/// The labels that an issue could hold two of if `group` were a *Pick one* group with `joining`
/// added to it and `leaving` taken out of the count.
fn crowd(
    catalogue: &Catalogue,
    group: LabelGroupId,
    joining: Option<LabelId>,
    leaving: Option<LabelId>,
) -> Vec<LabelId> {
    let mut set: Vec<LabelId> = catalogue
        .labels
        .iter()
        .filter(|label| label.group == Some(group) && Some(label.id) != leaving)
        .map(|label| label.id)
        .collect();
    if let Some(joining) = joining
        && !set.contains(&joining)
    {
        set.push(joining);
    }
    set
}

impl<S: AuthStore> Ctx<'_, S> {
    async fn label_catalogue(&self) -> Result<Catalogue, AppError> {
        let store = self.store();
        Ok(Catalogue {
            groups: store
                .label_groups()
                .await
                .map_err(|error| internal::<S>(&error))?,
            labels: store
                .labels()
                .await
                .map_err(|error| internal::<S>(&error))?,
        })
    }

    /// Whether some issue has two of the labels now.
    async fn any_issue_has_two(&self, labels: Vec<LabelId>) -> Result<bool, AppError> {
        self.store()
            .read_snapshot(move |transaction| {
                Box::pin(async move { two_of_in::<S>(transaction, &labels).await })
            })
            .await
            .map_err(|error| internal::<S>(&error))
    }
}

/// Lists the labels of the organization with their groups. Anybody who is signed in may.
///
/// # Errors
/// `not_signed_in` without a session.
pub async fn labels<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: LabelsQuery,
) -> Result<Authorized<Labels>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let catalogue = ctx.label_catalogue().await?;
    Ok(held.authorize(Labels {
        groups: catalogue.groups.iter().map(LabelGroupView::from).collect(),
        labels: catalogue.labels.iter().map(LabelView::from).collect(),
        may_manage: held.allows(bb_policy::Permission::ManageLabels),
    }))
}

/// Makes a group of labels, at the end of the order.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags*, `request.invalid` for a name that cannot be
/// one or as many groups as there may be, `name.taken` for a name another group has.
pub async fn make_label_group<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MakeLabelGroup,
) -> Result<Authorized<LabelGroupView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx
        .require(
            &actor,
            bb_policy::Permission::ManageLabels,
            Place::Organization,
        )
        .await?;
    let name = clean_text(&input.name, TAG_NAME_MAX)?;
    let catalogue = ctx.label_catalogue().await?;
    if catalogue.groups.len() >= LABEL_GROUPS_MAX {
        return Err(ErrorCode::LimitReached.into());
    }
    if catalogue.group_name_taken(&name, None) {
        return Err(ErrorCode::NameTaken.into());
    }
    let group = LabelGroup {
        id: LabelGroupId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        name,
        pick_one: input.pick_one,
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
        ],
    )?;
    let (why, stored) = (Why::default(), group.clone());
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_tags_and_labels(transaction).await?;
                let groups = S::label_groups_in(transaction).await?;
                let key = name_key(&stored.name);
                if groups.iter().any(|other| name_key(&other.name) == key) {
                    return Err(stopped.stop::<S>(ErrorCode::NameTaken));
                }
                S::insert_label_group(transaction, &stored).await
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NameTaken))?;
    Ok(held.authorize(LabelGroupView::from(&group)))
}

/// Changes a group of labels: its name, whether it is a *Pick one* group, whether it is retired.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags*, `resource.not_found` for a group that is
/// not there, `request.invalid` for nothing asked or a name that cannot be one, `name.taken`,
/// `label.pick_one_conflict` when some issue has two labels of the group, and the group is to be
/// a *Pick one* group, `edit.version_changed` when somebody changed the group since it was looked
/// at.
pub async fn change_label_group<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeLabelGroup,
) -> Result<Authorized<LabelGroupView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx
        .require(
            &actor,
            bb_policy::Permission::ManageLabels,
            Place::Organization,
        )
        .await?;
    if input.name.is_none() && input.pick_one.is_none() && input.retired.is_none() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let catalogue = ctx.label_catalogue().await?;
    let group = catalogue.group(input.group)?.clone();
    if group.version != input.version {
        return Err(ErrorCode::VersionChanged.into());
    }
    let name = match &input.name {
        Some(name) => clean_text(name, TAG_NAME_MAX)?,
        None => group.name.clone(),
    };
    if catalogue.group_name_taken(&name, Some(group.id)) {
        return Err(ErrorCode::NameTaken.into());
    }
    let pick_one = input.pick_one.unwrap_or(group.pick_one);
    let now = ctx.now();
    let retired_at = match input.retired {
        Some(true) => group.retired_at.or(Some(now)),
        Some(false) => None,
        None => group.retired_at,
    };
    if pick_one
        && !group.pick_one
        && ctx
            .any_issue_has_two(crowd(&catalogue, group.id, None, None))
            .await?
    {
        return Err(ErrorCode::PickOneConflict.into());
    }
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
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::LabelGroup, group.id),
        changes,
    )?;
    let (id, version, why) = (group.id, group.version, Why::default());
    let (stopped, written) = (why.clone(), name.clone());
    let watch_conflict = pick_one && !group.pick_one;
    let crowd_ids = crowd(&catalogue, group.id, None, None);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_tags_and_labels(transaction).await?;
                let groups = S::label_groups_in(transaction).await?;
                let key = name_key(&written);
                if groups
                    .iter()
                    .any(|other| other.id != id && name_key(&other.name) == key)
                {
                    return Err(stopped.stop::<S>(ErrorCode::NameTaken));
                }
                if watch_conflict && two_of_in::<S>(transaction, &crowd_ids).await? {
                    return Err(stopped.stop::<S>(ErrorCode::PickOneConflict));
                }
                if S::update_label_group(transaction, id, &written, pick_one, retired_at, version)
                    .await?
                {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::VersionChanged))
                }
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::VersionChanged))?;
    let stored = ctx.label_catalogue().await?;
    Ok(held.authorize(LabelGroupView::from(stored.group(id)?)))
}

/// Makes a label, at the end of the order, with the next color in turn when none is asked for.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags*, `resource.not_found` for a group that is
/// not there, `request.invalid` for a name that cannot be one or a retired group, `name.taken` for
/// a name the group has, `limit.reached` for as many labels as there may be.
pub async fn make_label<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MakeLabel,
) -> Result<Authorized<LabelView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx
        .require(
            &actor,
            bb_policy::Permission::ManageLabels,
            Place::Organization,
        )
        .await?;
    let name = clean_text(&input.name, TAG_NAME_MAX)?;
    let catalogue = ctx.label_catalogue().await?;
    if catalogue.labels.len() >= LABELS_MAX {
        return Err(ErrorCode::LimitReached.into());
    }
    if let Some(group) = input.group
        && catalogue.group(group)?.retired_at.is_some()
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if catalogue.label_name_taken(&name, input.group, None) {
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
                let key = name_key(&stored.name);
                if labels
                    .iter()
                    .any(|other| other.group == stored.group && name_key(&other.name) == key)
                {
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
/// `access.forbidden` without *Manage labels and tags*, `resource.not_found` for a label that is
/// not there, `request.invalid` for a name that cannot be one or for nothing asked, `name.taken`
/// for a name another label of the group has.
pub async fn label_change_preview<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: LabelChangePreviewQuery,
) -> Result<Authorized<TagChangePreview>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx
        .require(
            &actor,
            bb_policy::Permission::ManageLabels,
            Place::Organization,
        )
        .await?;
    let catalogue = ctx.label_catalogue().await?;
    let label = catalogue.label(input.label)?;
    let rewrite = match (&input.name, input.into) {
        (Some(name), None) => {
            let name = clean_text(name, TAG_NAME_MAX)?;
            if catalogue.label_name_taken(&name, label.group, Some(label.id)) {
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
            Rewrite {
                sorted: Sorted::Label,
                from: label.id.into(),
                rename: None,
                into: Some(Replacement {
                    id: into.id.into(),
                    name: into.name.clone(),
                }),
            }
        }
        _ => return Err(ErrorCode::InvalidRequest.into()),
    };
    let preview = ctx.preview_rewrite(&actor, rewrite).await?;
    Ok(held.authorize(preview))
}

/// Changes a label: its name (every issue that has it records the new name), color, group and
/// whether it is retired. What is left out stays as it is.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags*, `resource.not_found` for a label or a group
/// that is not there, `request.invalid` for nothing asked, a name that cannot be one or a retired
/// group, `name.taken`, `label.pick_one_conflict` when joining a *Pick one* group would give some
/// issue two of its labels, `edit.version_changed` when somebody changed the label since it was
/// looked at.
pub async fn change_label<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeLabel,
) -> Result<Authorized<LabelView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx
        .require(
            &actor,
            bb_policy::Permission::ManageLabels,
            Place::Organization,
        )
        .await?;
    if input.name.is_none()
        && input.color.is_none()
        && input.group.is_keep()
        && input.retired.is_none()
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let catalogue = ctx.label_catalogue().await?;
    let label = catalogue.label(input.label)?.clone();
    if label.version != input.version {
        return Err(ErrorCode::VersionChanged.into());
    }
    let name = match &input.name {
        Some(name) => clean_text(name, TAG_NAME_MAX)?,
        None => label.name.clone(),
    };
    let color = input.color.unwrap_or(label.color);
    let group = match input.group {
        Edit::Keep => label.group,
        Edit::Clear => None,
        Edit::Set(target) => Some(target),
    };
    if group != label.group
        && let Some(target) = group
        && catalogue.group(target)?.retired_at.is_some()
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if catalogue.label_name_taken(&name, group, Some(label.id)) {
        return Err(ErrorCode::NameTaken.into());
    }
    let now = ctx.now();
    let retired_at = match input.retired {
        Some(true) => label.retired_at.or(Some(now)),
        Some(false) => None,
        None => label.retired_at,
    };
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
    };
    let (id, why) = (label.id, Why::default());
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { write.run::<S>(transaction, &events, &stopped).await })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::VersionChanged))?;
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
    let held = ctx
        .require(
            &actor,
            bb_policy::Permission::ManageLabels,
            Place::Organization,
        )
        .await?;
    let catalogue = ctx.label_catalogue().await?;
    let (from, into) = (
        catalogue.label(input.label)?.clone(),
        catalogue.label(input.into)?.clone(),
    );
    if from.id == into.id {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if from.version != input.version || into.version != input.into_version {
        return Err(ErrorCode::VersionChanged.into());
    }
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
            name: into.name.clone(),
        }),
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Label, from.id),
        vec![set_to(
            member::COMBINED_INTO,
            json!(EntityId::from(into.id).to_string()),
        )],
    )?;
    let events = ctx.event_maker(by(actor.person.id));
    let (ids, versions, why) = (
        (from.id, into.id),
        (from.version, into.version),
        Why::default(),
    );
    let stopped = why.clone();
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
                rewrite.append_events::<S>(transaction, &events).await?;
                S::retarget_issue_labels(transaction, ids.0, ids.1).await?;
                S::delete_label(transaction, ids.0).await?;
                Ok(())
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::VersionChanged))?;
    let stored = ctx.label_catalogue().await?;
    Ok(held.authorize(LabelView::from(stored.label(into.id)?)))
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
        let key = name_key(&self.name);
        if labels.iter().any(|other| {
            other.id != self.id && other.group == self.group && name_key(&other.name) == key
        }) {
            return Err(stopped.stop::<S>(ErrorCode::NameTaken));
        }
        if self.joins_pick_one && two_of_in::<S>(transaction, &self.crowd_ids).await? {
            return Err(stopped.stop::<S>(ErrorCode::PickOneConflict));
        }
        if let Some(rewrite) = &self.rewrite {
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
