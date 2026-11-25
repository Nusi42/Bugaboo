//! The routes of dashboards (CFG-03, ADR-0050): `/api/v1/dashboards` for the person's own and the
//! ones shared with them, one dashboard with the boxes that check for the person, each box's
//! answer, *Home*, the ready-made boxes and the preview of a box.
//!
//! A dashboard is the person's own: only a session reaches it, never an access key (ADR-0050,
//! decision 10). It is private when it is made; only its maker changes, shares or deletes it. A
//! dashboard that is not for the person is not found, as one that does not exist. The use cases
//! are in `bb-app`, behind [`AuthApi`](bb_app::auth::AuthApi).

use std::sync::Arc;

use bb_app::dashboards::{BoxChange, BoxView, DashboardItem, DashboardView, RestoredDashboard};
use bb_core::ErrorCode;
use bb_core::dashboard::{
    BoxKind, BoxOptions, BoxPlace, BoxSource, BoxSpec, ChartShape, ReadyMade, TimeSpan,
};
use bb_core::id::{DashboardBoxId, DashboardId, SavedSearchId};
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;

use super::auth::AuthHandle;
use super::saved_searches::ShareRef;
use crate::errors::ApiError;
use crate::lean::FieldSpec;
use crate::limits::Limits;

mod answers;
mod handlers;

const DASHBOARDS_LIST: &str = "dashboards";

/// What a dashboard in a list holds, by name.
const DASHBOARD_FIELDS: [FieldSpec; 6] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("maker"),
    FieldSpec::by_default("mine"),
    FieldSpec::by_default("updated_at"),
    FieldSpec::by_default("version"),
];

/// A box as it is asked for: its kind, its place on the grid of 12 columns, its title, what it
/// reads and the options of its kind. There is no option that sorts by size or ranks people.
#[derive(Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct BoxBody {
    /// The box it was, when a change keeps it; left out for a new box.
    #[serde(default)]
    id: Option<String>,
    /// The kind.
    kind: BoxKind,
    /// The first column, from 0.
    column: u32,
    /// The first row, from 0; the places are packed upwards.
    row: u32,
    /// The width in columns: at least 3.
    width: u32,
    /// The height in rows: at least 2.
    height: u32,
    /// The title.
    title: String,
    /// The search text it reads; not with `saved_search`.
    #[serde(default)]
    search_text: Option<String>,
    /// The ID of the saved search it reads; not with `search_text`.
    #[serde(default)]
    saved_search: Option<String>,
    /// For *Numbers and chart* as bars: the word of the field the totals are grouped by.
    #[serde(default)]
    by: Option<String>,
    /// For *Numbers and chart*: bars or one number.
    #[serde(default)]
    shape: Option<ChartShape>,
    /// For *Time totals*: the time of the search or the person's own week.
    #[serde(default)]
    span: Option<TimeSpan>,
}

impl BoxBody {
    /// The box it was, and the box as it is asked for; `request.invalid` for an ID that is none or
    /// a box that names both a text and a saved search.
    fn read(self) -> Result<BoxChange, ApiError> {
        let invalid = || ApiError::new(ErrorCode::InvalidRequest);
        let id = self
            .id
            .map(|id| id.parse::<DashboardBoxId>().map_err(|_| invalid()))
            .transpose()?;
        let source = match (self.search_text, self.saved_search) {
            (Some(text), None) => Some(BoxSource::Text(text)),
            (None, Some(saved)) => Some(BoxSource::Saved(
                saved.parse::<SavedSearchId>().map_err(|_| invalid())?,
            )),
            (None, None) => None,
            (Some(_), Some(_)) => return Err(invalid()),
        };
        Ok(BoxChange {
            id,
            spec: BoxSpec {
                kind: self.kind,
                place: BoxPlace {
                    column: self.column,
                    row: self.row,
                    width: self.width,
                    height: self.height,
                },
                title: self.title,
                source,
                options: BoxOptions {
                    by: self.by,
                    shape: self.shape,
                    span: self.span,
                },
            },
        })
    }

    /// The boxes of a body, each read.
    fn read_all(boxes: Vec<Self>) -> Result<Vec<BoxChange>, ApiError> {
        boxes.into_iter().map(Self::read).collect()
    }
}

/// A box as the person sees it: its place packed for them.
#[derive(Serialize, ToSchema)]
pub(crate) struct BoxItem {
    /// The ID of the box.
    id: String,
    /// The kind.
    kind: BoxKind,
    /// The first column, from 0.
    column: u32,
    /// The first row, from 0.
    row: u32,
    /// The width in columns.
    width: u32,
    /// The height in rows.
    height: u32,
    /// The title.
    title: String,
    /// The search text it reads, in the spelling the person's language reads it; null for a box
    /// that reads a saved search or no search.
    #[schema(required = true)]
    search_text: Option<String>,
    /// The ID of the saved search it reads; null otherwise.
    #[schema(required = true)]
    saved_search: Option<String>,
    /// The word of the field the totals are grouped by; null for none.
    #[schema(required = true)]
    by: Option<String>,
    /// Bars or one number; null for a kind that is no chart.
    #[schema(required = true)]
    shape: Option<ChartShape>,
    /// Whose time is added up; null for a kind that adds up no time.
    #[schema(required = true)]
    span: Option<TimeSpan>,
}

impl BoxItem {
    fn of(id: Option<DashboardBoxId>, spec: &BoxSpec) -> Self {
        let (search_text, saved_search) = match &spec.source {
            Some(BoxSource::Text(text)) => (Some(text.clone()), None),
            Some(BoxSource::Saved(saved)) => (None, Some(saved.to_string())),
            None => (None, None),
        };
        Self {
            id: id.map(|id| id.to_string()).unwrap_or_default(),
            kind: spec.kind,
            column: spec.place.column,
            row: spec.place.row,
            width: spec.place.width,
            height: spec.place.height,
            title: spec.title.clone(),
            search_text,
            saved_search,
            by: spec.options.by.clone(),
            shape: spec.options.shape,
            span: spec.options.span,
        }
    }
}

impl From<&BoxView> for BoxItem {
    fn from(view: &BoxView) -> Self {
        Self::of(Some(view.id), &view.spec)
    }
}

/// A dashboard as the person sees it, with the boxes that check for them.
#[derive(Serialize, ToSchema)]
pub(crate) struct DashboardBody {
    /// The ID, in the address `/d/<id>`.
    id: String,
    /// What it is called.
    name: String,
    /// Who made it, as the person sees people who write.
    maker: PersonRef,
    /// Whether the person made it.
    mine: bool,
    /// Whether it is the person's *Home*.
    home: bool,
    /// Whom it is shared with; only its maker is told.
    shared_with: Vec<ShareRef>,
    /// The boxes the person sees, in order, packed upwards: no gap tells of a box left out.
    boxes: Vec<BoxItem>,
    /// When it was last changed.
    updated_at: String,
    /// Counts up with every change; a change is made on the version that was looked at.
    version: i64,
}

impl Masked for DashboardBody {}

impl From<&DashboardView> for DashboardBody {
    fn from(view: &DashboardView) -> Self {
        Self {
            id: view.id.to_string(),
            name: view.name.clone(),
            maker: view.maker.clone(),
            mine: view.mine,
            home: view.home,
            shared_with: view.shared_with.iter().map(ShareRef::from).collect(),
            boxes: view.boxes.iter().map(BoxItem::from).collect(),
            updated_at: view.updated_at.to_string(),
            version: view.version,
        }
    }
}

/// A dashboard in a list.
#[derive(Serialize, ToSchema)]
pub(crate) struct DashboardSummary {
    /// The ID.
    id: String,
    /// What it is called.
    name: String,
    /// Who made it, as the person sees people who write.
    maker: PersonRef,
    /// Whether the person made it.
    mine: bool,
    /// When it was last changed.
    updated_at: String,
    /// Counts up with every change.
    version: i64,
}

impl Masked for DashboardSummary {}

impl From<&DashboardItem> for DashboardSummary {
    fn from(item: &DashboardItem) -> Self {
        Self {
            id: item.id.to_string(),
            name: item.name.clone(),
            maker: item.maker.clone(),
            mine: item.mine,
            updated_at: item.updated_at.to_string(),
            version: item.version,
        }
    }
}

/// What taking a dashboard out of the trash answers.
#[derive(Serialize, ToSchema)]
pub(crate) struct OptionalDashboardBody {
    /// The dashboard, when it is one the person may open.
    #[schema(required = true)]
    dashboard: Option<DashboardBody>,
}

impl Masked for OptionalDashboardBody {}

impl From<&RestoredDashboard> for OptionalDashboardBody {
    fn from(restored: &RestoredDashboard) -> Self {
        Self {
            dashboard: restored.dashboard.as_ref().map(DashboardBody::from),
        }
    }
}

/// A ready-made box: which one, and the box it adds.
#[derive(Serialize, ToSchema)]
pub(crate) struct ReadyMadeItem {
    /// Which one; its title is the message `ready-made-box.<which>`.
    which: ReadyMade,
    /// The box it adds, at the top left, with its title in the words of the catalog.
    #[serde(rename = "box")]
    ready_box: BoxItem,
}

/// The ready-made boxes, in the order *Add box* offers them.
#[derive(Serialize, ToSchema)]
pub(crate) struct ReadyMadeBody {
    /// The boxes, in the order of the one list of the code.
    boxes: Vec<ReadyMadeItem>,
}

impl Masked for ReadyMadeBody {}

/// Create a dashboard.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewDashboardBody {
    /// What it is called.
    name: String,
    /// Its boxes, in order: at most 20.
    #[serde(default)]
    boxes: Vec<BoxBody>,
}

/// Change a dashboard: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChangeDashboardBody {
    /// The version the person looked at.
    version: i64,
    /// A new name.
    #[serde(default)]
    name: Option<String>,
    /// All the boxes, in order: a box that is not named any more goes; a box named by its `id`
    /// keeps it.
    #[serde(default)]
    boxes: Option<Vec<BoxBody>>,
}

fn dashboard_id(text: &str) -> Result<DashboardId, ApiError> {
    super::path_id::<DashboardId>(text)
}

/// The routes of dashboards.
pub(crate) fn routes<S>(limits: &Arc<Limits>) -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .merge(handlers::routes())
        .merge(answers::routes(limits))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_a_listed_dashboard() {
        let item = DashboardSummary {
            id: String::new(),
            name: String::new(),
            maker: PersonRef::Someone,
            mine: true,
            updated_at: String::new(),
            version: 1,
        };
        crate::lean::assert_fields_are_the_keys_of(&DASHBOARD_FIELDS, &item);
    }
}
