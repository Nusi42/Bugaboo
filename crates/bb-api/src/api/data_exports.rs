//! The routes of *Download my data* (PRIV-03, ADR-0044, decisions 2 and 4): ask for the file of
//! one's own data and see where it stands, ask for somebody else's with *Download someone's data*,
//! list every export, and download a file through a signed link that is good for five minutes.

use std::collections::BTreeMap;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse as _, Response};
use bb_app::privacy::export::{
    AskMyData, AskSomeonesData, DataExportsQuery, DownloadExportQuery, ExportLinkQuery, ExportView,
    MyDataExportQuery,
};
use bb_core::data_export::ExportState;
use bb_core::error_code::PROBLEM_CONTENT_TYPE;
use bb_core::id::UserId;
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::openapi::response::{Response as ResponseDescription, ResponseBuilder};
use utoipa::openapi::{ContentBuilder, Ref, RefOr};
use utoipa::{IntoResponses, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer, answer_authorized, problem_answers,
};
use super::files::{FileBytes, body_of, download_headers};
use super::{VERSION_PATH, path_id};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The name of the list of exports in its cursors.
const EXPORTS_LIST: &str = "data-exports";

/// The fields of an export in the list.
const ITEM_FIELDS: [FieldSpec; 9] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("person"),
    FieldSpec::by_default("asked_by"),
    FieldSpec::by_default("reason"),
    FieldSpec::by_default("status"),
    FieldSpec::by_default("asked_at"),
    FieldSpec::by_default("ready_at"),
    FieldSpec::by_default("bytes"),
    FieldSpec::by_default("deleted_at"),
];

problem_answers! {
    /// A file of the person's data is being created already.
    WaitingAnswer => ("409", "A file of this person's data is being created already (data-export.waiting); one at a time."),
}

fn url_of(link: &str) -> String {
    format!("{VERSION_PATH}/data-exports/download/{link}")
}

/// A file of *Download my data*.
#[derive(Serialize, ToSchema)]
pub(crate) struct DataExportItem {
    /// The ID of the export.
    id: String,
    /// Whose data the file holds, as the reader may see them.
    person: PersonRef,
    /// Who asked for it: the person, or an administrator.
    asked_by: PersonRef,
    /// Why an administrator asked for somebody else's data.
    #[schema(required = true)]
    reason: Option<String>,
    /// Where it stands.
    status: ExportState,
    /// When it was asked for.
    asked_at: String,
    /// When the file was ready.
    #[schema(required = true)]
    ready_at: Option<String>,
    /// The size of the file in bytes, once it is ready.
    #[schema(required = true)]
    bytes: Option<i64>,
    /// When the file and its record are deleted: 7 days after it was asked for.
    deleted_at: String,
}

impl Masked for DataExportItem {}

impl From<&ExportView> for DataExportItem {
    fn from(view: &ExportView) -> Self {
        let export = &view.export;
        Self {
            id: export.id.to_string(),
            person: view.person.clone(),
            asked_by: view.asked_by.clone(),
            reason: export.reason.clone(),
            status: export.state,
            asked_at: export.asked_at.to_string(),
            ready_at: export.ready_at.map(|moment| moment.to_string()),
            bytes: export.bytes,
            deleted_at: export.deleted_at().to_string(),
        }
    }
}

/// The newest file of the person's own data, if there is one.
#[derive(Serialize, ToSchema)]
pub(crate) struct MyDataExportBody {
    /// The export; `null` before the first and once it is deleted.
    #[schema(required = true)]
    export: Option<DataExportItem>,
}

impl Masked for MyDataExportBody {}

/// Why an administrator asks for somebody else's data; the person is told.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AskSomeonesDataBody {
    /// The reason, in the administrator's own words (at most 500 characters).
    reason: String,
}

/// The signed address that downloads a file of an export.
#[derive(Serialize, ToSchema)]
pub(crate) struct DataExportAddressBody {
    /// Good for five minutes and for the reader only.
    download_url: String,
}

impl Masked for DataExportAddressBody {}

/// Asks for the file of the person's own data. A job creates it; the person is told when it is
/// ready. Only a session asks, never an access key.
#[utoipa::path(
    post,
    path = "/api/v1/me/data-export",
    operation_id = "askForMyData",
    tag = "privacy",
    responses(
        (status = 200, description = "The export, waiting for its file.", body = DataExportItem),
        NotSignedInAnswer,
        ForbiddenAnswer,
        WaitingAnswer
    )
)]
async fn ask_for_my_data(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let asked = auth
        .api()?
        .ask_for_my_data(info, AskMyData { session_token })
        .await?;
    answer_authorized(&asked.map(|view| DataExportItem::from(&view)))
}

/// Where the newest file of the person's own data stands.
#[utoipa::path(
    get,
    path = "/api/v1/me/data-export",
    operation_id = "getMyDataExport",
    tag = "privacy",
    responses(
        (status = 200, description = "The newest export, or nothing.", body = MyDataExportBody),
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn my_data_export(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .my_data_export(info, MyDataExportQuery { session_token })
        .await?;
    answer_authorized(&found.map(|view| MyDataExportBody {
        export: view.as_ref().map(DataExportItem::from),
    }))
}

/// Asks, with *Download someone's data*, for the file of somebody else's data, always with a
/// reason. The person is told, and the security log records it.
#[utoipa::path(
    post,
    path = "/api/v1/people/{person}/data-export",
    operation_id = "askForSomeonesData",
    tag = "privacy",
    params(("person" = String, Path, description = "The ID of the person.")),
    request_body = AskSomeonesDataBody,
    responses(
        (status = 200, description = "The export, waiting for its file.", body = DataExportItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        WaitingAnswer
    )
)]
async fn ask_for_someones_data(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(person): ApiPath<String>,
    ApiJson(body): ApiJson<AskSomeonesDataBody>,
) -> Result<Response, ApiError> {
    let asked = auth
        .api()?
        .ask_for_someones_data(
            info,
            AskSomeonesData {
                session_token,
                person: path_id::<UserId>(&person)?,
                reason: body.reason,
            },
        )
        .await?;
    answer_authorized(&asked.map(|view| DataExportItem::from(&view)))
}

/// Every file of people's data, newest first, for a holder of *Download someone's data*.
#[utoipa::path(
    get,
    path = "/api/v1/admin/data-exports",
    operation_id = "listDataExports",
    tag = "privacy",
    params(ListParams),
    responses(
        (status = 200, description = "The exports.", body = Page<DataExportItem>),
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn list_data_exports(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let (position, bad_cursor) = params.start(EXPORTS_LIST);
    let skip = position
        .as_deref()
        .and_then(|last| last.parse::<u64>().ok())
        .map_or(0, |last| last.saturating_add(1));
    let found = auth
        .api()?
        .data_exports(
            info,
            DataExportsQuery {
                session_token,
                skip,
                limit: u64::from(params.limit()),
            },
        )
        .await?;
    bad_cursor?;
    let list = found.map(|page| {
        let shown = u64::try_from(page.exports.len()).unwrap_or(u64::MAX);
        Listed {
            items: page.exports.iter().map(DataExportItem::from).collect(),
            next: page
                .more
                .then(|| (skip + shown).saturating_sub(1).to_string()),
        }
    });
    let fields = params.fields_readable(&ITEM_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(EXPORTS_LIST, &list, &fields)?))
}

/// The signed address that downloads the file of a ready export, for the person who asked for it.
#[utoipa::path(
    get,
    path = "/api/v1/data-exports/{export}/address",
    operation_id = "getDataExportAddress",
    tag = "privacy",
    params(("export" = String, Path, description = "The ID of the export.")),
    responses(
        (status = 200, description = "The signed address, good for five minutes.", body = DataExportAddressBody),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn data_export_address(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(export): ApiPath<String>,
) -> Result<Response, ApiError> {
    let link = auth
        .api()?
        .export_link(
            info,
            ExportLinkQuery {
                session_token,
                export: export.parse().ok(),
            },
        )
        .await?;
    answer_authorized(&link.map(|link| DataExportAddressBody {
        download_url: url_of(&link.token),
    }))
}

/// Downloads the file of an export through its signed address. The download is written into the
/// security log; the file is sent as an attachment with `nosniff` and a policy that allows nothing.
#[utoipa::path(
    get,
    path = "/api/v1/data-exports/download/{file}",
    operation_id = "downloadDataExport",
    tag = "privacy",
    params(("file" = String, Path, description = "The signed address, as the address of the export gives it.")),
    responses(
        (status = 200, description = "The ZIP file.", content_type = "application/zip", body = FileBytes),
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn download_data_export(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(file): ApiPath<String>,
) -> Result<Response, ApiError> {
    let file = auth
        .api()?
        .download_export(
            info,
            DownloadExportQuery {
                session_token,
                link: file,
            },
        )
        .await?;
    let headers = download_headers(&file.name, &file.media_type, file.body.size);
    Ok((StatusCode::OK, headers, body_of(file.body)).into_response())
}

pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(ask_for_my_data, my_data_export))
        .routes(routes!(ask_for_someones_data))
        .routes(routes!(list_data_exports))
        .routes(routes!(data_export_address))
        .routes(routes!(download_data_export))
}
