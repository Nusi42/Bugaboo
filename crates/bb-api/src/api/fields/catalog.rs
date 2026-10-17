//! The kinds of field, the list of fields, one field and where it is used.

use super::{
    ApiError, ApiPath, ApiQuery, AuthHandle, FieldBody, FieldId, FieldListBody, FieldQuery,
    FieldTypeListBody, FieldTypesQuery, FieldUsedByBody, FieldsParams, FieldsQuery,
    ForbiddenAnswer, Info, NotFoundAnswer, NotSignedInAnswer, Response, Session, State,
    answer_authorized, path_id,
};

/// The kinds of information a field can hold, with what each kind allows.
#[utoipa::path(
    get,
    path = "/api/v1/field-types",
    operation_id = "listFieldTypes",
    tag = "fields",
    responses((status = 200, description = "The kinds of information.", body = FieldTypeListBody), NotSignedInAnswer)
)]
pub(super) async fn list_field_types(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let page = auth
        .api()?
        .field_types(info, FieldTypesQuery { session_token })
        .await?;
    answer_authorized(&page.map(|page| FieldTypeListBody::from(&page)))
}

/// The own fields the person sees, for the organization or for one project.
#[utoipa::path(
    get,
    path = "/api/v1/fields",
    operation_id = "listFields",
    tag = "fields",
    params(FieldsParams),
    responses((status = 200, description = "The fields.", body = FieldListBody), NotSignedInAnswer, NotFoundAnswer)
)]
pub(super) async fn list_fields(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<FieldsParams>,
) -> Result<Response, ApiError> {
    let page = auth
        .api()?
        .fields(
            info,
            FieldsQuery {
                session_token,
                applies_to: params.applies_to,
                project: params.project,
                retired: params.retired.unwrap_or(false),
            },
        )
        .await?;
    answer_authorized(&page.map(|page| FieldListBody::from(&page)))
}

/// One own field the person sees.
#[utoipa::path(
    get,
    path = "/api/v1/fields/{field}",
    operation_id = "getField",
    tag = "fields",
    params(("field" = String, Path, description = "The ID of the field.")),
    responses((status = 200, description = "The field.", body = FieldBody), NotSignedInAnswer, NotFoundAnswer)
)]
pub(super) async fn get_field(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(field): ApiPath<String>,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .field(
            info,
            FieldQuery {
                session_token,
                field: path_id::<FieldId>(&field)?,
            },
        )
        .await?;
    answer_authorized(&found.map(|field| FieldBody::from(&field)))
}

/// Where an own field is used now. Needs the permission *Manage fields*.
#[utoipa::path(
    get,
    path = "/api/v1/fields/{field}/used-by",
    operation_id = "getFieldUsedBy",
    tag = "fields",
    params(("field" = String, Path, description = "The ID of the field.")),
    responses((status = 200, description = "Where the field is used.", body = FieldUsedByBody), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
pub(super) async fn get_field_used_by(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(field): ApiPath<String>,
) -> Result<Response, ApiError> {
    let usage = auth
        .api()?
        .field_usage(
            info,
            FieldQuery {
                session_token,
                field: path_id::<FieldId>(&field)?,
            },
        )
        .await?;
    answer_authorized(&usage.map(|usage| FieldUsedByBody::from(&usage)))
}
