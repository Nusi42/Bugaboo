//! The walk of the relation map on both databases (ISS-16, UX design §4.15): one statement for each
//! step away, built on the one step of every walk (`search::filter::step`), so the map passes only
//! through the issues the search would read for the same person.
//!
//! A step is a statement of its own instead of one recursive table: a recursive walk that goes both
//! ways along every relation touches every issue again at each step before it could stop at the cap
//! of the map (architecture §7, *The walk of the relation map*). A step starts from at most the cap
//! of issues and stops inside the statement at the number it may still add, so the work of a map is
//! bounded by its cap and not by how many issues are connected or hidden. The whole walk has the
//! time of one search.

use bb_app::search::{MapStep, SearchFailure, SearchRequest};
use bb_core::id::IssueId;
use sea_query::{Alias, Expr, ExprTrait as _, Iden, Order, Query, SelectStatement, UnionType};

use crate::auth::AuthStorage;
use crate::db::{Statement, StoredId as _};
use crate::search::filter::{Line, step};
use crate::search::seen_by;

/// The column of the far end of a step.
#[derive(Clone, Copy)]
struct Far;

impl Iden for Far {
    fn unquoted(&self) -> &'static str {
        "far"
    }
}

/// The far ends of one kind of line from the near issues of `map`.
fn far_ends(request: &SearchRequest, map: &MapStep, line: &Line<'_>) -> SelectStatement {
    let near: Vec<Vec<u8>> = map.near.iter().map(|issue| issue.stored()).collect();
    let is_near = move |end: Expr| end.is_in(near.clone());
    let mut select = Query::select();
    let far = step(&mut select, line, &is_near, seen_by(request));
    select.expr_as(far, Far);
    select
}

/// The statement of one step: the far ends of the lines chosen, not known yet, by ID, at most
/// `most`. `None` when nothing is chosen.
pub(crate) fn step_statement(request: &SearchRequest, map: &MapStep) -> Option<SelectStatement> {
    let relations =
        (!map.kinds.is_empty()).then(|| far_ends(request, map, &Line::Relations(Some(&map.kinds))));
    let parents = map.part_of.then(|| far_ends(request, map, &Line::Parents));
    let mut lines = match (relations, parents) {
        (Some(mut relations), Some(parents)) => {
            relations.union(UnionType::All, parents);
            relations
        }
        (Some(only), None) | (None, Some(only)) => only,
        (None, None) => return None,
    };
    let known: Vec<Vec<u8>> = map.known.iter().map(|issue| issue.stored()).collect();
    let steps = Alias::new("steps");
    let mut select = Query::select();
    select
        .distinct()
        .column((steps.clone(), Far))
        .from_subquery(lines.take(), steps.clone())
        .and_where(Expr::col((steps.clone(), Far)).is_not_in(known))
        .order_by((steps, Far), Order::Asc)
        .limit(map.most);
    Some(select)
}

impl AuthStorage {
    /// One step away on the map (`SearchStore::map_step`).
    pub(crate) async fn map_step_in(
        &self,
        request: &SearchRequest,
        map: &MapStep,
    ) -> Result<Vec<IssueId>, SearchFailure> {
        if map.near.is_empty() || map.most == 0 {
            return Ok(Vec::new());
        }
        let Some(select) = step_statement(request, map) else {
            return Ok(Vec::new());
        };
        // One deadline for the whole walk: the step gets the time of a search that is left.
        let left = crate::search::SEARCH_TIME.saturating_sub(map.began.elapsed());
        if left.is_zero() {
            return Err(SearchFailure::TooSlow);
        }
        let rows = self
            .database()
            .fetch_all_within(Statement::Select(&select), left)
            .await
            .map_err(crate::search::stopped)?;
        rows.iter()
            .map(|row| row.id::<IssueId>(Far))
            .collect::<Result<_, _>>()
            .map_err(crate::search::failed)
    }
}
