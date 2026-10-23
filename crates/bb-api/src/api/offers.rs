//! Where issue types and groups of labels are offered (ISS-02, TAG-02): the rows of a type or a
//! group, read a page at a time, and one change of several places together. The rows are judged at
//! the place they are for: *Manage issue types, kinds of relation and layouts* (or *Manage labels
//! and tags*) in that project, or for the whole organization.

use axum::extract::State;
use axum::response::Response;
use bb_app::auth::RequestInfo;
use bb_app::offers::{ChangeOffers, OfferChange, OfferRowView, Offers, OffersQuery};
use bb_core::id::{IssueTypeId, LabelGroupId};
use bb_core::offer::OfferSubject;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::{PlaceRef, cursor_error};
use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer, answer_authorized,
};
use super::path_id;
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The fields of a row in the list of rows: all of them are by default, a row is small.
const OFFER_ROW_FIELDS: [FieldSpec; 3] = [
    FieldSpec::by_default("place"),
    FieldSpec::by_default("offered"),
    FieldSpec::by_default("may_change"),
];

/// The name of the lists, so their cursors are their own.
const ISSUE_TYPE_OFFERS_LIST: &str = "issue-type-offers";
const LABEL_GROUP_OFFERS_LIST: &str = "label-group-offers";

/// The project a list of issue types or of groups of labels is asked about: each then says whether
/// the project offers it (the fields `offered_here`, `own_offer` and `may_change_offer`).
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct OfferProject {
    /// The ID or short name of a project.
    pub(crate) project: Option<String>,
}

/// One row of where a subject is offered.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct OfferRowItem {
    /// The project (and its sub-projects) or the whole organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(nested)]
    place: Option<PlaceRef>,
    /// Whether the subject is offered there.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    offered: Option<bool>,
    /// Whether the person who asks may change the row.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    may_change: Option<bool>,
}

impl From<&OfferRowView> for OfferRowItem {
    fn from(row: &OfferRowView) -> Self {
        Self {
            place: Some(PlaceRef::from(row.place.as_ref())),
            offered: Some(row.offered),
            may_change: Some(row.may_change),
        }
    }
}

/// Where a subject is offered after a change.
#[derive(Serialize, ToSchema)]
pub(crate) struct OffersBody {
    /// Every row there is.
    rows: Vec<OfferRowItem>,
    /// The version the subject has now.
    version: i64,
}

impl Masked for OffersBody {}

impl From<&Offers> for OffersBody {
    fn from(offers: &Offers) -> Self {
        Self {
            rows: offers.rows.iter().map(OfferRowItem::from).collect(),
            version: offers.version,
        }
    }
}

/// One place to change.
#[derive(Deserialize, ToSchema)]
pub(crate) struct OfferChangeBody {
    /// The ID or short name of the project; left out for the whole organization.
    project: Option<String>,
    /// `true` offers the subject there, `false` does not; left out takes the row away, so the place
    /// follows the place above it again.
    offered: Option<bool>,
}

/// The places to change together, on the version that was looked at.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeOffersBody {
    /// The version the person looked at.
    version: i64,
    /// The places. A place may be named once.
    changes: Vec<OfferChangeBody>,
}

impl ChangeOffersBody {
    fn changes(self) -> (i64, Vec<OfferChange>) {
        let changes = self
            .changes
            .into_iter()
            .map(|change| OfferChange {
                project: change.project,
                offered: change.offered,
            })
            .collect();
        (self.version, changes)
    }
}

/// Reads the rows of a subject a page at a time.
async fn list(
    auth: AuthHandle,
    (info, session_token): (RequestInfo, String),
    params: ListParams,
    (name, subject): (&str, OfferSubject),
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(name);
    let page = auth
        .api()?
        .offers(
            info,
            OffersQuery {
                session_token,
                subject,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(OfferRowItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&OFFER_ROW_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(name, &list, &fields)?))
}

/// Changes the rows of a subject.
async fn change(
    auth: AuthHandle,
    (info, session_token): (RequestInfo, String),
    subject: OfferSubject,
    body: ChangeOffersBody,
) -> Result<Response, ApiError> {
    let (version, changes) = body.changes();
    let after = auth
        .api()?
        .change_offers(
            info,
            ChangeOffers {
                session_token,
                subject,
                version,
                changes,
            },
        )
        .await?;
    answer_authorized(&after.map(|offers| OffersBody::from(&offers)))
}

/// Where an issue type is offered: the rows that say so, a page at a time. Open to everybody who is
/// signed in.
#[utoipa::path(
    get,
    path = "/api/v1/issue-types/{issue_type}/offers",
    operation_id = "listIssueTypeOffers",
    tag = "issue-types",
    params(("issue_type" = String, Path, description = "The ID of the issue type."), ListParams),
    responses((status = 200, description = "The rows.", body = Page<OfferRowItem>), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_issue_type_offers(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue_type): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let issue_type: IssueTypeId = path_id(&issue_type)?;
    list(
        auth,
        (info, session_token),
        params,
        (ISSUE_TYPE_OFFERS_LIST, OfferSubject::IssueType(issue_type)),
    )
    .await
}

/// Changes where an issue type is offered: all the places together or none. Each place is judged on
/// its own: it needs the permission *Manage issue types, kinds of relation and layouts* there.
#[utoipa::path(
    put,
    path = "/api/v1/issue-types/{issue_type}/offers",
    operation_id = "changeIssueTypeOffers",
    tag = "issue-types",
    params(("issue_type" = String, Path, description = "The ID of the issue type.")),
    request_body = ChangeOffersBody,
    responses(
        (status = 200, description = "Every row, and the version of the type.", body = OffersBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_issue_type_offers(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue_type): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeOffersBody>,
) -> Result<Response, ApiError> {
    let issue_type: IssueTypeId = path_id(&issue_type)?;
    change(
        auth,
        (info, session_token),
        OfferSubject::IssueType(issue_type),
        body,
    )
    .await
}

/// Where a group of labels is offered: the rows that say so, a page at a time. Open to everybody who
/// is signed in.
#[utoipa::path(
    get,
    path = "/api/v1/label-groups/{label_group}/offers",
    operation_id = "listLabelGroupOffers",
    tag = "tags",
    params(("label_group" = String, Path, description = "The ID of the label group."), ListParams),
    responses((status = 200, description = "The rows.", body = Page<OfferRowItem>), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_label_group_offers(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(group): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let group: LabelGroupId = path_id(&group)?;
    list(
        auth,
        (info, session_token),
        params,
        (LABEL_GROUP_OFFERS_LIST, OfferSubject::LabelGroup(group)),
    )
    .await
}

/// Changes where a group of labels is offered: all the places together or none. Each place is
/// judged on its own: it needs the permission *Manage labels and tags* there.
#[utoipa::path(
    put,
    path = "/api/v1/label-groups/{label_group}/offers",
    operation_id = "changeLabelGroupOffers",
    tag = "tags",
    params(("label_group" = String, Path, description = "The ID of the label group.")),
    request_body = ChangeOffersBody,
    responses(
        (status = 200, description = "Every row, and the version of the group.", body = OffersBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_label_group_offers(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(group): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeOffersBody>,
) -> Result<Response, ApiError> {
    let group: LabelGroupId = path_id(&group)?;
    change(
        auth,
        (info, session_token),
        OfferSubject::LabelGroup(group),
        body,
    )
    .await
}

/// The routes of the offers.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_issue_type_offers, change_issue_type_offers))
        .routes(routes!(list_label_group_offers, change_label_group_offers))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_a_row() {
        let item = OfferRowItem {
            place: Some(PlaceRef::Organization),
            offered: Some(true),
            may_change: Some(true),
        };
        crate::lean::assert_fields_are_the_keys_of(&OFFER_ROW_FIELDS, &item);
    }
}
