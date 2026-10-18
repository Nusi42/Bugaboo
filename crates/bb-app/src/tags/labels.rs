//! The organization's labels (TAG-02, TAG-03): the list, groups of labels with *Pick one*, making
//! and changing them, renaming and combining labels across all issues.
//!
//! Everybody who is signed in reads the list (the people who put labels on issues choose from it),
//! as far as they may know a group (`offers/known.rs`). A group of labels, and the labels in it, are
//! looked after by whoever holds *Manage labels and tags* where the group was created: the whole
//! organization, or one project (M3-F122); labels that sit in no group belong to the whole
//! organization. The name of a group is used once among the groups that can meet in one project
//! (`bb_core::offer::can_meet`); the name of a label once in a group (and once among the labels
//! that sit in no group), without regard to capitals. A place has its own limits of groups and
//! labels. A label
//! is retired, never deleted, except by combining it with another one: old issues keep it and new
//! ones are not offered it. A group can only be a *Pick one* group, and a label can only join one,
//! when no issue would then hold two labels of it.

use std::collections::HashMap;
use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::{IssueId, LabelGroupId, LabelId, ProjectId};
use bb_core::offer::{can_meet, has_room};
use bb_core::own_fields::{ChoiceColor, Edit};
use bb_core::paging::window;
use bb_core::tags::{LABEL_GROUPS_MAX, LABELS_MAX, Label, LabelGroup, name_key};
use bb_core::version::FIRST_VERSION;
use bb_policy::{Authorized, Evaluation, Permission, Place, Tree};
use jiff::Timestamp;
use serde_json::json;

use super::refs::NameRef;
use super::rewrite::{Combining, ISSUES_PER_READ, Replacement, Rewrite, Sorted};
use super::types::{
    ChangeLabel, ChangeLabelGroup, CombineLabels, CreateLabel, CreateLabelGroup,
    LabelChangePreviewQuery, LabelGroupPage, LabelGroupView, LabelGroupsQuery, LabelPage,
    LabelView, LabelsQuery, TagChangePreview,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, EventMaker, by, entity, internal, set_to};
use crate::input::clean_tag_name;
use crate::member::{label as member, label_group as group_member};
use crate::offers::{Here, Standing, Subject};
use crate::projects::{Shown, Why, tree_in};

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

    /// The project a label's group was made in; none for a label in no group.
    fn owner_of(&self, group: Option<LabelGroupId>) -> Result<Option<ProjectId>, ErrorCode> {
        group.map_or(Ok(None), |group| {
            self.group(group).map(|found| found.project)
        })
    }

    /// Whether one more label fits in the place its group was created in (none for a label in no
    /// group).
    fn has_room_for_label(&self, group: Option<LabelGroupId>) -> Result<bool, ErrorCode> {
        let owner = self.owner_of(group)?;
        Ok(has_room(
            self.labels
                .iter()
                .map(|label| self.owner_of(label.group).ok().flatten()),
            owner,
            LABELS_MAX,
        ))
    }
}

/// Whether another group than `except` that can meet this one in a project already has the name,
/// without regard to capitals.
fn group_name_taken(
    groups: &[LabelGroup],
    tree: &Tree,
    name: &str,
    owner: Option<ProjectId>,
    except: Option<LabelGroupId>,
) -> bool {
    let key = name_key(name);
    groups.iter().any(|other| {
        Some(other.id) != except
            && name_key(&other.name) == key
            && can_meet(owner, other.project, |inside, outside| {
                tree.is_within(inside, outside)
            })
    })
}

/// The projects whose issues a person may rewrite labels in: all of them for whoever holds *Manage
/// labels and tags* for the whole organization, else the projects where they hold it.
#[derive(Clone)]
struct Reach(Option<Vec<ProjectId>>);

impl Reach {
    fn of(shown: &Shown) -> Self {
        if shown.holds(Place::Organization, Permission::ManageLabels) {
            return Self(None);
        }
        Self(Some(shown.holding(Permission::ManageLabels)))
    }

    /// Whether every one of these projects is in reach.
    fn covers(&self, projects: &[ProjectId]) -> bool {
        self.0
            .as_ref()
            .is_none_or(|own| projects.iter().all(|project| own.contains(project)))
    }

    /// The same, inside a change after the lock, on the issues as they are stored now: a label
    /// that was put on an issue of another project meanwhile is seen. Stops with `access.forbidden`.
    async fn ensure_in<S: AuthStore>(
        &self,
        transaction: &mut S::Transaction,
        label: LabelId,
        stopped: &Why,
    ) -> Result<(), S::Error> {
        if self.0.is_none() || self.covers(&S::projects_with_label_in(transaction, label).await?) {
            Ok(())
        } else {
            Err(stopped.stop::<S>(ErrorCode::Forbidden))
        }
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
            let issues =
                S::issues_with_label_in(transaction, *label, after, ISSUES_PER_READ).await?;
            for issue in &issues {
                let seen = counted.entry(*issue).or_insert(0);
                *seen += 1;
                if *seen >= 2 {
                    return Ok(true);
                }
            }
            if issues.len() < usize::try_from(ISSUES_PER_READ).unwrap_or(usize::MAX) {
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
    /// What the person who looks after the labels of a group needs to judge a change: they may know
    /// the group, and hold *Manage labels and tags* where the group was created, or for the whole
    /// organization for a label in no group.
    async fn label_manager(
        &self,
        actor: &Actor,
        shown: &Shown,
        catalogue: &Catalogue,
        group: Option<LabelGroupId>,
    ) -> Result<Arc<Evaluation>, AppError> {
        let owner = catalogue.owner_of(group)?;
        // A group the person may not know is not there for them (`offers/known.rs`).
        if let Some(group) = group {
            self.known_groups(shown, vec![catalogue.group(group)?.clone()])
                .await?
                .pop()
                .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        }
        self.require(actor, Permission::ManageLabels, Place::from(owner))
            .await
    }

    /// A group of labels after a change, as its changer sees it.
    async fn label_group_after(
        &self,
        actor: &Actor,
        group: &LabelGroup,
    ) -> Result<LabelGroupView, AppError> {
        let shown = self.shown_to(actor).await?;
        Ok(label_group_view(
            group,
            Standing::of(&shown, None, &Subject::of_group(group)),
        ))
    }

    /// Refuses (`resource.not_found`) a label that is not there or that is in a group of labels
    /// the person may not know (`offers/known.rs`).
    pub(crate) async fn label_known(&self, actor: &Actor, label: LabelId) -> Result<(), AppError> {
        let group = self.label_catalogue().await?.label(label)?.group;
        match group {
            Some(group) => self.known_group(actor, group).await.map(|_| ()),
            None => Ok(()),
        }
    }

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

    /// Refuses (`access.forbidden`, without saying how many issues) a rename or a combination of a
    /// label that would rewrite issues in a project the person does not look after: a label stays
    /// on an issue that moved out of the project of its group. The person who holds *Manage labels
    /// and tags* for the whole organization is not held back.
    async fn ensure_rewrite_in_reach(&self, shown: &Shown, label: &Label) -> Result<(), AppError> {
        let projects = self
            .store()
            .projects_with_label(label.id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        if Reach::of(shown).covers(&projects) {
            Ok(())
        } else {
            Err(ErrorCode::Forbidden.into())
        }
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

/// When a group or a label is retired after a change that says `Some(true)` (retire), `Some(false)`
/// (offer again) or `None` (as it is): a retired one keeps the moment it was retired at.
fn retired_state(
    current: Option<Timestamp>,
    asked: Option<bool>,
    now: Timestamp,
) -> Option<Timestamp> {
    match asked {
        Some(true) => current.or(Some(now)),
        Some(false) => None,
        None => current,
    }
}

/// A group of labels as the person sees it: where it was created, whether they look after it, and
/// what the project a list is about does with it.
fn label_group_view(group: &LabelGroup, standing: Standing) -> LabelGroupView {
    LabelGroupView {
        id: group.id,
        name: group.name.clone(),
        pick_one: group.pick_one,
        retired: group.retired_at.is_some(),
        created_in: standing.created_in,
        may_manage: standing.may_manage,
        here: standing.here,
        version: group.version,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The groups of labels the person may know that a list about the project holds, and the rows of
    /// that project.
    async fn listed_groups(
        &self,
        shown: &Shown,
        catalogue: &mut Catalogue,
        project: Option<&str>,
    ) -> Result<(Vec<LabelGroup>, Option<Here>), AppError> {
        let here = self.offers_here(shown, project).await?;
        let known = self
            .listed_group_rows(shown, here.as_ref(), std::mem::take(&mut catalogue.groups))
            .await?;
        Ok((known, here))
    }
}

/// Lists the groups of labels of the organization a page at a time. Anybody who is signed in may,
/// and sees the groups they may know.
///
/// # Errors
/// `request.invalid` for a cursor that is not one, `resource.not_found` for a project the person
/// may not see.
pub async fn label_groups<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: LabelGroupsQuery,
) -> Result<Authorized<LabelGroupPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let shown = ctx.shown_to(&actor).await?;
    let mut catalogue = ctx.label_catalogue().await?;
    let (groups, here) = ctx
        .listed_groups(&shown, &mut catalogue, input.project.as_deref())
        .await?;
    let page = window(groups.len(), input.cursor.as_deref(), input.limit)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    Ok(held.authorize(LabelGroupPage {
        items: groups[page.start..page.end]
            .iter()
            .map(|group| {
                label_group_view(
                    group,
                    Standing::of(&shown, here.as_ref(), &Subject::of_group(group)),
                )
            })
            .collect(),
        next_cursor: page.next,
    }))
}

/// Lists the labels of the organization a page at a time: those in a group the person may know,
/// and those in no group. Anybody who is signed in may.
///
/// # Errors
/// `request.invalid` for a cursor that is not one, `resource.not_found` for a project the person
/// may not see.
pub async fn labels<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: LabelsQuery,
) -> Result<Authorized<LabelPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let shown = ctx.shown_to(&actor).await?;
    let mut catalogue = ctx.label_catalogue().await?;
    let (groups, _) = ctx
        .listed_groups(&shown, &mut catalogue, input.project.as_deref())
        .await?;
    let listed: Vec<&Label> = catalogue
        .labels
        .iter()
        .filter(|label| {
            label
                .group
                .is_none_or(|group| groups.iter().any(|each| each.id == group))
        })
        .collect();
    let page = window(listed.len(), input.cursor.as_deref(), input.limit)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    Ok(held.authorize(LabelPage {
        items: listed[page.start..page.end]
            .iter()
            .map(|label| LabelView::from(*label))
            .collect(),
        next_cursor: page.next,
    }))
}

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
/// `resource.not_found` for a group that is not there, `request.invalid` for nothing asked or a
/// name that cannot be one, `name.taken`,
/// `label.pick_one_conflict` when some issue has two labels of the group, and the group is to be
/// a *Pick one* group, `edit.version_changed` when somebody changed the group since it was looked
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

/// Creates a label, at the end of the order, with the next color in turn when none is asked for.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags* where the group was created,
/// `resource.not_found` for a group that is not there, `request.invalid` for a name that cannot be
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
    let shown = ctx.shown_to(&actor).await?;
    let catalogue = ctx.label_catalogue().await?;
    let label = catalogue.label(input.label)?;
    let held = ctx
        .label_manager(&actor, &shown, &catalogue, label.group)
        .await?;
    let rewrite = match (&input.name, input.into) {
        (Some(name), None) => {
            let name = clean_tag_name(name)?;
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
            // The issues of both labels change, so the person looks after both.
            ctx.label_manager(&actor, &shown, &catalogue, into.group)
                .await?;
            Rewrite {
                sorted: Sorted::Label,
                from: label.id.into(),
                rename: None,
                into: Some(Replacement {
                    id: into.id.into(),
                    named: NameRef {
                        name: into.name.clone(),
                        group: into.group,
                    },
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
    if catalogue.label_name_taken(&name, group, Some(label.id)) {
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
