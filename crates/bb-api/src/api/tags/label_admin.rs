//! Listing, creating, previewing, changing and combining labels and label groups (TAG-01).

use super::{
    ApiError, ApiJson, ApiPath, ApiQuery, AuthHandle, ChangeLabel, ChangeLabelBody,
    ChangeLabelGroup, ChangeLabelGroupBody, ChangePreviewBody, CombineBody, CombineLabels,
    ConflictAnswer, CreateLabel, CreateLabelGroup, Edit, ForbiddenAnswer, Info, InvalidAnswer,
    LabelBody, LabelChangePreviewQuery, LabelGroupBody, LabelGroupId, LabelId, LabelsBody,
    LabelsQuery, NewLabelBody, NewLabelGroupBody, NotFoundAnswer, NotSignedInAnswer, OfferProject,
    PreviewChangeBody, Response, Session, State, answer_authorized, body_id, path_id,
};

/// The labels of the organization with their groups. Open to everybody who is signed in: the
/// people who put labels on issues choose from them. With `project`, each group says whether that
/// project offers it.
#[utoipa::path(
    get,
    path = "/api/v1/labels",
    operation_id = "listLabels",
    tag = "tags",
    params(OfferProject),
    responses((status = 200, description = "The groups and the labels.", body = LabelsBody), NotSignedInAnswer, NotFoundAnswer)
)]
pub(super) async fn list_labels(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(asked): ApiQuery<OfferProject>,
) -> Result<Response, ApiError> {
    let labels = auth
        .api()?
        .labels(
            info,
            LabelsQuery {
                session_token,
                project: asked.project,
            },
        )
        .await?;
    answer_authorized(&labels.map(LabelsBody::from))
}

/// Creates a group of labels for the whole organization or in a project. Needs *Manage labels and
/// tags* where it is made.
#[utoipa::path(
    post,
    path = "/api/v1/label-groups",
    operation_id = "createLabelGroup",
    tag = "tags",
    request_body = NewLabelGroupBody,
    responses(
        (status = 200, description = "The group.", body = LabelGroupBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn create_label_group(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewLabelGroupBody>,
) -> Result<Response, ApiError> {
    let made = auth
        .api()?
        .create_label_group(
            info,
            CreateLabelGroup {
                session_token,
                project: body.project,
                name: body.name,
                pick_one: body.pick_one,
            },
        )
        .await?;
    answer_authorized(&made.map(|group| LabelGroupBody::from(&group)))
}

/// Changes a group of labels. Needs *Manage labels and tags* where the group was made.
#[utoipa::path(
    patch,
    path = "/api/v1/label-groups/{group}",
    operation_id = "changeLabelGroup",
    tag = "tags",
    params(("group" = String, Path, description = "The ID of the group.")),
    request_body = ChangeLabelGroupBody,
    responses(
        (status = 200, description = "The group as it is now.", body = LabelGroupBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn change_label_group(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(group): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeLabelGroupBody>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .change_label_group(
            info,
            ChangeLabelGroup {
                session_token,
                group: path_id::<LabelGroupId>(&group)?,
                version: body.version,
                name: body.name,
                pick_one: body.pick_one,
                retired: body.retired,
            },
        )
        .await?;
    answer_authorized(&changed.map(|group| LabelGroupBody::from(&group)))
}

/// Creates a label. Needs *Manage labels and tags*.
#[utoipa::path(
    post,
    path = "/api/v1/labels",
    operation_id = "createLabel",
    tag = "tags",
    request_body = NewLabelBody,
    responses(
        (status = 200, description = "The label.", body = LabelBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn create_label(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewLabelBody>,
) -> Result<Response, ApiError> {
    let group = body
        .group
        .as_deref()
        .map(body_id::<LabelGroupId>)
        .transpose()?;
    let made = auth
        .api()?
        .create_label(
            info,
            CreateLabel {
                session_token,
                name: body.name,
                color: body.color,
                group,
            },
        )
        .await?;
    answer_authorized(&made.map(|label| LabelBody::from(&label)))
}

/// What renaming a label, or combining it with another, would do to the issues the person sees.
/// Needs *Manage labels and tags*.
#[utoipa::path(
    post,
    path = "/api/v1/labels/{label}/preview",
    operation_id = "previewLabelChange",
    tag = "tags",
    params(("label" = String, Path, description = "The ID of the label.")),
    request_body = PreviewChangeBody,
    responses(
        (status = 200, description = "What would change.", body = ChangePreviewBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn preview_label_change(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(label): ApiPath<String>,
    ApiJson(body): ApiJson<PreviewChangeBody>,
) -> Result<Response, ApiError> {
    let preview = auth
        .api()?
        .label_change_preview(
            info,
            LabelChangePreviewQuery {
                session_token,
                label: path_id::<LabelId>(&label)?,
                name: body.name,
                into: body.into.as_deref().map(body_id::<LabelId>).transpose()?,
            },
        )
        .await?;
    answer_authorized(&preview.map(ChangePreviewBody::from))
}

/// Changes a label: its name (every issue that has it records the new name), color, group and
/// whether it is retired. Needs *Manage labels and tags*.
#[utoipa::path(
    patch,
    path = "/api/v1/labels/{label}",
    operation_id = "changeLabel",
    tag = "tags",
    params(("label" = String, Path, description = "The ID of the label.")),
    request_body = ChangeLabelBody,
    responses(
        (status = 200, description = "The label as it is now.", body = LabelBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn change_label(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(label): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeLabelBody>,
) -> Result<Response, ApiError> {
    let group = match body.group {
        Edit::Keep => Edit::Keep,
        Edit::Clear => Edit::Clear,
        Edit::Set(text) => Edit::Set(body_id::<LabelGroupId>(&text)?),
    };
    let changed = auth
        .api()?
        .change_label(
            info,
            ChangeLabel {
                session_token,
                label: path_id::<LabelId>(&label)?,
                version: body.version,
                name: body.name,
                color: body.color,
                group,
                retired: body.retired,
            },
        )
        .await?;
    answer_authorized(&changed.map(|label| LabelBody::from(&label)))
}

/// Moves every issue from this label to another one, then takes this label away. Needs *Manage
/// labels and tags*.
#[utoipa::path(
    post,
    path = "/api/v1/labels/{label}/combine",
    operation_id = "combineLabels",
    tag = "tags",
    params(("label" = String, Path, description = "The ID of the label that goes away.")),
    request_body = CombineBody,
    responses(
        (status = 200, description = "The label that stays.", body = LabelBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
pub(super) async fn combine_labels(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(label): ApiPath<String>,
    ApiJson(body): ApiJson<CombineBody>,
) -> Result<Response, ApiError> {
    let kept = auth
        .api()?
        .combine_labels(
            info,
            CombineLabels {
                session_token,
                label: path_id::<LabelId>(&label)?,
                version: body.version,
                into: body_id::<LabelId>(&body.into)?,
                into_version: body.into_version,
            },
        )
        .await?;
    answer_authorized(&kept.map(|label| LabelBody::from(&label)))
}
