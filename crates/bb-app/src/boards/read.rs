//! The board read (PLAN-06, PRJ-06): the columns with their counts and card limits, the rows, and
//! the first cards of each cell, in the order of the search.
//!
//! Everything is read through the compiled search with the reader's rights, so a count holds only
//! what the reader sees (PRIV-13) and a hidden issue is in no column. The search is prepared once.
//! The counts are the totals of the search (`Ctx::counted`, the one way a search is added up):
//! grouped by the field of the columns, and on a board with rows grouped by the rows and then by
//! the field of the columns, a statement each. The first cards of every cell are one more statement
//! (`SearchStore::search_cell_pages`), and what the cards show is read once for all of them
//! (PERF-N1). Nothing grows with the number of rows, columns or cards.

use std::collections::HashMap;

use bb_core::ErrorCode;
use bb_core::fields::FieldKey;
use bb_core::id::{IssueId, IssueTypeId, ProjectId};
use bb_core::issue::{Issue, Priority, StatusGroup};
use bb_core::readable_id::ReadableId;
use bb_core::saved_search::BoardOptions;
use bb_policy::{Authorized, Place as Where};

use super::cells::{
    CellCount, CellFound, RowPlace, RowsRead, cells_found, column_counts, column_heads, grouped_by,
    rows_and_counts,
};
use super::columns::{Column, ColumnsBy, board_columns, column_of, columns_by, columns_word};
use super::moves::card_moves;
use super::types::{BoardCard, BoardCell, BoardPage, BoardQuery, CARDS_MAX};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::issues::{ReadOnRequest, WithAgent};
use crate::plan::timeline_groups_by;
use crate::search::{
    CellAt, CellWindow, Cells, FieldTarget, GroupKey, KeyPair, Looked, Prepared, SearchStore,
    ShownKey, key_place, stopped, target_of,
};

impl<S: AuthStore + SearchStore> Ctx<'_, S> {
    /// The issue types of the issues the search finds, when the reader may search by the type:
    /// they choose the workflows the columns of statuses come from.
    async fn types_found(&self, prepared: &Prepared) -> Result<Vec<IssueTypeId>, AppError> {
        let Some(word) = prepared
            .looked
            .fields
            .iter()
            .find(|each| each.target == FieldTarget::Type)
            .map(|each| each.field.word.clone())
        else {
            return Ok(Vec::new());
        };
        Ok(self
            .totals_in(prepared, &grouped_by(&word))
            .await?
            .groups
            .into_iter()
            .filter_map(|group| match group.key {
                GroupKey::IssueType(id) => Some(id),
                _ => None,
            })
            .collect())
    }

    /// The rows of a board and the counts of its cells: the totals of the search grouped by the
    /// field of the rows (a field the timeline groups by, not the field of the columns) and then by
    /// the field of the columns, each row shown as the reader may see it (people shown alike one
    /// row), in the order the board shows them ([`super::cells`]).
    async fn board_rows(
        &self,
        actor: &Actor,
        base: &mut Prepared,
        (rows, columns): (&str, &[Column]),
        columns_word: &str,
    ) -> Result<(RowsRead, Vec<CellCount>), AppError> {
        let grouping = self.grouping_of(actor, base, rows, None).await?;
        let wrong = || AppError::Refused(ErrorCode::InvalidRequest);
        if !timeline_groups_by(grouping.target)
            || target_of(&base.looked.fields, columns_word) == Some(grouping.target)
        {
            return Err(wrong());
        }
        let shown = self.cell_totals(base, &grouping, columns_word).await?;
        let goals = self.goal_names(&base.looked, &shown.keys).await?;
        let looked = &base.looked;
        let place = |key: &GroupKey, row: &ShownKey| row_place(looked, &goals, key, row);
        let groups = shown.counted.groups.into_iter().zip(shown.keys).collect();
        Ok(rows_and_counts(groups, (columns, Some(shown.by)), place))
    }

    /// The Issue IDs of the goals among the rows (rows by *Goal*), read at once, to put them in
    /// order by what the reader sees.
    async fn goal_names(
        &self,
        looked: &Looked,
        keys: &[KeyPair],
    ) -> Result<Vec<(IssueId, String)>, AppError> {
        let ids: Vec<IssueId> = keys
            .iter()
            .filter_map(|(key, _)| match key {
                GroupKey::Issue(id) => Some(*id),
                _ => None,
            })
            .collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self
            .store()
            .issues_by_ids(&ids)
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .filter_map(|issue| {
                let project = looked.seen.stored(issue.project)?;
                let readable = ReadableId::new(project.short_name.clone(), issue.code.clone());
                Some((issue.id, readable.to_string()))
            })
            .collect())
    }

    /// The cards of the cells: each issue with what the reader may see of it, the fields of its
    /// card and where it may go. What they show is read once for every cell.
    async fn cards_of(
        &self,
        actor: &Actor,
        base: &Prepared,
        (by, columns): (ColumnsBy, &[Column]),
        found: Vec<CellFound>,
    ) -> Result<Vec<BoardCell>, AppError> {
        let issues: Vec<Issue> = found
            .iter()
            .flat_map(|cell| cell.issues.iter().cloned())
            .collect();
        let mut items = self
            .page_items(
                actor,
                &base.looked.seen,
                &issues,
                ReadOnRequest {
                    people: true,
                    labels: true,
                    agent: WithAgent::No,
                    time: true,
                },
            )
            .await?
            .into_iter();
        let chains: HashMap<ProjectId, Vec<ProjectId>> = base
            .looked
            .seen
            .seen_places(None)
            .into_iter()
            .filter_map(|(chain, _)| Some((*chain.last()?, chain)))
            .collect();
        let mut fields: HashMap<(ProjectId, IssueTypeId), Vec<FieldKey>> = HashMap::new();
        let mut cells = Vec::with_capacity(found.len());
        for cell in found {
            let mut cards = Vec::with_capacity(cell.issues.len());
            for issue in &cell.issues {
                let (held, view) = items
                    .next()
                    .ok_or_else(|| AppError::Internal("a card without its issue".into()))?;
                let chain = chains.get(&issue.project).cloned().unwrap_or_default();
                let place = (issue.project, issue.issue_type);
                if let std::collections::hash_map::Entry::Vacant(vacant) = fields.entry(place) {
                    let project =
                        base.looked.seen.stored(issue.project).ok_or_else(|| {
                            AppError::Internal("a card of an unseen project".into())
                        })?;
                    vacant.insert(
                        self.card_fields((project, &chain), issue.issue_type, &held)
                            .await?,
                    );
                }
                let workflow = self
                    .workflow_in_force(&chain, Some(issue.issue_type))
                    .await?;
                let changeable = match by {
                    ColumnsBy::Choice(_) => self.changeable_fields(&held, issue).await?,
                    ColumnsBy::Status | ColumnsBy::StatusGroup => Vec::new(),
                };
                let moves = card_moves(
                    (by, columns),
                    (&held, &view),
                    &workflow,
                    &changeable,
                    (cell.column, issue.issue_type),
                );
                cards.push(BoardCard {
                    issue: held.authorize(view),
                    fields: fields.get(&place).cloned().unwrap_or_default(),
                    moves,
                });
            }
            cells.push(BoardCell {
                column: cell.column,
                row: cell.row,
                count: cell.count,
                cards,
                next: cell.next,
            });
        }
        Ok(cells)
    }
}

/// The place of a row in the order the board shows (see [`super::cells`]): a person by the name
/// the reader sees, a project by its short name, a goal by its Issue ID, a priority, a status
/// group, a status or a choice by the order of its field; never by an internal ID.
fn row_place(
    looked: &Looked,
    goals: &[(IssueId, String)],
    key: &GroupKey,
    shown: &ShownKey,
) -> RowPlace {
    let place = |at: Option<usize>| u64::try_from(at.unwrap_or(usize::MAX)).unwrap_or(u64::MAX);
    match key {
        GroupKey::Priority(each) => (
            place(Priority::LIST_ORDER.iter().position(|one| one == each)),
            String::new(),
        ),
        GroupKey::StatusGroup(each) => (
            place(StatusGroup::ALL.iter().position(|one| one == each)),
            String::new(),
        ),
        GroupKey::Text(status) => (
            place(
                looked
                    .workflows
                    .iter()
                    .flat_map(|(_, workflow)| &workflow.statuses)
                    .position(|each| each.key.as_str() == status),
            ),
            String::new(),
        ),
        GroupKey::Choice(id) => (
            place(
                looked
                    .own
                    .iter()
                    .flat_map(|definition| &definition.choices)
                    .position(|choice| choice.id == *id),
            ),
            String::new(),
        ),
        GroupKey::Project(id) => (
            0,
            looked
                .seen
                .stored(*id)
                .map(|project| project.short_name.to_string())
                .unwrap_or_default(),
        ),
        GroupKey::Issue(id) => (
            0,
            goals
                .iter()
                .find(|(goal, _)| goal == id)
                .map(|(_, name)| name.clone())
                .unwrap_or_default(),
        ),
        _ => (0, key_place(shown).text),
    }
}

/// A board: the columns of the search with their counts and limits, its rows, and the first cards
/// of each cell; or, with `more`, the next cards of one cell.
///
/// # Errors
/// The mistakes of the text; `resource.not-found` for a saved search or a project the person may
/// not open; `request.invalid` for options a board cannot have (a field of columns that is no
/// status, status group or choice field of one value the reader may read, a field of rows the
/// board does not offer), a number of cards outside 1 to [`CARDS_MAX`], or a cell that is no cell;
/// `search.too-slow` for a search that takes longer than the limit.
pub async fn board<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    query: BoardQuery,
) -> Result<Authorized<BoardPage>, AppError> {
    let invalid = || AppError::Refused(ErrorCode::InvalidRequest);
    let actor = ctx.actor(&query.session_token).await?;
    if !(1..=CARDS_MAX).contains(&query.limit) {
        return Err(invalid());
    }
    let view = ctx
        .prepare_view(
            &actor,
            &query.search,
            (query.project.as_deref(), query.only_this_project),
        )
        .await?;
    // A saved board shows the options it keeps unless the request names others.
    let options: BoardOptions = query
        .options
        .clone()
        .or_else(|| view.saved.as_ref().map(|saved| saved.board.clone()))
        .unwrap_or_default();
    if !options.is_valid() {
        return Err(invalid());
    }
    let mut base = view.prepared;
    let by = columns_by(&base.looked, &options).ok_or_else(invalid)?;
    let word = columns_word(&options);
    let found_types = match by {
        ColumnsBy::Status => ctx.types_found(&base).await?,
        ColumnsBy::StatusGroup | ColumnsBy::Choice(_) => Vec::new(),
    };
    let columns = board_columns(&base.looked, by, &found_types)?;
    let totals = ctx.totals_in(&base, &grouped_by(&word)).await?;
    let heads = column_heads(&columns, &totals, &options.limits);
    let (rows, counts) = match options.rows.as_deref() {
        None => (RowsRead::none(), column_counts(&heads)),
        Some(rows) => {
            ctx.board_rows(&actor, &mut base, (rows, &columns), &word)
                .await?
        }
    };
    let only = match query.more {
        None => None,
        Some(ask) => Some(CellAt {
            column: ask.column,
            row: match ask.row {
                None => None,
                Some(at) => Some(rows.keys.get(at).cloned().ok_or_else(invalid)?),
            },
        }),
    };
    let hits = ctx
        .store()
        .search_cell_pages(
            &base.request,
            &Cells {
                columns: word,
                column_of: column_of(&columns),
                rows: rows.by.clone(),
            },
            &CellWindow {
                skip: query.more.map_or(0, |ask| ask.skip),
                take: u64::from(query.limit),
                only,
                rows: rows.keys.clone(),
            },
        )
        .await
        .map_err(stopped)?;
    let found = cells_found(counts, hits, &rows, query.more);
    let cells = ctx.cards_of(&actor, &base, (by, &columns), found).await?;
    let organization = ctx.evaluation_of(&actor, Where::Organization).await?;
    Ok(organization.authorize(BoardPage {
        search_text: view.text,
        columns: heads,
        rows: rows.shown,
        rows_truncated: rows.truncated,
        cells,
    }))
}
