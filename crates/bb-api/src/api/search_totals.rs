//! The route of the totals of a search (QRY-08, PRIV-13, ADR-0050 decision 7):
//! `POST /api/v1/search/totals`. It answers the count and the sums asked for, for each group of one
//! field and for all the issues together, only over the issues and the values the person may see.
//! It counts against the limit of searches like the other search routes, and its cursor is sealed
//! for the person and the totals it was made for. The use case is `bb_app::search::search_totals`.

use std::sync::Arc;

use axum::Extension;
use axum::extract::State;
use axum::response::Response;
use bb_app::search::{
    GroupOrder, SearchScope, SearchTotalsQuery, SumShown, TotalShown, totals_list,
};
use bb_core::id::ProjectId;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use super::auth::{AuthHandle, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session};
use super::search::{SearchGroupKeyBody, counted, group_key};
use crate::errors::{ApiError, ApiJson};
use crate::limits::Limits;
use crate::paging;

/// How the groups are ordered.
#[derive(Clone, Copy, Default, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TotalsOrderBody {
    /// By their keys, as *Group by* orders them; people by their names.
    #[default]
    Key,
    /// The biggest first. Refused for a field of people: no total ranks people.
    Count,
}

/// Ask for the totals of a search.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchTotalsBody {
    /// The search, as text of the search language; left out or empty, every issue the person may
    /// read.
    #[serde(default)]
    q: Option<String>,
    /// The ID of a project: the search looks in it and the projects inside it that the person sees.
    #[serde(default)]
    within: Option<String>,
    /// The word of the field to group by (any field *Group by* takes); left out, only the total of
    /// all the issues.
    #[serde(default)]
    by: Option<String>,
    /// The sums wanted, in the order the answer gives them: `count`, `estimated`, `time_left`,
    /// `time_spent` and `number:<field>` for an own field of numbers (at most eight besides
    /// `count`). A field the person may read nowhere is refused like one that does not exist.
    #[serde(default)]
    sums: Vec<String>,
    /// How the groups are ordered.
    #[serde(default)]
    order: TotalsOrderBody,
    /// Where the next page of groups starts (`next_cursor` of the answer before).
    #[serde(default)]
    cursor: Option<String>,
}

/// One sum of a total.
#[derive(Serialize, ToSchema)]
pub(crate) struct SearchSumBody {
    /// The sum: whole minutes for a length and the count for `count`; for an own field a value in
    /// the form of the field (a decimal as text). Null when no issue gave a value the person may
    /// read.
    #[schema(required = true)]
    value: Option<Value>,
    /// How many issues gave a value the person may read ("12 h over 9 of 14 issues").
    valued: u64,
}

impl From<SumShown> for SearchSumBody {
    fn from(sum: SumShown) -> Self {
        Self {
            value: sum.value,
            valued: sum.valued,
        }
    }
}

/// The total of all the issues the search finds.
#[derive(Serialize, ToSchema)]
pub(crate) struct SearchTotalBody {
    /// How many issues the person may read the search finds; an issue in several groups counts
    /// once.
    issues: u64,
    /// The sums, in the order of `sums` of the answer.
    sums: Vec<SearchSumBody>,
}

impl From<TotalShown> for SearchTotalBody {
    fn from(total: TotalShown) -> Self {
        Self {
            issues: total.issues,
            sums: total.sums.into_iter().map(SearchSumBody::from).collect(),
        }
    }
}

/// The total of one group.
#[derive(Serialize, ToSchema)]
pub(crate) struct SearchTotalRowBody {
    /// What the group is; a person as the person asking may see them.
    key: SearchGroupKeyBody,
    /// How many issues the search finds in it.
    issues: u64,
    /// The filter that selects the issues of the group, as search text; null when the group has
    /// no name the person may use.
    #[schema(required = true)]
    filter: Option<String>,
    /// The name the group is shown by; null when it has none the person may use.
    #[schema(required = true)]
    name: Option<String>,
    /// The sums, in the order of `sums` of the answer.
    sums: Vec<SearchSumBody>,
}

/// The totals of a search.
#[derive(Serialize, ToSchema)]
pub(crate) struct SearchTotalsAnswer {
    /// The words of the sums, in the order of the sums of every row.
    sums: Vec<String>,
    /// A page of the groups, in the order asked for; empty without `by`.
    groups: Vec<SearchTotalRowBody>,
    /// All the issues together.
    all: SearchTotalBody,
    /// The cursor of the next page of groups; null on the last page.
    #[schema(required = true)]
    next_cursor: Option<String>,
}

impl Masked for SearchTotalsAnswer {}

/// The totals of a search: the count and the sums asked for, of each group of one field and of all
/// the issues together. A total counts only the issues the person may read and a value only where
/// they may read its field; people stand in the order of their names, never of their size.
#[utoipa::path(
    post,
    path = "/api/v1/search/totals",
    operation_id = "totalSearchedIssues",
    tag = "search",
    request_body = SearchTotalsBody,
    responses(
        (status = 200, description = "The totals.", body = SearchTotalsAnswer),
        InvalidAnswer, NotSignedInAnswer, NotFoundAnswer
    )
)]
pub(super) async fn total_searched_issues(
    State(auth): State<AuthHandle>,
    Extension(limits): Extension<Arc<Limits>>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<SearchTotalsBody>,
) -> Result<Response, ApiError> {
    let person = counted(&auth, &limits, &info, &session_token).await?;
    let scope = match body.within.as_deref() {
        None => SearchScope::Everywhere,
        Some(text) => SearchScope::Within(
            text.parse::<ProjectId>()
                .map_err(|_| ApiError::new(bb_core::ErrorCode::InvalidRequest))?,
        ),
    };
    let mut query = SearchTotalsQuery {
        session_token,
        text: body.q.unwrap_or_default(),
        scope,
        by: body.by,
        sums: body.sums,
        order: match body.order {
            TotalsOrderBody::Key => GroupOrder::Key,
            TotalsOrderBody::Count => GroupOrder::Count,
        },
        cursor: None,
    };
    let list = totals_list(person, &query);
    query.cursor = body
        .cursor
        .as_deref()
        .map(|cursor| paging::open(&list, cursor))
        .transpose()?;
    let found = auth.api()?.search_totals(info, query).await?;
    let mut failed = None;
    let answer = found.map(|totals| {
        let next_cursor = totals
            .next_cursor
            .map(|position| paging::seal(&list, &position))
            .transpose()
            .unwrap_or_else(|error| {
                failed = Some(error);
                None
            });
        SearchTotalsAnswer {
            sums: totals.sums,
            groups: totals
                .groups
                .into_iter()
                .map(|row| SearchTotalRowBody {
                    key: group_key(row.group.key),
                    issues: row.group.issues,
                    filter: row.group.filter,
                    name: row.group.name,
                    sums: row.sums.into_iter().map(SearchSumBody::from).collect(),
                })
                .collect(),
            all: totals.all.into(),
            next_cursor,
        }
    });
    if let Some(error) = failed {
        return Err(error);
    }
    super::auth::answer_authorized(&answer)
}
