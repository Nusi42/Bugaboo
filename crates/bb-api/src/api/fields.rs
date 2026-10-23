//! The own-field routes below `/api/v1/fields` (CF-01, CF-03, CF-04, CF-06, CF-07): the fields a
//! person sees, and making, changing and retiring a field and its choices for those who hold
//! *Manage fields*. A field is retired, never deleted.
//!
//! A field the person may not see is in no answer and answers `resource.not-found` like one that
//! does not exist. Every change names the `version` it looked at. The use cases are in `bb-app`,
//! behind [`AuthApi`](bb_app::auth::AuthApi).

use axum::extract::State;
use axum::response::Response;
use bb_app::fields::{
    AddChoice, ChangeChoice, ChangeField, ChoiceView, CombineChoices, CreateField, FieldQuery,
    FieldTypePage, FieldTypesQuery, FieldUsage, FieldView, FieldsQuery, NewChoice, ReorderChoices,
    SaveFieldText, ScopeView,
};
use bb_core::id::{ChoiceId, FieldId, IssueTypeId, LockToken, ProjectId};
use bb_core::issue::IssueIcon;
use bb_core::own_fields::{ChoiceColor, Edit, FieldFor, FieldType, KindTrait, Look, Rules, Scope};
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized,
};
use super::issues::{IssueItem, IssueTypeRef};
use super::projects::ProjectRef;
use super::{body_id, path_id};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};

mod bodies;
mod catalog;
mod manage;
mod text;

pub(crate) use bodies::*;
use catalog::{
    __path_get_field, __path_get_field_used_by, __path_list_field_types, __path_list_fields,
    get_field, get_field_used_by, list_field_types, list_fields,
};
use manage::{
    __path_add_choice, __path_change_choice, __path_change_field, __path_combine_choices,
    __path_create_field, __path_reorder_choices, add_choice, change_choice, change_field,
    combine_choices, create_field, reorder_choices,
};
use text::{__path_save_issue_field_text, save_issue_field_text};

/// The routes of the own fields.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_field_types))
        .routes(routes!(list_fields, create_field))
        .routes(routes!(get_field, change_field))
        .routes(routes!(get_field_used_by))
        .routes(routes!(add_choice))
        .routes(routes!(reorder_choices))
        .routes(routes!(change_choice))
        .routes(routes!(combine_choices))
        .routes(routes!(save_issue_field_text))
}
