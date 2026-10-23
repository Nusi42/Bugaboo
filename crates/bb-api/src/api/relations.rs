//! The routes of sub-issues and related issues (ISS-03, ISS-04): the kinds of relation below
//! `/api/v1/relation-kinds`, and what an issue stands among below `/api/v1/issues/{issue}`.
//!
//! An issue appears in an answer only to a person who may see it, as an [`IssueRef`] that holds the
//! fields the person may see in the project of that issue. No answer holds a count of what the
//! person may not see (PRIV-13): the progress counts the sub-issues the person sees.

use axum::extract::State;
use axum::response::Response;
use bb_app::relations::{
    AddRelation, ChangeRelationKind, CreateRelationKind, IssueLine, IssueRelations,
    IssueRelationsQuery, RelatedIssuesQuery, RelationKindView, RelationKindsQuery, RelationView,
    RemoveRelation, SetParent, SubIssuesQuery,
};
use bb_core::id::{RelationId, RelationKindId};
use bb_core::issue::Priority;
use bb_core::relation::Direction;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::cursor_error;
use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer, answer_authorized, done,
};
use super::issues::IssueTypeRef;
use super::projects::ProjectRef;
use super::workflows::StatusRef;
use super::{body_id, path_id};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The fields of an issue in a list of sub-issues, and which of them a list holds when the request
/// names none.
const ISSUE_REF_FIELDS: [FieldSpec; 7] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("readable_id"),
    FieldSpec::on_request("project"),
    FieldSpec::by_default("issue_type"),
    FieldSpec::by_default("title"),
    FieldSpec::by_default("status"),
    FieldSpec::by_default("priority"),
];
const SUB_ISSUES_LIST: &str = "sub-issues";

/// The fields of a related issue in a list.
const RELATED_FIELDS: [FieldSpec; 5] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("relation_kind"),
    FieldSpec::by_default("direction"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("issue"),
];
const RELATED_LIST: &str = "related-issues";

/// The fields of a kind of relation in a list: all of them are by default, a kind is small.
const KIND_FIELDS: [FieldSpec; 5] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("forward"),
    FieldSpec::by_default("backward"),
    FieldSpec::by_default("retired"),
    FieldSpec::by_default("version"),
];
const KINDS_LIST: &str = "relation-kinds";

/// An issue as a small reference: its readable ID and what the viewer may see of it in its own
/// project. A title or a status that the viewer's role hides there is left out.
#[derive(Serialize, ToSchema)]
pub(crate) struct IssueRef {
    /// The internal ID of the issue.
    pub(crate) id: String,
    /// The readable ID now (`WEB-142`).
    pub(crate) readable_id: String,
    /// The project it is in.
    pub(crate) project: ProjectRef,
    /// Its issue type.
    pub(crate) issue_type: IssueTypeRef,
    /// What it is about.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<String>,
    /// Where it stands in its workflow: the key, the name and the status group.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) status: Option<StatusRef>,
    /// How important it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) priority: Option<Priority>,
}

// The members were dropped for the viewer in the project of the issue, which can differ from the
// place of the answer.
impl Masked for IssueRef {}

impl From<&IssueLine> for IssueRef {
    fn from(line: &IssueLine) -> Self {
        Self {
            id: line.id.to_string(),
            readable_id: line.key.clone(),
            project: ProjectRef::from(&line.project),
            issue_type: IssueTypeRef::from(&line.issue_type),
            title: line.title.clone(),
            status: line.status.as_ref().map(StatusRef::from),
            priority: line.priority,
        }
    }
}

/// How far the sub-issues of an issue are. Only the sub-issues the viewer may see are in it.
#[derive(Serialize, ToSchema)]
pub(crate) struct ProgressBody {
    /// The sub-issues in the group *Done*.
    done: u32,
    /// All the sub-issues the viewer sees.
    total: u32,
}

/// Where an issue stands among the others.
#[derive(Serialize, ToSchema)]
pub(crate) struct IssueRelationsBody {
    /// The internal ID of the issue.
    issue: String,
    /// The version of the issue: what *Part of* is changed from.
    version: i64,
    /// The issue it sits under, when the viewer may see it.
    #[serde(skip_serializing_if = "Option::is_none")]
    part_of: Option<IssueRef>,
    /// How far its sub-issues are; left out without sub-issues, or when the viewer may not see
    /// their status.
    #[serde(skip_serializing_if = "Option::is_none")]
    progress: Option<ProgressBody>,
    /// Whether the viewer may add and take away sub-issues and relations here.
    may_change: bool,
}

impl Masked for IssueRelationsBody {}

impl From<IssueRelations> for IssueRelationsBody {
    fn from(relations: IssueRelations) -> Self {
        Self {
            issue: relations.issue.to_string(),
            version: relations.version,
            part_of: relations.part_of.as_ref().map(IssueRef::from),
            progress: relations.progress.map(|progress| ProgressBody {
                done: progress.done,
                total: progress.total,
            }),
            may_change: relations.may_change,
        }
    }
}

/// One relation as one end of it reads it.
#[derive(Serialize, ToSchema)]
pub(crate) struct RelatedIssueItem {
    /// The ID of the relation, to take it away again.
    id: String,
    /// What it means (the ID of its kind).
    relation_kind: String,
    /// Which end the issue asked about is at.
    direction: Direction,
    /// What that end reads (*blocks*, *is blocked by*).
    name: String,
    /// The issue at the other end.
    issue: IssueRef,
}

impl Masked for RelatedIssueItem {}

impl From<&RelationView> for RelatedIssueItem {
    fn from(view: &RelationView) -> Self {
        Self {
            id: view.id.to_string(),
            relation_kind: view.kind.to_string(),
            direction: view.direction,
            name: view.name.clone(),
            issue: IssueRef::from(&view.issue),
        }
    }
}

/// A kind of relation: the fields that can be asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct RelationKindItem {
    /// The ID of the kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    /// What the issue it starts from reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    forward: Option<String>,
    /// What the other issue reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    backward: Option<String>,
    /// Whether it is retired: old relations keep it, new ones cannot have it.
    #[serde(skip_serializing_if = "Option::is_none")]
    retired: Option<bool>,
    /// Counts up with every change. A change is made on the version that was looked at.
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<i64>,
}

impl Masked for RelationKindItem {}

impl From<&RelationKindView> for RelationKindItem {
    fn from(kind: &RelationKindView) -> Self {
        Self {
            id: Some(kind.id.to_string()),
            forward: Some(kind.forward.clone()),
            backward: Some(kind.backward.clone()),
            retired: Some(kind.retired),
            version: Some(kind.version),
        }
    }
}

/// A kind of relation to make.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewRelationKindBody {
    /// What the issue it starts from reads (*blocks*).
    forward: String,
    /// What the other issue reads (*is blocked by*). The same words for a kind that reads alike
    /// from both ends (*related to*).
    backward: String,
}

/// What to change about a kind of relation: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeRelationKindBody {
    /// The version the person looked at.
    version: i64,
    /// A new forward name.
    forward: Option<String>,
    /// A new backward name.
    backward: Option<String>,
    /// `true` retires the kind (no new relations of it), `false` offers it again.
    retired: Option<bool>,
}

/// A relation to add.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewRelationBody {
    /// The issue at the other end, by its readable or internal ID.
    other: String,
    /// What the relation means (the ID of its kind).
    relation_kind: String,
    /// Which end the issue of the route is: *this issue blocks the other* is `forward`.
    direction: Direction,
}

/// An issue to put under another one, or to take out.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ParentBody {
    /// The version of the sub-issue the person looked at.
    version: i64,
    /// The issue it goes under, by its readable or internal ID; `null` takes it out from under its
    /// parent.
    parent: Option<String>,
}

/// The kinds of relation in the order they are offered in, a page at a time, retired ones too.
/// Open to everybody who is signed in.
#[utoipa::path(
    get,
    path = "/api/v1/relation-kinds",
    operation_id = "listRelationKinds",
    tag = "relations",
    params(ListParams),
    responses((status = 200, description = "The kinds of relation.", body = Page<RelationKindItem>), NotSignedInAnswer)
)]
async fn list_relation_kinds(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(KINDS_LIST);
    let page = auth
        .api()?
        .relation_kinds(
            info,
            RelationKindsQuery {
                session_token,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(RelationKindItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&KIND_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(KINDS_LIST, &list, &fields)?))
}

/// Creates a kind of relation. Needs the permission *Manage issue types, kinds of relation and
/// layouts*.
#[utoipa::path(
    post,
    path = "/api/v1/relation-kinds",
    operation_id = "createRelationKind",
    tag = "relations",
    request_body = NewRelationKindBody,
    responses(
        (status = 200, description = "The kind of relation.", body = RelationKindItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        ConflictAnswer
    )
)]
async fn create_relation_kind(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewRelationKindBody>,
) -> Result<Response, ApiError> {
    let made = auth
        .api()?
        .create_relation_kind(
            info,
            CreateRelationKind {
                session_token,
                forward: body.forward,
                backward: body.backward,
            },
        )
        .await?;
    answer_authorized(&made.map(|kind| RelationKindItem::from(&kind)))
}

/// Changes the names of a kind of relation, or retires it (or offers it again). Needs the
/// permission *Manage issue types, kinds of relation and layouts*.
#[utoipa::path(
    patch,
    path = "/api/v1/relation-kinds/{relation_kind}",
    operation_id = "changeRelationKind",
    tag = "relations",
    params(("relation_kind" = String, Path, description = "The ID of the kind of relation.")),
    request_body = ChangeRelationKindBody,
    responses(
        (status = 200, description = "The kind of relation as it is now.", body = RelationKindItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_relation_kind(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(kind): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeRelationKindBody>,
) -> Result<Response, ApiError> {
    let kind: RelationKindId = path_id(&kind)?;
    let changed = auth
        .api()?
        .change_relation_kind(
            info,
            ChangeRelationKind {
                session_token,
                kind,
                version: body.version,
                forward: body.forward,
                backward: body.backward,
                retired: body.retired,
            },
        )
        .await?;
    answer_authorized(&changed.map(|kind| RelationKindItem::from(&kind)))
}

/// How an issue stands among the others: the issue it sits under and the progress of its
/// sub-issues. The lists are `sub-issues` and `related-issues`.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/relations",
    operation_id = "getIssueRelations",
    tag = "relations",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    responses((status = 200, description = "How the issue stands.", body = IssueRelationsBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_issue_relations(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .issue_relations(
            info,
            IssueRelationsQuery {
                session_token,
                issue,
            },
        )
        .await?;
    answer_authorized(&found.map(IssueRelationsBody::from))
}

/// The sub-issues of an issue that the person may see, a page at a time, in the order they were
/// put there.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/sub-issues",
    operation_id = "listSubIssues",
    tag = "relations",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue."), ListParams),
    responses((status = 200, description = "The sub-issues.", body = Page<IssueRef>), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_sub_issues(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(SUB_ISSUES_LIST);
    let page = auth
        .api()?
        .sub_issues(
            info,
            SubIssuesQuery {
                session_token,
                issue,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(IssueRef::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&ISSUE_REF_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(
        SUB_ISSUES_LIST,
        &list,
        &fields,
    )?))
}

/// The issues related to an issue, whose other end the person may see, a page at a time, by kind
/// and then oldest first. Each relation reads in the words of this issue's end.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/related-issues",
    operation_id = "listRelatedIssues",
    tag = "relations",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue."), ListParams),
    responses((status = 200, description = "The related issues.", body = Page<RelatedIssueItem>), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_related_issues(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(RELATED_LIST);
    let page = auth
        .api()?
        .related_issues(
            info,
            RelatedIssuesQuery {
                session_token,
                issue,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(RelatedIssueItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&RELATED_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(RELATED_LIST, &list, &fields)?))
}

/// Puts an issue under another one, or takes it out from under its parent. Needs the right to
/// change both issues. No issue sits under itself or under one of its own sub-issues.
#[utoipa::path(
    put,
    path = "/api/v1/issues/{issue}/parent",
    operation_id = "setIssueParent",
    tag = "relations",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the sub-issue.")),
    request_body = ParentBody,
    responses(
        (status = 200, description = "How the issue stands now.", body = IssueRelationsBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn set_issue_parent(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiJson(body): ApiJson<ParentBody>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .set_parent(
            info,
            SetParent {
                session_token,
                issue,
                version: body.version,
                parent: body.parent,
            },
        )
        .await?;
    answer_authorized(&changed.map(IssueRelationsBody::from))
}

/// Adds a relation between this issue and another. Needs the right to change this issue and to see
/// the other. The relation shows on both issues, each in its own words.
#[utoipa::path(
    post,
    path = "/api/v1/issues/{issue}/relations",
    operation_id = "addRelation",
    tag = "relations",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    request_body = NewRelationBody,
    responses(
        (status = 200, description = "The relation as this issue reads it.", body = RelatedIssueItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn add_relation(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiJson(body): ApiJson<NewRelationBody>,
) -> Result<Response, ApiError> {
    let added = auth
        .api()?
        .add_relation(
            info,
            AddRelation {
                session_token,
                issue,
                other: body.other,
                kind: body_id::<RelationKindId>(&body.relation_kind)?,
                direction: body.direction,
            },
        )
        .await?;
    answer_authorized(&added.map(|relation| RelatedIssueItem::from(&relation)))
}

/// Takes a relation away (*Undo* adds it again). Needs the right to change one of its issues.
#[utoipa::path(
    delete,
    path = "/api/v1/relations/{relation}",
    operation_id = "removeRelation",
    tag = "relations",
    params(("relation" = String, Path, description = "The ID of the relation.")),
    responses(
        (status = 204, description = "The relation is gone."),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn remove_relation(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(relation): ApiPath<String>,
) -> Result<Response, ApiError> {
    let relation: RelationId = path_id(&relation)?;
    auth.api()?
        .remove_relation(
            info,
            RemoveRelation {
                session_token,
                relation,
            },
        )
        .await?;
    Ok(done())
}

/// The routes of sub-issues and related issues.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_relation_kinds, create_relation_kind))
        .routes(routes!(change_relation_kind))
        .routes(routes!(get_issue_relations, add_relation))
        .routes(routes!(list_sub_issues))
        .routes(routes!(list_related_issues))
        .routes(routes!(set_issue_parent))
        .routes(routes!(remove_relation))
}

#[cfg(test)]
mod tests {
    use bb_policy::PersonRef;

    use super::*;

    #[test]
    fn perf_02_the_field_tables_name_exactly_the_fields_of_the_items() {
        let line = IssueRef {
            id: String::new(),
            readable_id: String::new(),
            project: ProjectRef {
                id: String::new(),
                short_name: String::new(),
                name: String::new(),
            },
            issue_type: IssueTypeRef {
                id: String::new(),
                name: String::new(),
                icon: bb_core::issue::IssueIcon::Bug,
                retired: false,
            },
            title: Some(String::new()),
            status: Some(StatusRef::blank()),
            priority: Some(Priority::Normal),
        };
        crate::lean::assert_fields_are_the_keys_of(&ISSUE_REF_FIELDS, &line);
        let kind = RelationKindItem {
            id: Some(String::new()),
            forward: Some(String::new()),
            backward: Some(String::new()),
            retired: Some(false),
            version: Some(1),
        };
        crate::lean::assert_fields_are_the_keys_of(&KIND_FIELDS, &kind);
        let related = RelatedIssueItem {
            id: String::new(),
            relation_kind: String::new(),
            direction: Direction::Forward,
            name: String::new(),
            issue: line,
        };
        crate::lean::assert_fields_are_the_keys_of(&RELATED_FIELDS, &related);
        // A person ref needs no table here; the import keeps the test honest about the kinds.
        let _ = PersonRef::Someone;
    }
}
