//! *Check access* below `/api/v1/check-access` (SEC-03, UX §5.2): what a person can do in a
//! project (or in the whole organization), every field with its level and how they see people,
//! each with the reasons as data. The browser turns the reasons into sentences from message keys.
//!
//! A reason never names a project the person who asks cannot see: it says *a project you can't
//! see*. The people to choose from are listed by `listPeopleToCheck`; only a person who sees names
//! there gets anybody. Both routes need *Check access* in the place.

use axum::extract::State;
use axum::response::Response;
use bb_app::access::{PeopleToCheckPage, PeopleToCheckQuery};
use bb_app::roles::{
    AccessAnswer, CheckAccessQuery, LevelAnswer, LevelPlace, PermissionAnswer, ReasonRowView,
    ReasonView, ReasonWho, RoleSays,
};
use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::UserId;
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Masked, Permission, PersonRef, Topic};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_keys::KeyProjectRef;
use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, NotFoundAnswer, NotSignedInAnswer, Session, answer,
    answer_authorized,
};
use super::field_permissions::{PlaceParams, RoleRefBody};
use crate::errors::{ApiError, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// A group named in a reason.
#[derive(Serialize, ToSchema)]
pub(crate) struct GroupRefBody {
    /// The ID of the group.
    id: String,
    /// What the group is called.
    name: String,
}

/// Whom a row of *Who has which role* was for.
#[derive(Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ReasonWhoBody {
    /// The person who is checked.
    Person,
    /// A group of theirs.
    Group {
        /// The group.
        group: GroupRefBody,
    },
}

/// One row of *Who has which role* that played a part.
#[derive(Serialize, ToSchema)]
pub(crate) struct ReasonRowBody {
    /// The ID of the row.
    assignment: String,
    /// Whom it is for.
    who: ReasonWhoBody,
    /// The project it is on, or `null` for the whole organization; *a project you can't see* when
    /// the person who asks may not know it.
    project: Option<KeyProjectRef>,
    /// Where it counts.
    applies_to: AppliesTo,
    /// Whether it comes from above the place that was checked.
    inherited: bool,
}

impl From<&ReasonRowView> for ReasonRowBody {
    fn from(row: &ReasonRowView) -> Self {
        Self {
            assignment: row.assignment.to_string(),
            who: match &row.who {
                ReasonWho::Person => ReasonWhoBody::Person,
                ReasonWho::Group { group } => ReasonWhoBody::Group {
                    group: GroupRefBody {
                        id: group.id.to_string(),
                        name: group.name.clone(),
                    },
                },
            },
            project: row.project.as_ref().map(KeyProjectRef::from),
            applies_to: row.applies_to,
            inherited: row.inherited,
        }
    }
}

/// Why a permission is given or not.
#[derive(Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ReasonBody {
    /// The role of a row gives it.
    GivenByRole {
        /// The row.
        row: ReasonRowBody,
        /// The role.
        role: RoleRefBody,
    },
    /// *Decide who has which role* is kept from a row that *No access* removed.
    KeptFromRemovedRow {
        /// The row.
        row: ReasonRowBody,
        /// Its role.
        role: RoleRefBody,
        /// The *No access* that decided.
        no_access: ReasonRowBody,
    },
    /// The role of a row has it, but *No access* removed the role.
    RemovedByNoAccess {
        /// The row.
        row: ReasonRowBody,
        /// Its role.
        role: RoleRefBody,
        /// The *No access* that decided.
        no_access: ReasonRowBody,
    },
    /// The role has it, but it counts only for the whole organization.
    CountsOnlyForTheOrganization {
        /// The row.
        row: ReasonRowBody,
        /// The role.
        role: RoleRefBody,
    },
    /// The access key does not keep it.
    NarrowedByKey,
    /// The access key does not count here.
    OutsideKeyProjects,
    /// No role gives it.
    NoRoleGivesIt,
}

impl From<&ReasonView> for ReasonBody {
    fn from(reason: &ReasonView) -> Self {
        match reason {
            ReasonView::GivenByRole { row, role } => Self::GivenByRole {
                row: row.into(),
                role: role.into(),
            },
            ReasonView::KeptFromRemovedRow {
                row,
                role,
                no_access,
            } => Self::KeptFromRemovedRow {
                row: row.into(),
                role: role.into(),
                no_access: no_access.into(),
            },
            ReasonView::RemovedByNoAccess {
                row,
                role,
                no_access,
            } => Self::RemovedByNoAccess {
                row: row.into(),
                role: role.into(),
                no_access: no_access.into(),
            },
            ReasonView::CountsOnlyForTheOrganization { row, role } => {
                Self::CountsOnlyForTheOrganization {
                    row: row.into(),
                    role: role.into(),
                }
            }
            ReasonView::NarrowedByKey => Self::NarrowedByKey,
            ReasonView::OutsideKeyProjects => Self::OutsideKeyProjects,
            ReasonView::NoRoleGivesIt => Self::NoRoleGivesIt,
        }
    }
}

/// One permission with *yes* or *no* and the reasons.
#[derive(Serialize, ToSchema)]
pub(crate) struct PermissionAnswerBody {
    /// The key of the permission; its sentence is the message `permission.<key>`.
    permission: Permission,
    /// What the permission is about; the topic is the message `permission-topic.<topic>`.
    topic: Topic,
    /// Whether the person has it here.
    allowed: bool,
    /// Why.
    reasons: Vec<ReasonBody>,
}

impl From<&PermissionAnswer> for PermissionAnswerBody {
    fn from(decision: &PermissionAnswer) -> Self {
        Self {
            permission: decision.permission,
            topic: decision.permission.topic(),
            allowed: decision.allowed,
            reasons: decision.reasons.iter().map(Into::into).collect(),
        }
    }
}

/// Where the level a role says was set.
#[derive(Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum LevelPlaceBody {
    /// For a project (the place or the nearest parent that set one).
    Project {
        /// The project, or *a project you can't see*.
        project: KeyProjectRef,
    },
    /// The role's own level, for the whole organization.
    RoleOwn,
    /// Nothing is set: closed.
    Unset,
}

impl From<&LevelPlace> for LevelPlaceBody {
    fn from(place: &LevelPlace) -> Self {
        match place {
            LevelPlace::Project { project } => Self::Project {
                project: KeyProjectRef::from(project),
            },
            LevelPlace::RoleOwn => Self::RoleOwn,
            LevelPlace::Unset => Self::Unset,
        }
    }
}

/// What one role says about a field or a kind.
#[derive(Serialize, ToSchema)]
pub(crate) struct RoleSaysBody<L> {
    /// The role.
    role: RoleRefBody,
    /// Its level.
    level: L,
    /// Where it was set.
    set: LevelPlaceBody,
}

/// A field or a kind with its level and what each role says.
#[derive(Serialize, ToSchema)]
pub(crate) struct LevelAnswerBody<K, L> {
    /// The field or the kind.
    subject: K,
    /// The level the person has.
    level: L,
    /// What each role that counts says.
    roles: Vec<RoleSaysBody<L>>,
    /// An access key lowered the level to *Can see*.
    capped_by_key: bool,
}

fn level_body<K: Copy, L: Copy>(decision: &LevelAnswer<K, L>) -> LevelAnswerBody<K, L> {
    LevelAnswerBody {
        subject: decision.subject,
        level: decision.level,
        roles: decision.roles.iter().map(says).collect(),
        capped_by_key: decision.capped_by_key,
    }
}

fn says<L: Copy>(says: &RoleSays<L>) -> RoleSaysBody<L> {
    RoleSaysBody {
        role: RoleRefBody::from(&says.role),
        level: says.level,
        set: LevelPlaceBody::from(&says.set),
    }
}

/// What a person can do in a place, and why.
#[derive(Serialize, ToSchema)]
pub(crate) struct CheckAccessBody {
    /// The person who was checked, as the person who asks may see them.
    person: PersonRef,
    /// The project that was checked, or `null` for the whole organization.
    project: Option<KeyProjectRef>,
    /// Every permission that counts here.
    permissions: Vec<PermissionAnswerBody>,
    /// Every field of the catalogue.
    fields: Vec<LevelAnswerBody<Field, FieldLevel>>,
    /// Every kind of people information.
    kinds: Vec<LevelAnswerBody<PeopleKind, PeopleLevel>>,
}

impl Masked for CheckAccessBody {}

impl From<AccessAnswer> for CheckAccessBody {
    fn from(checked: AccessAnswer) -> Self {
        Self {
            person: checked.person,
            project: checked.place.as_ref().map(KeyProjectRef::from),
            permissions: checked.permissions.iter().map(Into::into).collect(),
            fields: checked.fields.iter().map(level_body).collect(),
            kinds: checked.kinds.iter().map(level_body).collect(),
        }
    }
}

/// The path of the person to check.
#[derive(Deserialize)]
pub(crate) struct PersonPath {
    person: String,
}

/// What the person is called in a list of people to choose from.
pub(super) const PERSON_FIELDS: [FieldSpec; 3] = [
    FieldSpec::by_default("shown"),
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
];
/// The name of the list, so its cursors are its own.
const PEOPLE_LIST: &str = "check-access-people";

/// What narrows the list of people to choose from.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct PeopleSearch {
    /// Words that every found name holds, in any order; left out or empty, everybody is listed.
    pub(super) search: Option<String>,
}

/// What a person can do in a project, or in the whole organization when no project is named:
/// every permission with *yes* or *no*, every field with its level and how the person sees
/// people, each with the reasons. Needs the permission *Check what other people can do, and why*
/// in the place.
#[utoipa::path(
    get,
    path = "/api/v1/check-access/people/{person}",
    operation_id = "checkAccess",
    tag = "access",
    params(("person" = String, Path, description = "The ID of the person to check."), PlaceParams),
    responses((status = 200, description = "What the person can do, and why.", body = CheckAccessBody), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn check_access(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(who): ApiPath<PersonPath>,
    ApiQuery(place): ApiQuery<PlaceParams>,
) -> Result<Response, ApiError> {
    let person: UserId = who
        .person
        .parse()
        .map_err(|_| ApiError::new(ErrorCode::NotFound))?;
    let project = place.project()?;
    let found = auth
        .api()?
        .check_access(
            info,
            CheckAccessQuery {
                session_token,
                person,
                place: project,
            },
        )
        .await?;
    answer_authorized(&found.map(CheckAccessBody::from))
}

/// The people to choose from for *Check access*, a page at a time, by name. A person who cannot
/// see names there is listed nobody. Needs the permission *Check what other people can do, and
/// why* in the place.
#[utoipa::path(
    get,
    path = "/api/v1/check-access/people",
    operation_id = "listPeopleToCheck",
    tag = "access",
    params(ListParams, PlaceParams, PeopleSearch),
    responses((status = 200, description = "The people, by name.", body = Page<PersonRef>), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn list_people_to_check(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
    ApiQuery(place): ApiQuery<PlaceParams>,
    ApiQuery(find): ApiQuery<PeopleSearch>,
) -> Result<Response, ApiError> {
    let project = place.project()?;
    // Who asks is judged first (401 before 400): a cursor that does not open is passed on as one
    // the use case cannot read, so its answer comes after the check of the person.
    let position = params.position(PEOPLE_LIST);
    let cursor = match &position {
        Ok(position) => position.clone(),
        Err(_) => Some(String::new()),
    };
    let page = auth
        .api()?
        .people_to_check(
            info,
            PeopleToCheckQuery {
                session_token,
                place: project,
                search: find.search,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(|error| match error {
            // The only thing the use case can find invalid is the cursor.
            bb_app::auth::AppError::Refused(ErrorCode::InvalidRequest) => {
                ApiError::new(ErrorCode::BadCursor)
            }
            other => ApiError::from(other),
        })?;
    position?;
    let list = page.map(|page: PeopleToCheckPage| Listed {
        items: page.items,
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&PERSON_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(PEOPLE_LIST, &list, &fields)?))
}

/// The routes of *Check access*.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(check_access))
        .routes(routes!(list_people_to_check))
}
