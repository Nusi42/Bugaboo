//! The routes of tags and labels (TAG-01, TAG-02, TAG-03): the tags and labels of an issue below
//! `/api/v1/issues/{issue}`, the tags to offer below `/api/v1/projects/{project}`, and the tags and
//! labels of the organization below `/api/v1/tags`, `/api/v1/labels` and `/api/v1/label-groups` for
//! the people who hold *Manage labels and tags*.
//!
//! A tag or label on an issue is shown only to a person whose role lets them see the field *Tags*
//! or *Labels* of that issue. Renaming and combining ask for the version of what the person looked
//! at, and a preview (`/preview`) says what they would do to the issues the person sees. The use
//! cases are in `bb-app`, behind [`AuthApi`](bb_app::auth::AuthApi).

use axum::extract::State;
use axum::response::Response;
use bb_app::tags::{
    AddLabel, AddTag, ChangeLabel, ChangeLabelGroup, ChangeTag, ChangedIssue, CombineLabels,
    CombineTags, CreateLabel, CreateLabelGroup, IssueLabelsAndTags, IssueLabelsAndTagsQuery,
    LabelChangePreviewQuery, LabelGroupView, LabelRef, LabelView, Labels, LabelsQuery, RemoveLabel,
    RemoveTag, TagChangePreview, TagChangePreviewQuery, TagRef, TagSuggestionsQuery, TagView,
    TagsQuery,
};
use bb_core::id::{LabelGroupId, LabelId, ProjectId, TagId};
use bb_core::own_fields::{ChoiceColor, Edit};
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
use super::issues::IssueTypeRef;
use super::offers::OfferProject;
use super::previews::PreviewSummaryBody;
use super::projects::ProjectRef;
use super::relations::IssueRef;
use super::{body_id, path_id};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

mod bodies;
mod issue;
mod label_admin;
mod tag_admin;

pub(crate) use bodies::*;
use issue::{
    __path_add_label, __path_add_tag, __path_get_issue_labels_and_tags,
    __path_list_tag_suggestions, __path_remove_label, __path_remove_tag, add_label, add_tag,
    get_issue_labels_and_tags, list_tag_suggestions, remove_label, remove_tag,
};
use label_admin::{
    __path_change_label, __path_change_label_group, __path_combine_labels, __path_create_label,
    __path_create_label_group, __path_list_labels, __path_preview_label_change, change_label,
    change_label_group, combine_labels, create_label, create_label_group, list_labels,
    preview_label_change,
};
use tag_admin::{
    __path_change_tag, __path_combine_tags, __path_list_tags, __path_preview_tag_change,
    change_tag, combine_tags, list_tags, preview_tag_change,
};

/// The fields of a tag in the list the people who look after tags read, and which of them the list
/// holds when the request names none.
const TAG_FIELDS: [FieldSpec; 6] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("color"),
    FieldSpec::by_default("whole_organization"),
    FieldSpec::on_request("projects"),
    FieldSpec::by_default("version"),
];
const TAGS_LIST: &str = "tags";

/// The routes of tags and labels.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_issue_labels_and_tags))
        .routes(routes!(add_tag))
        .routes(routes!(remove_tag))
        .routes(routes!(add_label))
        .routes(routes!(remove_label))
        .routes(routes!(list_tag_suggestions))
        .routes(routes!(list_tags))
        .routes(routes!(preview_tag_change))
        .routes(routes!(change_tag))
        .routes(routes!(combine_tags))
        .routes(routes!(list_labels, create_label))
        .routes(routes!(create_label_group))
        .routes(routes!(change_label_group))
        .routes(routes!(preview_label_change))
        .routes(routes!(change_label))
        .routes(routes!(combine_labels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_a_tag() {
        let tag = TagItem {
            id: Some(String::new()),
            name: Some(String::new()),
            color: Some(ChoiceColor::Cat1),
            whole_organization: Some(true),
            projects: Some(Vec::new()),
            version: Some(1),
        };
        crate::lean::assert_fields_are_the_keys_of(&TAG_FIELDS, &tag);
    }
}
