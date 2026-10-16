//! Creating and changing a field and its choices.

use super::{
    AddChoice, AddChoiceBody, ApiError, ApiJson, ApiPath, AuthHandle, ChangeChoice,
    ChangeChoiceBody, ChangeField, ChangeFieldBody, ChoiceId, CombineChoices, CombineChoicesBody,
    ConflictAnswer, CreateField, FieldBody, FieldId, ForbiddenAnswer, Info, InvalidAnswer,
    NewChoice, NewFieldBody, NotFoundAnswer, NotSignedInAnswer, ReorderChoices, ReorderChoicesBody,
    Response, ScopeInput, Session, State, answer_authorized, body_id, path_id,
};

/// Creates an own field. Needs the permission *Manage fields*.
#[utoipa::path(
    post,
    path = "/api/v1/fields",
    operation_id = "createField",
    tag = "fields",
    request_body = NewFieldBody,
    responses(
        (status = 200, description = "The field.", body = FieldBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn create_field(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewFieldBody>,
) -> Result<Response, ApiError> {
    let scope = body.scope.unwrap_or_default().into_scope()?;
    let choices = body
        .choices
        .unwrap_or_default()
        .into_iter()
        .map(|choice| NewChoice {
            name: choice.name,
            color: choice.color,
            icon: choice.icon,
        })
        .collect();
    let made = auth
        .api()?
        .create_field(
            info,
            CreateField {
                session_token,
                name: body.name,
                help: body.help,
                kind: body.kind,
                applies_to: body.applies_to,
                rules: body.rules.unwrap_or_default(),
                default_value: body.default_value,
                look: body.look,
                never_to_assistant: body.never_to_assistant.unwrap_or(false),
                scope,
                choices,
            },
        )
        .await?;
    answer_authorized(&made.map(|field| FieldBody::from(&field)))
}

/// Changes an own field, or retires it (or offers it again). Needs the permission *Manage
/// fields*.
#[utoipa::path(
    patch,
    path = "/api/v1/fields/{field}",
    operation_id = "changeField",
    tag = "fields",
    params(("field" = String, Path, description = "The ID of the field.")),
    request_body = ChangeFieldBody,
    responses(
        (status = 200, description = "The field as it is now.", body = FieldBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn change_field(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(field): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeFieldBody>,
) -> Result<Response, ApiError> {
    let field = path_id::<FieldId>(&field)?;
    let scope = body.scope.map(ScopeInput::into_scope).transpose()?;
    let changed = auth
        .api()?
        .change_field(
            info,
            ChangeField {
                session_token,
                field,
                version: body.version,
                name: body.name,
                help: body.help,
                rules: body.rules,
                default_value: body.default_value,
                look: body.look,
                never_to_assistant: body.never_to_assistant,
                scope,
                retired: body.retired,
            },
        )
        .await?;
    answer_authorized(&changed.map(|field| FieldBody::from(&field)))
}

/// Adds a choice to a *Pick one* or *Pick several* field. Needs the permission *Manage fields*.
#[utoipa::path(
    post,
    path = "/api/v1/fields/{field}/choices",
    operation_id = "addChoice",
    tag = "fields",
    params(("field" = String, Path, description = "The ID of the field.")),
    request_body = AddChoiceBody,
    responses(
        (status = 200, description = "The field as it is now.", body = FieldBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn add_choice(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(field): ApiPath<String>,
    ApiJson(body): ApiJson<AddChoiceBody>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .add_choice(
            info,
            AddChoice {
                session_token,
                field: path_id::<FieldId>(&field)?,
                version: body.version,
                name: body.name,
                color: body.color,
                icon: body.icon,
            },
        )
        .await?;
    answer_authorized(&changed.map(|field| FieldBody::from(&field)))
}

/// Changes the name, color or icon of a choice, or retires it. Needs the permission *Manage
/// fields*.
#[utoipa::path(
    patch,
    path = "/api/v1/fields/{field}/choices/{choice}",
    operation_id = "changeChoice",
    tag = "fields",
    params(
        ("field" = String, Path, description = "The ID of the field."),
        ("choice" = String, Path, description = "The ID of the choice.")
    ),
    request_body = ChangeChoiceBody,
    responses(
        (status = 200, description = "The field as it is now.", body = FieldBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn change_choice(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((field, choice)): ApiPath<(String, String)>,
    ApiJson(body): ApiJson<ChangeChoiceBody>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .change_choice(
            info,
            ChangeChoice {
                session_token,
                field: path_id::<FieldId>(&field)?,
                choice: path_id::<ChoiceId>(&choice)?,
                version: body.version,
                name: body.name,
                color: body.color,
                icon: body.icon,
                retired: body.retired,
            },
        )
        .await?;
    answer_authorized(&changed.map(|field| FieldBody::from(&field)))
}

/// Puts the choices of a field in a new order. Needs the permission *Manage fields*.
#[utoipa::path(
    put,
    path = "/api/v1/fields/{field}/choices/order",
    operation_id = "reorderChoices",
    tag = "fields",
    params(("field" = String, Path, description = "The ID of the field.")),
    request_body = ReorderChoicesBody,
    responses(
        (status = 200, description = "The field as it is now.", body = FieldBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn reorder_choices(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(field): ApiPath<String>,
    ApiJson(body): ApiJson<ReorderChoicesBody>,
) -> Result<Response, ApiError> {
    let order = body
        .order
        .iter()
        .map(|text| body_id::<ChoiceId>(text))
        .collect::<Result<Vec<_>, _>>()?;
    let changed = auth
        .api()?
        .reorder_choices(
            info,
            ReorderChoices {
                session_token,
                field: path_id::<FieldId>(&field)?,
                version: body.version,
                order,
            },
        )
        .await?;
    answer_authorized(&changed.map(|field| FieldBody::from(&field)))
}

/// Moves every value of a choice to another choice and retires the first (*Combine with another
/// choice*). Every item it touches gets a record in its history. Needs the permission *Manage
/// fields*.
#[utoipa::path(
    post,
    path = "/api/v1/fields/{field}/choices/{choice}/combine",
    operation_id = "combineChoices",
    tag = "fields",
    params(
        ("field" = String, Path, description = "The ID of the field."),
        ("choice" = String, Path, description = "The ID of the choice that goes away.")
    ),
    request_body = CombineChoicesBody,
    responses(
        (status = 200, description = "The field as it is now.", body = FieldBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn combine_choices(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((field, choice)): ApiPath<(String, String)>,
    ApiJson(body): ApiJson<CombineChoicesBody>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .combine_choices(
            info,
            CombineChoices {
                session_token,
                field: path_id::<FieldId>(&field)?,
                choice: path_id::<ChoiceId>(&choice)?,
                into: body_id::<ChoiceId>(&body.into)?,
                version: body.version,
            },
        )
        .await?;
    answer_authorized(&changed.map(|field| FieldBody::from(&field)))
}
