//! Groups (SEC-02): making, renaming and deleting groups, and who is in them. People and their
//! accounts are in `auth`; roles and who has which role are in `roles`.
//!
//! Every change asks the engine first (*Manage groups* for the whole organization), refuses what
//! would leave nobody who can manage roles, writes its domain event and a security-log entry, and
//! returns data only as `Authorized<T>`. Putting a person into a group gives them the group's
//! roles, so it is held to the same rule as giving a role: nobody puts somebody into a group that
//! holds more than they hold themselves (ADR-0025, decision 6).

mod group_page;
mod list;
mod used_by;

pub use group_page::{GroupQuery, group};
pub(crate) use list::write_cursor;
pub use list::{
    AssignmentChoicesQuery, ChoiceKind, ChoicePage, ChoiceView, HeldRole, MemberChoicesQuery,
    PeopleQuery, PersonDetail, PersonLine, PersonPage, PersonQuery, member_choices, people, person,
};
pub use used_by::{GroupDetail, GroupUsedBy, RoleUsedBy};

use bb_core::ErrorCode;
use bb_core::access::{Group, GroupMember};
use bb_core::id::{GroupId, ProjectId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::sign_in::SecurityAction;
use bb_policy::guards::ensure_a_manager_remains;
use bb_policy::{Authorized, Masked, PeopleContext, Permission, PersonRef, Place};
use jiff::Timestamp;
use serde::Serialize;
use serde_json::json;

use crate::access::{ensure_may_give_in, ensure_may_lift_in};
use crate::auth::{
    AppError, AuthStore, Ctx, Done, by, change_failed, changed, entity, internal, set_to,
};
use crate::input::clean_name;

/// Make a group.
pub struct MakeGroup {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What the group is called.
    pub name: String,
}

/// Rename a group.
pub struct RenameGroup {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The group.
    pub group: GroupId,
    /// The new name.
    pub name: String,
}

/// Delete a group, with its members and the rows that give it a role.
pub struct DeleteGroup {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The group.
    pub group: GroupId,
}

/// Put a person into a group.
pub struct AddGroupMember {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The group.
    pub group: GroupId,
    /// The person.
    pub person: UserId,
}

/// Take a person out of a group.
pub struct RemoveGroupMember {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The group.
    pub group: GroupId,
    /// The person.
    pub person: UserId,
}

/// List the groups, in *Organization settings* or for choosing one on a project.
pub struct GroupsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project the person chooses a group for, or `None` for *Organization settings*.
    pub place: Option<ProjectId>,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many groups the page holds at most.
    pub limit: u32,
}

/// List the people in a group.
pub struct GroupMembersQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The group.
    pub group: GroupId,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many people the page holds at most.
    pub limit: u32,
}

/// A group as it is shown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GroupView {
    /// The ID of the group.
    pub id: GroupId,
    /// What it is called.
    pub name: String,
}

impl Masked for GroupView {}

impl From<&Group> for GroupView {
    fn from(group: &Group) -> Self {
        Self {
            id: group.id,
            name: group.name.clone(),
        }
    }
}

/// The groups, by name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GroupList {
    /// The groups on this page.
    pub groups: Vec<GroupView>,
    /// Where the next page starts; `None` on the last page.
    pub next_cursor: Option<String>,
}

impl Masked for GroupList {}

/// One person in a group, shown as the viewer may see them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MemberView {
    /// The person.
    pub person: PersonRef,
    /// When they were put into the group.
    pub added_at: Timestamp,
}

/// A group with the people in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GroupMembers {
    /// The group.
    pub group: GroupView,
    /// The people on this page, in the order they were put there.
    pub members: Vec<MemberView>,
    /// Where the next page starts; `None` on the last page.
    pub next_cursor: Option<String>,
}

impl Masked for GroupMembers {}

impl<S: AuthStore> Ctx<'_, S> {
    /// The group, or `resource.not_found`.
    async fn group_or_missing(&self, id: GroupId) -> Result<Group, AppError> {
        self.store()
            .group(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }
}

/// Makes a group.
///
/// # Errors
/// `access.forbidden` without *Manage groups*, `request.invalid` for a name that cannot be one,
/// `name.taken` when another group has the name.
pub async fn make_group<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MakeGroup,
) -> Result<Authorized<GroupView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManageGroups, Place::Organization)
        .await?;
    let name = clean_name(&input.name)?;
    let group = Group {
        id: GroupId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        name,
    };
    let event = ctx.event(
        by(actor.person.id),
        entity("group", group.id)?,
        vec![set_to("name", json!(group.name))],
    )?;
    let stored = group.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::insert_group(transaction, &stored).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Some(ErrorCode::NameTaken)))?;
    ctx.note(
        SecurityAction::GroupMade,
        Some(actor.person.id),
        Some(entity("group", group.id)?),
    )
    .await;
    Ok(access.authorize(GroupView::from(&group)))
}

/// Renames a group.
///
/// # Errors
/// `access.forbidden` without *Manage groups*, `resource.not_found` for an unknown group,
/// `request.invalid` for a name that cannot be one, `name.taken` when another group has it.
pub async fn rename_group<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RenameGroup,
) -> Result<Authorized<GroupView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManageGroups, Place::Organization)
        .await?;
    let name = clean_name(&input.name)?;
    let before = ctx.group_or_missing(input.group).await?;
    let id = before.id;
    let event = ctx.event(
        by(actor.person.id),
        entity("group", id)?,
        vec![bb_core::event::FieldChange {
            field: "name".to_owned(),
            before: Some(json!(before.name)),
            after: Some(json!(name)),
        }],
    )?;
    let new_name = name.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::rename_group(transaction, id, &new_name).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| stopped::<S>(&error, ErrorCode::NotFound, ErrorCode::NameTaken))?;
    ctx.note(
        SecurityAction::GroupRenamed,
        Some(actor.person.id),
        Some(entity("group", id)?),
    )
    .await;
    Ok(access.authorize(GroupView { id, name }))
}

/// Deletes a group, with its members and the rows that give it a role.
///
/// # Errors
/// `access.forbidden` without *Manage groups*, `resource.not_found` for an unknown group,
/// `access.would_lock_out` when it would leave nobody who can manage roles.
pub async fn delete_group<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteGroup,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.require(&actor, Permission::ManageGroups, Place::Organization)
        .await?;
    let id = ctx.group_or_missing(input.group).await?.id;
    let event = ctx.event(
        by(actor.person.id),
        entity("group", id)?,
        vec![changed("group")],
    )?;
    let by_person = actor.person.id;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                // Deleting the group lifts the *No access* it has, for its people.
                ensure_may_lift_in::<S>(transaction, by_person, id).await?;
                let before = S::organization_state_in(transaction).await?;
                if !S::delete_group(transaction, id).await? {
                    return Err(S::aborted());
                }
                let after = S::organization_state_in(transaction).await?;
                ensure_a_manager_remains(&before, &after).map_err(S::refused)
            })
        })
        .await
        .map_err(|error| stopped::<S>(&error, ErrorCode::NotFound, ErrorCode::NotFound))?;
    ctx.note(
        SecurityAction::GroupDeleted,
        Some(actor.person.id),
        Some(entity("group", id)?),
    )
    .await;
    Ok(Done)
}

/// What a change that stopped itself says: the rule of the policy engine that refused it, that the
/// record it meant is gone, or that its name is taken.
pub(crate) fn stopped<S: AuthStore>(
    error: &S::Error,
    aborted: ErrorCode,
    duplicate: ErrorCode,
) -> AppError {
    if let Some(refusal) = S::as_refusal(error) {
        return refusal.into();
    }
    if S::is_aborted(error) {
        return AppError::Refused(aborted);
    }
    if S::is_duplicate(error) {
        return AppError::Refused(duplicate);
    }
    internal::<S>(error)
}

/// Puts a person into a group.
///
/// # Errors
/// `access.forbidden` without *Manage groups*, `resource.not_found` for an unknown group or
/// person, `access.too_much` when the group holds a role with more than the person who asks holds
/// where the role is given.
pub async fn add_group_member<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AddGroupMember,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.require(&actor, Permission::ManageGroups, Place::Organization)
        .await?;
    let group = ctx.group_or_missing(input.group).await?;
    if ctx
        .store()
        .person(input.person)
        .await
        .map_err(|error| internal::<S>(&error))?
        .is_none()
    {
        return Err(ErrorCode::NotFound.into());
    }
    let member = GroupMember {
        group: group.id,
        person: input.person,
        added_at: ctx.now(),
    };
    let event = ctx.event(
        by(actor.person.id),
        entity("group", group.id)?,
        vec![changed("member")],
    )?;
    let giver = actor.person.id;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                // The person gets every role of the group: the person who asks must hold all of
                // it, at every place the group has it, as it is stored now.
                for row in S::assignments_of_group_in(transaction, member.group).await? {
                    ensure_may_give_in::<S>(transaction, giver, &row).await?;
                }
                S::insert_group_member(transaction, &member).await
            })
        })
        .await
        .map_err(|error| stopped::<S>(&error, ErrorCode::NotFound, ErrorCode::InvalidRequest))?;
    ctx.note(
        SecurityAction::GroupMemberAdded,
        Some(actor.person.id),
        Some(entity("group", group.id)?),
    )
    .await;
    Ok(Done)
}

/// Takes a person out of a group.
///
/// # Errors
/// `access.forbidden` without *Manage groups*, `resource.not_found` for an unknown group or a
/// person who is not in it, `access.would_lock_out` when it would leave nobody who can manage
/// roles.
pub async fn remove_group_member<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RemoveGroupMember,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.require(&actor, Permission::ManageGroups, Place::Organization)
        .await?;
    let group = ctx.group_or_missing(input.group).await?.id;
    let person = input.person;
    let event = ctx.event(
        by(actor.person.id),
        entity("group", group)?,
        vec![changed("member")],
    )?;
    let by_person = actor.person.id;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                // Taking the person out lifts the *No access* the group has, for them.
                ensure_may_lift_in::<S>(transaction, by_person, group).await?;
                let before = S::organization_state_in(transaction).await?;
                if !S::delete_group_member(transaction, group, person).await? {
                    return Err(S::aborted());
                }
                let after = S::organization_state_in(transaction).await?;
                ensure_a_manager_remains(&before, &after).map_err(S::refused)
            })
        })
        .await
        .map_err(|error| stopped::<S>(&error, ErrorCode::NotFound, ErrorCode::NotFound))?;
    ctx.note(
        SecurityAction::GroupMemberRemoved,
        Some(actor.person.id),
        Some(entity("group", group)?),
    )
    .await;
    Ok(Done)
}

/// The groups, by name, a page at a time.
///
/// In *Organization settings* the list takes *Manage groups*; on a project it takes *Decide who has
/// which role* there, because that is where a person chooses a group to give a role to.
///
/// # Errors
/// `access.forbidden` without the permission, `resource.not_found` for a project the person may
/// not know about, `request.invalid` for a cursor that is not one.
pub async fn groups<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: GroupsQuery,
) -> Result<Authorized<GroupList>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = match input.place {
        None => {
            ctx.require(&actor, Permission::ManageGroups, Place::Organization)
                .await?
        }
        Some(project) => {
            ctx.require(&actor, Permission::DecideRoles, Place::Project(project))
                .await?
        }
    };
    let groups = ctx
        .store()
        .groups()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let page = bb_core::paging::window(groups.len(), input.cursor.as_deref(), input.limit)
        .ok_or(ErrorCode::InvalidRequest)?;
    Ok(access.authorize(GroupList {
        groups: groups[page.start..page.end]
            .iter()
            .map(GroupView::from)
            .collect(),
        next_cursor: page.next,
    }))
}

/// The people in a group, a page at a time, each shown as the viewer may see them: people who
/// manage groups see names (M2-Q3).
///
/// # Errors
/// `access.forbidden` without *Manage groups*, `resource.not_found` for an unknown group,
/// `request.invalid` for a cursor that is not one.
pub async fn group_members<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: GroupMembersQuery,
) -> Result<Authorized<GroupMembers>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManageGroups, Place::Organization)
        .await?;
    let group = ctx.group_or_missing(input.group).await?;
    let rows = ctx
        .store()
        .group_members(group.id)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let page = bb_core::paging::window(rows.len(), input.cursor.as_deref(), input.limit)
        .ok_or(ErrorCode::InvalidRequest)?;
    let mut members = Vec::with_capacity(page.end - page.start);
    for row in &rows[page.start..page.end] {
        members.push(MemberView {
            person: ctx
                .person_ref(
                    &access,
                    PeopleKind::ProjectMembers,
                    PeopleContext::Management,
                    row.person,
                )
                .await?,
            added_at: row.added_at,
        });
    }
    Ok(access.authorize(GroupMembers {
        group: GroupView::from(&group),
        members,
        next_cursor: page.next,
    }))
}
