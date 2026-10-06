//! People in *Organization settings* (SEC-02, PRIV-12, M2-Q3): the list, the page of one person,
//! and the choices of people that the pickers of groups and roles offer.
//!
//! The list and the page need *Manage people*, which is also what shows e-mail addresses. A
//! person's groups are shown only to somebody who also holds *Manage groups*, and a person's roles
//! only where the viewer decides who has which role: a row somebody may not decide about is left
//! out, so nothing here tells what the viewer may not know. The pickers show names only, never
//! addresses, and search by name only.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, Assignment};
use bb_core::id::{AssignmentId, GroupId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::person::Person;
use bb_policy::{
    Authorized, Evaluation, Masked, PeopleContext, Permission, PersonRef, Place, Target, resolve,
};
use jiff::Timestamp;
use serde::Serialize;

use super::GroupView;
use crate::access::{Actor, ProjectLabel};
use crate::auth::{AppError, AuthStore, Ctx, PeoplePosition, internal};
use crate::roles::RoleRef;

/// List the people, a page at a time.
pub struct PeopleQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Words that every found person holds in their name or address.
    pub search: Option<String>,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many people the page holds at most.
    pub limit: u32,
    /// Whether the groups of each person are wanted: they are read only then.
    pub groups: bool,
    /// Whether the roles of each person are wanted: they are read only then.
    pub roles: bool,
}

/// One person, with their groups and roles.
pub struct PersonQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The person.
    pub person: UserId,
}

/// One row of *Who has which role* that a person holds, as the viewer may see it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HeldRole {
    /// The ID of the row.
    pub id: AssignmentId,
    /// The role, or `None` for *No access*.
    pub role: Option<RoleRef>,
    /// The project the row is on, or `None` for the whole organization.
    pub project: Option<ProjectLabel>,
    /// Where it counts.
    pub applies_to: AppliesTo,
}

/// A person as the list shows them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PersonLine {
    /// The person, by name (people who manage people always see names).
    pub person: PersonRef,
    /// Their e-mail address.
    pub email: String,
    /// The groups they are in; `None` when the viewer may not see groups.
    pub groups: Option<Vec<GroupView>>,
    /// The rows of *Who has which role* of the person's own that the viewer may see.
    pub roles: Vec<HeldRole>,
}

impl Masked for PersonLine {}

/// A page of the people.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PersonPage {
    /// The people, by name.
    pub items: Vec<PersonLine>,
    /// Where the next page starts; `None` on the last page.
    pub next_cursor: Option<String>,
}

impl Masked for PersonPage {}

/// One person with everything the page of a person shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PersonDetail {
    /// The person, with their groups and roles.
    pub line: PersonLine,
    /// When the account was made.
    pub created_at: Timestamp,
}

impl Masked for PersonDetail {}

/// What a picker looks for: people, groups or roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceKind {
    /// A person.
    Person,
    /// A group.
    Group,
    /// A role.
    Role,
}

/// List the people, groups or roles to give a role to, or to give.
pub struct AssignmentChoicesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project the row is for, or `None` for the whole organization.
    pub project: Option<bb_core::id::ProjectId>,
    /// What to list.
    pub kind: ChoiceKind,
    /// Words that every found thing holds in its name.
    pub search: Option<String>,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many choices the page holds at most.
    pub limit: u32,
}

/// List the people that can be put into a group.
pub struct MemberChoicesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The group.
    pub group: GroupId,
    /// Words that every found person holds in their name.
    pub search: Option<String>,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many choices the page holds at most.
    pub limit: u32,
}

/// One thing to pick: a person, a group or a role, by the name the viewer may see.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChoiceView {
    /// The ID of the person, group or role.
    pub id: String,
    /// What it is called. A person is named only as the viewer may see them: somebody who is
    /// not shown by name is not offered.
    pub name: String,
    /// For the picker of a group's members: whether the person is in the group already.
    pub already_in: Option<bool>,
}

impl Masked for ChoiceView {}

/// A page of choices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChoicePage {
    /// The choices, by name.
    pub items: Vec<ChoiceView>,
    /// Where the next page starts; `None` on the last page.
    pub next_cursor: Option<String>,
}

impl Masked for ChoicePage {}

/// Where a page of people ends, as text: the ID of the last person that was shown, and nothing
/// else (a name that was left out of the page must not be in the cursor).
pub(crate) fn write_cursor(last: &Person) -> String {
    last.id.to_string()
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The position a cursor stands for: the name of the person it names, found again, so the
    /// next page starts after them. A person who is gone, or text that is no ID, is no cursor.
    pub(crate) async fn people_position(
        &self,
        cursor: Option<&str>,
    ) -> Result<Option<PeoplePosition>, AppError> {
        let Some(text) = cursor else {
            return Ok(None);
        };
        let invalid = || AppError::Refused(ErrorCode::InvalidRequest);
        let id: UserId = text.parse().map_err(|_| invalid())?;
        let person = self
            .store()
            .person(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or_else(invalid)?;
        Ok(Some(PeoplePosition {
            name: person.display_name,
            id,
        }))
    }
}

/// A person by name, to those who manage people (they always see names, M2-Q3). The place of
/// the viewer's evaluation decides *Who sees people*; a viewer who does not see names gets the
/// resolved reference, which is not a name.
fn shown(viewer: &Evaluation, person: &Person) -> PersonRef {
    resolve(
        viewer,
        PeopleKind::ProjectMembers,
        PeopleContext::Management,
        Target {
            id: person.id,
            name: &person.display_name,
            role: None,
            removed: None,
        },
    )
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The role rows of a person that the viewer may see: those at a place where the viewer
    /// decides who has which role (the whole organization, or a project).
    async fn held_roles_seen_by(
        &self,
        actor: &Actor,
        rows: &[Assignment],
    ) -> Result<Vec<HeldRole>, AppError> {
        let mut names: BTreeMap<RoleId, String> = BTreeMap::new();
        let mut held = Vec::new();
        for row in rows {
            let place = row.project.map_or(Place::Organization, Place::Project);
            let may = match self.evaluation_of(actor, place).await {
                Ok(evaluation) => evaluation.allows(Permission::DecideRoles),
                Err(_) => false,
            };
            if !may {
                continue;
            }
            let project = match row.project {
                Some(project) => Some(self.project_label(actor, project).await?),
                None => None,
            };
            let role = match row.role {
                Some(id) => {
                    let name = if let Some(known) = names.get(&id) {
                        known.clone()
                    } else {
                        let found = self
                            .store()
                            .role(id)
                            .await
                            .map_err(|error| internal::<S>(&error))?
                            .map(|record| record.role.name)
                            .unwrap_or_default();
                        names.insert(id, found.clone());
                        found
                    };
                    Some(RoleRef { id, name })
                }
                None => None,
            };
            held.push(HeldRole {
                id: row.id,
                role,
                project,
                applies_to: row.applies_to,
            });
        }
        // The whole organization first, then the projects, each group in the order the rows were
        // made.
        held.sort_by_key(|row| row.project.is_some());
        Ok(held)
    }

    /// A person as the list and the page of a person show them.
    async fn person_line(
        &self,
        actor: &Actor,
        viewer: &Evaluation,
        person: &Person,
        wanted: Wanted,
    ) -> Result<PersonLine, AppError> {
        let groups = if wanted.groups && viewer.allows(Permission::ManageGroups) {
            Some(
                self.store()
                    .groups_of(person.id)
                    .await
                    .map_err(|error| internal::<S>(&error))?
                    .iter()
                    .map(GroupView::from)
                    .collect(),
            )
        } else {
            None
        };
        let roles = if wanted.roles {
            let rows = self
                .store()
                .assignments_of_person(person.id)
                .await
                .map_err(|error| internal::<S>(&error))?;
            self.held_roles_seen_by(actor, &rows).await?
        } else {
            Vec::new()
        };
        Ok(PersonLine {
            person: shown(viewer, person),
            email: person.email.clone(),
            groups,
            roles,
        })
    }
}

/// The people, by name, a page at a time, with their e-mail addresses.
///
/// # Errors
/// `access.forbidden` without *Manage people*, `request.invalid` for a cursor that is not one.
pub async fn people<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PeopleQuery,
) -> Result<Authorized<PersonPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManagePeople, Place::Organization)
        .await?;
    let limit = u64::from(bb_core::paging::page_size(Some(input.limit)));
    let after = ctx.people_position(input.cursor.as_deref()).await?;
    // One more than the page, to know whether there is a next one.
    let mut found = ctx
        .store()
        .people_page(input.search.as_deref(), true, after.as_ref(), limit + 1)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let more = u64::try_from(found.len()).is_ok_and(|count| count > limit);
    found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    let next_cursor = more.then(|| found.last().map(write_cursor)).flatten();
    let mut items = Vec::with_capacity(found.len());
    for person in &found {
        let wanted = Wanted {
            groups: input.groups,
            roles: input.roles,
        };
        items.push(ctx.person_line(&actor, &access, person, wanted).await?);
    }
    Ok(access.authorize(PersonPage { items, next_cursor }))
}

/// One person with their groups and roles.
///
/// # Errors
/// `access.forbidden` without *Manage people*, `resource.not_found` for an unknown person.
pub async fn person<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PersonQuery,
) -> Result<Authorized<PersonDetail>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManagePeople, Place::Organization)
        .await?;
    let found = ctx
        .store()
        .person(input.person)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    Ok(access.authorize(PersonDetail {
        line: ctx
            .person_line(
                &actor,
                &access,
                &found,
                Wanted {
                    groups: true,
                    roles: true,
                },
            )
            .await?,
        created_at: found.created_at,
    }))
}

/// A choice of a person, when the viewer sees them by name; `None` for anybody they may not name.
fn person_choice(
    viewer: &Evaluation,
    person: &Person,
    already_in: Option<bool>,
) -> Option<ChoiceView> {
    match shown(viewer, person) {
        PersonRef::Name { id, name } => Some(ChoiceView {
            id: id.to_string(),
            name,
            already_in,
        }),
        PersonRef::Role { .. } | PersonRef::Someone | PersonRef::Former { .. } => None,
    }
}

/// The people (by name only) that can be put into a group; those who are in it already say so.
///
/// # Errors
/// `access.forbidden` without *Manage groups*, `resource.not_found` for an unknown group,
/// `request.invalid` for a cursor that is not one.
pub async fn member_choices<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MemberChoicesQuery,
) -> Result<Authorized<ChoicePage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManageGroups, Place::Organization)
        .await?;
    let group = ctx.group_or_missing(input.group).await?;
    let members: Vec<UserId> = ctx
        .store()
        .group_members(group.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .iter()
        .map(|member| member.person)
        .collect();
    let page = ctx
        .people_choices(
            &access,
            input.search.as_deref(),
            input.cursor.as_deref(),
            input.limit,
            |person| Some(members.contains(&person.id)),
        )
        .await?;
    Ok(access.authorize(page))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// A page of people to choose from, by name, as the viewer may see them.
    pub(crate) async fn people_choices(
        &self,
        viewer: &Evaluation,
        search: Option<&str>,
        cursor: Option<&str>,
        limit: u32,
        already_in: impl Fn(&Person) -> Option<bool> + Send,
    ) -> Result<ChoicePage, AppError> {
        let limit = u64::from(bb_core::paging::page_size(Some(limit)));
        let after = self.people_position(cursor).await?;
        let mut found = self
            .store()
            .people_page(search, false, after.as_ref(), limit + 1)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let more = u64::try_from(found.len()).is_ok_and(|count| count > limit);
        found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
        let mut shown = Vec::new();
        let mut items = Vec::new();
        for person in &found {
            if let Some(choice) = person_choice(viewer, person, already_in(person)) {
                shown.push(person);
                items.push(choice);
            }
        }
        // The cursor names the last person who is in the page, not the last one that was looked at.
        let next_cursor = more
            .then(|| shown.last().map(|last| write_cursor(last)))
            .flatten();
        Ok(ChoicePage { items, next_cursor })
    }
}

/// What of a person a request wants besides their name and address.
#[derive(Clone, Copy)]
pub(crate) struct Wanted {
    pub(crate) groups: bool,
    pub(crate) roles: bool,
}
