//! The saved-search routes (QRY-06, CFG-02, ADR-0034): `/api/v1/saved-searches` for the person's
//! own and the ones shared with them, and `/api/v1/projects/{project}/list` for the list a project
//! shows.
//!
//! A saved search is the person's own: only a session reaches it, never an access key. It is
//! private when it is made; only its maker changes, shares or deletes it. A search that is not
//! for the person is not found, as one that does not exist (ADR-0034, decision 4). The use cases
//! are in `bb-app`, behind [`AuthApi`](bb_app::auth::AuthApi).

use bb_app::access::ProjectLabel;
use bb_app::saved_searches::{
    ChangeSearch, DeleteSearch, ProjectList, ProjectListQuery, RestoreSearch, SaveCopy, SaveSearch,
    SavedSearchView, SearchQuery, SearchesQuery, SetProjectList, ShareChoice, ShareChoicesQuery,
    ShareSearch, ShareView, StopSharingSearch,
};
use bb_core::ErrorCode;
use bb_core::id::SavedSearchId;
use bb_core::saved_search::{
    Column, Presentation, SearchSet, ShareKind, ShareTarget, Spacing, ViewKind,
};
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;

use super::auth::AuthHandle;
use crate::errors::ApiError;
use crate::lean::FieldSpec;

mod project_list;
mod searches;

const SEARCHES_LIST: &str = "saved-searches";
const CHOICES_LIST: &str = "share-choices";

/// What a saved search holds, by name; the text, how it is shown and whom it is shared with only
/// when asked for.
const SEARCH_FIELDS: [FieldSpec; 9] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("maker"),
    FieldSpec::by_default("mine"),
    FieldSpec::by_default("updated_at"),
    FieldSpec::by_default("version"),
    FieldSpec::on_request("search_text"),
    FieldSpec::on_request("presentation"),
    FieldSpec::on_request("shared_with"),
];
const CHOICE_FIELDS: [FieldSpec; 3] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("short_name"),
];

/// One column of a list: the field by its search word, and its width if the person set one.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub(crate) struct ColumnBody {
    /// The search word of the field (`assignedTo`).
    field: String,
    /// How wide the column is, in the units of the list; `null` for the width the list chooses.
    #[schema(required = true)]
    width: Option<u32>,
}

/// How the results of a saved search are shown.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub(crate) struct PresentationRef {
    /// The kind of view. M5 shows the kind `list`.
    view: ViewKind,
    /// The columns, in order. None at all means the columns of the project's list.
    columns: Vec<ColumnBody>,
    /// How close together the rows are: a choice of the setting `appearance.spacing`.
    spacing: String,
    /// The field the rows are grouped by, by its search word.
    #[schema(required = true)]
    group_by: Option<String>,
}

impl From<&Presentation> for PresentationRef {
    fn from(presentation: &Presentation) -> Self {
        Self {
            view: presentation.view,
            columns: presentation
                .columns
                .iter()
                .map(|column| ColumnBody {
                    field: column.field.clone(),
                    width: column.width,
                })
                .collect(),
            spacing: presentation.spacing.code().to_owned(),
            group_by: presentation.group_by.clone(),
        }
    }
}

impl TryFrom<PresentationRef> for Presentation {
    type Error = ApiError;

    fn try_from(body: PresentationRef) -> Result<Self, ApiError> {
        Ok(Self {
            view: body.view,
            columns: body
                .columns
                .into_iter()
                .map(|column| Column {
                    field: column.field,
                    width: column.width,
                })
                .collect(),
            spacing: Spacing::of(&body.spacing)
                .ok_or_else(|| ApiError::new(ErrorCode::InvalidRequest))?,
            group_by: body.group_by,
        })
    }
}

/// A group or a project a saved search is shared with.
#[derive(Serialize, ToSchema)]
pub(crate) struct ShareRef {
    /// A group or a project.
    kind: ShareKind,
    /// The ID of the group or the project.
    id: String,
    /// Its name; left out for a project the person may not know.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// The short name of a project the person may know.
    #[serde(skip_serializing_if = "Option::is_none")]
    short_name: Option<String>,
}

impl From<&ShareView> for ShareRef {
    fn from(share: &ShareView) -> Self {
        match share {
            ShareView::Group { id, name } => Self {
                kind: ShareKind::Group,
                id: id.to_string(),
                name: Some(name.clone()),
                short_name: None,
            },
            ShareView::Project { id, label } => {
                let (name, short_name) = match label {
                    ProjectLabel::Visible {
                        name, short_name, ..
                    } => (Some(name.clone()), Some(short_name.clone())),
                    ProjectLabel::Hidden => (None, None),
                };
                Self {
                    kind: ShareKind::Project,
                    id: id.to_string(),
                    name,
                    short_name,
                }
            }
        }
    }
}

/// A saved search as the person sees it.
#[derive(Serialize, ToSchema)]
pub(crate) struct SavedSearchItem {
    /// The ID, in the address `/s/<id>`.
    id: String,
    /// What it is called.
    name: String,
    /// The search text, in the canonical spelling of the search language.
    search_text: String,
    /// How its results are shown.
    presentation: PresentationRef,
    /// Who made it, as the person sees people who write.
    maker: PersonRef,
    /// Whether the person made it.
    mine: bool,
    /// Whom it is shared with; only its maker is told.
    shared_with: Vec<ShareRef>,
    /// When it was last changed.
    updated_at: String,
    /// Counts up with every change. A change is made on the version that was looked at.
    version: i64,
}

impl Masked for SavedSearchItem {}

impl From<&SavedSearchView> for SavedSearchItem {
    fn from(view: &SavedSearchView) -> Self {
        Self {
            id: view.id.to_string(),
            name: view.name.clone(),
            search_text: view.text.clone(),
            presentation: PresentationRef::from(&view.presentation),
            maker: view.maker.clone(),
            mine: view.mine,
            shared_with: view.shared_with.iter().map(ShareRef::from).collect(),
            updated_at: view.updated_at.to_string(),
            version: view.version,
        }
    }
}

/// A group or a project the person may share with.
#[derive(Serialize, ToSchema)]
pub(crate) struct ShareChoiceItem {
    /// The ID of the group or the project.
    id: String,
    /// Its name.
    name: String,
    /// The short name of a project.
    #[serde(skip_serializing_if = "Option::is_none")]
    short_name: Option<String>,
}

impl Masked for ShareChoiceItem {}

impl From<&ShareChoice> for ShareChoiceItem {
    fn from(choice: &ShareChoice) -> Self {
        Self {
            id: choice.id.clone(),
            name: choice.name.clone(),
            short_name: choice.short_name.clone(),
        }
    }
}

/// The list a project shows.
#[derive(Serialize, ToSchema)]
pub(crate) struct ProjectListBody {
    /// The saved search the project chose as its list, when the person may open it; `null` for the
    /// built-in default list.
    #[schema(required = true)]
    search: Option<SavedSearchItem>,
}

impl Masked for ProjectListBody {}

impl From<&ProjectList> for ProjectListBody {
    fn from(list: &ProjectList) -> Self {
        Self {
            search: list.search.as_ref().map(SavedSearchItem::from),
        }
    }
}

/// A search to save.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewSavedSearchBody {
    /// What it is called.
    name: String,
    /// The search text.
    search_text: String,
    /// How its results are shown; a comfortable list with the columns of the project's list when
    /// left out.
    presentation: Option<PresentationRef>,
}

/// What to change in a saved search. What is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeSavedSearchBody {
    /// The version the person looked at; a change made meanwhile stops this one.
    version: i64,
    /// A new name.
    name: Option<String>,
    /// A new search text.
    search_text: Option<String>,
    /// A new way to show the results.
    presentation: Option<PresentationRef>,
}

/// The name of a copy.
#[derive(Deserialize, ToSchema)]
pub(crate) struct CopyBody {
    /// What the copy is called; the name of the original when left out.
    name: Option<String>,
}

/// The group or the project to share with.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ShareBody {
    /// A group or a project.
    kind: ShareKind,
    /// The ID of the group or the project.
    id: String,
}

/// The saved search a project shows as its list.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ProjectListChoiceBody {
    /// The saved search, shared with the project; `null` for the built-in default list.
    #[schema(required = true)]
    search: Option<String>,
}

/// Which saved searches to list.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ScopeParams {
    /// The ones the person made (`mine`, the default) or the ones shared with them (`shared`).
    scope: Option<SearchSet>,
}

/// Which places to list.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct KindParams {
    /// Groups or projects.
    kind: ShareKind,
}

/// The routes of saved searches and of the list of a project.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .merge(searches::routes())
        .merge(project_list::routes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_field_tables_name_exactly_the_fields_of_the_items() {
        let search = SavedSearchItem {
            id: String::new(),
            name: String::new(),
            search_text: String::new(),
            presentation: PresentationRef::from(&Presentation::default()),
            maker: PersonRef::Someone,
            mine: true,
            shared_with: Vec::new(),
            updated_at: String::new(),
            version: 1,
        };
        crate::lean::assert_fields_are_the_keys_of(&SEARCH_FIELDS, &search);
        let choice = ShareChoiceItem {
            id: String::new(),
            name: String::new(),
            short_name: Some(String::new()),
        };
        crate::lean::assert_fields_are_the_keys_of(&CHOICE_FIELDS, &choice);
    }
}
