//! The group routes below `/api/v1/groups` (SEC-02): the list, making, renaming and deleting a
//! group, who is in it, and the choices of people to put into it. All of them need the permission
//! *Create, rename and delete groups and change who is in them*; what the use cases refuse beyond
//! that (the last administrator, no escalation, a *No access* that a change would lift) comes back
//! as the refusal of the use case.

use axum::extract::State;
use axum::response::Response;
use bb_app::people::{
    ChoicePage, ChoiceView, CreateGroup, DeleteGroup, GroupDetail, GroupMembers, GroupMembersQuery,
    GroupQuery, GroupView, GroupsQuery, MemberChoicesQuery, MemberView, RemoveGroupMember,
    RenameGroup,
};
use bb_core::id::{GroupId, UserId};
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::cursor_error;
use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer, answer_authorized, done,
};
use super::path_id;
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The fields of a group in a list that can be asked for.
const GROUP_FIELDS: [FieldSpec; 3] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("version"),
];
/// The name of the list, so its cursors are its own.
const GROUPS_LIST: &str = "groups";

/// A group in a list: the fields that can be asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct GroupItem {
    /// The ID of the group.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    /// What it is called.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Counts up with every change. A change is made on the version that was looked at.
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<i64>,
}

impl Masked for GroupItem {}

impl From<&GroupView> for GroupItem {
    fn from(group: &GroupView) -> Self {
        Self {
            id: Some(group.id.to_string()),
            name: Some(group.name.clone()),
            version: Some(group.version),
        }
    }
}

/// A group after it was made or renamed.
#[derive(Serialize, ToSchema)]
pub(crate) struct GroupBody {
    /// The ID of the group.
    id: String,
    /// What it is called.
    name: String,
    /// Counts up with every change. A change is made on the version that was looked at.
    version: i64,
}

impl Masked for GroupBody {}

impl From<&GroupView> for GroupBody {
    fn from(group: &GroupView) -> Self {
        Self {
            id: group.id.to_string(),
            name: group.name.clone(),
            version: group.version,
        }
    }
}

/// What a group is used for, as far as the person who asks may see.
#[derive(Serialize, ToSchema)]
pub(crate) struct GroupUsedByBody {
    /// Whether the group has a role for the whole organization.
    whole_organization: bool,
    /// In how many projects the person who asks can see that the group has a role or *No
    /// access*. Projects they cannot see are left out, so this number never tells of them.
    projects: u64,
    /// How many people are in the group. Whoever may ask sees all of them.
    members: u64,
}

/// One group with what it is used for.
#[derive(Serialize, ToSchema)]
pub(crate) struct GroupDetailBody {
    /// The group.
    group: GroupBody,
    /// What it is used for.
    used_by: GroupUsedByBody,
}

impl Masked for GroupDetailBody {}

impl From<&GroupDetail> for GroupDetailBody {
    fn from(detail: &GroupDetail) -> Self {
        Self {
            group: GroupBody::from(&detail.group),
            used_by: GroupUsedByBody {
                whole_organization: detail.used_by.whole_organization,
                projects: detail.used_by.projects,
                members: detail.used_by.members,
            },
        }
    }
}

/// A name for a group.
#[derive(Deserialize, ToSchema)]
pub(crate) struct GroupNameBody {
    /// What the group is called.
    name: String,
}

/// A new name for a group, made on the version that was looked at.
#[derive(Deserialize, ToSchema)]
pub(crate) struct RenameGroupBody {
    /// What the group is called from now on.
    name: String,
    /// The version of the group the person looked at; a change made meanwhile stops this one.
    version: i64,
}

const MEMBER_FIELDS: [FieldSpec; 2] = [
    FieldSpec::by_default("person"),
    FieldSpec::by_default("added_at"),
];
const MEMBERS_LIST: &str = "group-members";

/// A person in a group: the fields that can be asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct MemberItem {
    /// The person, by name (whoever manages groups sees names).
    #[serde(skip_serializing_if = "Option::is_none")]
    person: Option<PersonRef>,
    /// When the person was put into the group.
    #[serde(skip_serializing_if = "Option::is_none")]
    added_at: Option<String>,
}

impl Masked for MemberItem {}

impl From<&MemberView> for MemberItem {
    fn from(member: &MemberView) -> Self {
        Self {
            person: Some(member.person.clone()),
            added_at: Some(member.added_at.to_string()),
        }
    }
}

const CHOICE_FIELDS: [FieldSpec; 3] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
    FieldSpec::on_request("already_in"),
];
const CHOICES_LIST: &str = "member-choices";

/// A person that can be put into a group: the fields that can be asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct ChoiceItem {
    /// The ID of the person, group or role.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<String>,
    /// What it is called.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    /// For the choices of a group's members: whether the person is in the group already.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) already_in: Option<bool>,
}

impl Masked for ChoiceItem {}

impl From<&ChoiceView> for ChoiceItem {
    fn from(choice: &ChoiceView) -> Self {
        Self {
            id: Some(choice.id.clone()),
            name: Some(choice.name.clone()),
            already_in: choice.already_in,
        }
    }
}

/// What narrows a list of choices.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ChoiceFilter {
    /// Words that every found name holds, without regard to capitals.
    search: Option<String>,
}

/// The fields of a choice, for the other routes that offer choices.
pub(crate) const CHOICE_FIELD_TABLE: &[FieldSpec] = &CHOICE_FIELDS;

/// A page of choices as the answer of a route.
pub(crate) fn choice_page(
    list: &str,
    params: &ListParams,
    page: bb_policy::Authorized<ChoicePage>,
) -> Result<Response, ApiError> {
    let list_page = page.map(|page| Listed {
        items: page.items.iter().map(ChoiceItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(CHOICE_FIELD_TABLE, &list_page.hidden_properties())?;
    Ok(answer(Page::of_authorized(list, &list_page, &fields)?))
}

/// The groups, by name, a page at a time. Needs the permission *Create, rename and delete groups
/// and change who is in them*.
#[utoipa::path(
    get,
    path = "/api/v1/groups",
    operation_id = "listGroups",
    tag = "groups",
    params(ListParams),
    responses((status = 200, description = "The groups, by name.", body = Page<GroupItem>), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_groups(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(GROUPS_LIST);
    let found = auth
        .api()?
        .groups(
            info,
            GroupsQuery {
                session_token,
                place: None,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    let listed = found.map(|list| Listed {
        items: list.groups.iter().map(GroupItem::from).collect(),
        next: list.next_cursor,
    });
    let fields = params.fields_readable(&GROUP_FIELDS, &listed.hidden_properties())?;
    Ok(answer(Page::of_authorized(GROUPS_LIST, &listed, &fields)?))
}

/// Creates a group. Needs the permission *Create, rename and delete groups and change who is in them*.
#[utoipa::path(
    post,
    path = "/api/v1/groups",
    operation_id = "createGroup",
    tag = "groups",
    request_body = GroupNameBody,
    responses(
        (status = 200, description = "The group.", body = GroupBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        ConflictAnswer
    )
)]
async fn create_group(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<GroupNameBody>,
) -> Result<Response, ApiError> {
    let made = auth
        .api()?
        .create_group(
            info,
            CreateGroup {
                session_token,
                name: body.name,
            },
        )
        .await?;
    answer_authorized(&made.map(|group| GroupBody::from(&group)))
}

/// One group, with what it is used for. Needs the permission *Create, rename and delete groups and
/// change who is in them*.
#[utoipa::path(
    get,
    path = "/api/v1/groups/{group}",
    operation_id = "getGroup",
    tag = "groups",
    params(("group" = String, Path, description = "The ID of the group.")),
    responses((status = 200, description = "The group.", body = GroupDetailBody), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn get_group(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(group_id): ApiPath<String>,
) -> Result<Response, ApiError> {
    let group = path_id::<GroupId>(&group_id)?;
    let found = auth
        .api()?
        .group(
            info,
            GroupQuery {
                session_token,
                group,
            },
        )
        .await?;
    answer_authorized(&found.map(|detail| GroupDetailBody::from(&detail)))
}

/// Renames a group. Needs the permission *Create, rename and delete groups and change who is in
/// them*.
#[utoipa::path(
    patch,
    path = "/api/v1/groups/{group}",
    operation_id = "renameGroup",
    tag = "groups",
    params(("group" = String, Path, description = "The ID of the group.")),
    request_body = RenameGroupBody,
    responses(
        (status = 200, description = "The group as it is now.", body = GroupBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn rename_group(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(group_id): ApiPath<String>,
    ApiJson(body): ApiJson<RenameGroupBody>,
) -> Result<Response, ApiError> {
    let group = path_id::<GroupId>(&group_id)?;
    let renamed = auth
        .api()?
        .rename_group(
            info,
            RenameGroup {
                session_token,
                group,
                version: body.version,
                name: body.name,
            },
        )
        .await?;
    answer_authorized(&renamed.map(|group| GroupBody::from(&group)))
}

/// Deletes a group, with its members and the rows that give it a role. Needs the permission
/// *Create, rename and delete groups and change who is in them*, and, where the group has *No
/// access*, *Decide who has which role* there.
#[utoipa::path(
    delete,
    path = "/api/v1/groups/{group}",
    operation_id = "deleteGroup",
    tag = "groups",
    params(("group" = String, Path, description = "The ID of the group.")),
    responses(
        (status = 204, description = "The group is gone."),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn delete_group(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(group_id): ApiPath<String>,
) -> Result<Response, ApiError> {
    let group = path_id::<GroupId>(&group_id)?;
    auth.api()?
        .delete_group(
            info,
            DeleteGroup {
                session_token,
                group,
            },
        )
        .await?;
    Ok(done())
}

/// The people in a group, a page at a time. Needs the permission *Create, rename and delete groups
/// and change who is in them*.
#[utoipa::path(
    get,
    path = "/api/v1/groups/{group}/members",
    operation_id = "listGroupMembers",
    tag = "groups",
    params(("group" = String, Path, description = "The ID of the group."), ListParams),
    responses((status = 200, description = "The people in the group.", body = Page<MemberItem>), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn list_group_members(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(group_id): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let group = path_id::<GroupId>(&group_id)?;
    let (cursor, bad_cursor) = params.start(MEMBERS_LIST);
    let found = auth
        .api()?
        .group_members(
            info,
            GroupMembersQuery {
                session_token,
                group,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    let listed = found.map(|members: GroupMembers| Listed {
        items: members.members.iter().map(MemberItem::from).collect(),
        next: members.next_cursor,
    });
    let fields = params.fields_readable(&MEMBER_FIELDS, &listed.hidden_properties())?;
    Ok(answer(Page::of_authorized(MEMBERS_LIST, &listed, &fields)?))
}

/// Puts a person into a group. Needs the permission *Create, rename and delete groups and change
/// who is in them*; nobody puts somebody into a group that has a role with more than they hold
/// themselves.
#[utoipa::path(
    put,
    path = "/api/v1/groups/{group}/members/{person}",
    operation_id = "addGroupMember",
    tag = "groups",
    params(
        ("group" = String, Path, description = "The ID of the group."),
        ("person" = String, Path, description = "The ID of the person.")
    ),
    responses(
        (status = 204, description = "The person is in the group."),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn add_group_member(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((group_id, person_id)): ApiPath<(String, String)>,
) -> Result<Response, ApiError> {
    let group = path_id::<GroupId>(&group_id)?;
    let person = path_id::<UserId>(&person_id)?;
    auth.api()?
        .add_group_member(
            info,
            bb_app::people::AddGroupMember {
                session_token,
                group,
                person,
            },
        )
        .await?;
    Ok(done())
}

/// Takes a person out of a group. Needs the permission *Create, rename and delete groups and change
/// who is in them*, and, where the group has *No access*, *Decide who has which role* there.
#[utoipa::path(
    delete,
    path = "/api/v1/groups/{group}/members/{person}",
    operation_id = "removeGroupMember",
    tag = "groups",
    params(
        ("group" = String, Path, description = "The ID of the group."),
        ("person" = String, Path, description = "The ID of the person.")
    ),
    responses(
        (status = 204, description = "The person is not in the group."),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn remove_group_member(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((group_id, person_id)): ApiPath<(String, String)>,
) -> Result<Response, ApiError> {
    let group = path_id::<GroupId>(&group_id)?;
    let person = path_id::<UserId>(&person_id)?;
    auth.api()?
        .remove_group_member(
            info,
            RemoveGroupMember {
                session_token,
                group,
                person,
            },
        )
        .await?;
    Ok(done())
}

/// The people that can be put into a group, by name, a page at a time; those who are in it
/// already say so. Needs the permission *Create, rename and delete groups and change who is in
/// them*. Names only, never e-mail addresses.
#[utoipa::path(
    get,
    path = "/api/v1/groups/{group}/member-choices",
    operation_id = "listMemberChoices",
    tag = "groups",
    params(("group" = String, Path, description = "The ID of the group."), ListParams, ChoiceFilter),
    responses((status = 200, description = "The people to choose from.", body = Page<ChoiceItem>), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn list_member_choices(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(group_id): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
    ApiQuery(filter): ApiQuery<ChoiceFilter>,
) -> Result<Response, ApiError> {
    let group = path_id::<GroupId>(&group_id)?;
    let (cursor, bad_cursor) = params.start(CHOICES_LIST);
    let found = auth
        .api()?
        .member_choices(
            info,
            MemberChoicesQuery {
                session_token,
                group,
                search: filter.search,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    choice_page(CHOICES_LIST, &params, found)
}

/// The routes of the groups.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_groups, create_group))
        .routes(routes!(get_group, rename_group, delete_group))
        .routes(routes!(list_group_members))
        .routes(routes!(add_group_member, remove_group_member))
        .routes(routes!(list_member_choices))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_field_tables_name_exactly_the_fields_of_their_items() {
        crate::lean::assert_fields_are_the_keys_of(
            &GROUP_FIELDS,
            &GroupItem {
                id: Some("i".to_owned()),
                name: Some("n".to_owned()),
                version: Some(1),
            },
        );
        crate::lean::assert_fields_are_the_keys_of(
            &MEMBER_FIELDS,
            &MemberItem {
                person: Some(PersonRef::Someone),
                added_at: Some("t".to_owned()),
            },
        );
        crate::lean::assert_fields_are_the_keys_of(
            &CHOICE_FIELDS,
            &ChoiceItem {
                id: Some("i".to_owned()),
                name: Some("n".to_owned()),
                already_in: Some(false),
            },
        );
    }
}
