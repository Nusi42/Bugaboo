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
//!
//! This file reads the list and holds what the use cases share; making and changing a group is
//! `labels/groups.rs`, a label `labels/change.rs`, combining two `labels/combine.rs`.

mod change;
mod combine;
mod groups;

pub use change::{change_label, create_label, label_change_preview};
pub use combine::combine_labels;
pub use groups::{change_label_group, create_label_group};

use std::collections::HashMap;
use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::id::{IssueId, LabelGroupId, LabelId, ProjectId};
use bb_core::names::{Placed, name_among, taken_where_they_meet};
use bb_core::offer::has_room;
use bb_core::paging::window;
use bb_core::tags::{LABELS_MAX, Label, LabelGroup};
use bb_policy::{Authorized, Evaluation, Permission, Place, Tree};
use jiff::Timestamp;

use super::rewrite::ISSUES_PER_READ;
use super::types::{
    LabelGroupPage, LabelGroupView, LabelGroupsQuery, LabelPage, LabelView, LabelsQuery,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::offers::label_is_known;
use crate::offers::{Here, Standing, Subject};
use crate::projects::{Shown, Why};

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
        name_among(
            self.labels
                .iter()
                .filter(|other| Some(other.id) != except && other.group == group)
                .map(|other| other.name.as_str()),
            name,
        )
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
    taken_where_they_meet(
        groups,
        name,
        owner,
        |inside, outside| tree.is_within(inside, outside),
        |other| Placed {
            name: &other.name,
            project: other.project,
            is_the_one_changed: Some(other.id) == except,
        },
    )
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

    /// Refuses (`resource.not_found`) a label that is not there or that is in a label group
    /// the person may not know (`offers/known.rs`).
    pub(crate) async fn label_known(&self, actor: &Actor, label: LabelId) -> Result<(), AppError> {
        let catalogue = self.label_catalogue().await?;
        self.ensure_label_known(actor, catalogue.label(label)?)
            .await
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
        .filter(|label| label_is_known(label.group, &groups))
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
