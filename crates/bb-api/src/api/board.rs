//! The route of the board (PLAN-06, PRJ-06, ADR-0034, amendment of 2026-11-24): one route answers a
//! board of a saved search or a search text (`POST /api/v1/board`), worked out on the server for
//! the reader: the columns with their counts and card limits, the rows, the first cards of each
//! cell with the fields of the layout *Card* and where each card may go. A card is moved by the
//! status change or the field change of the issue that exists (`PATCH /api/v1/issues/{issue}`);
//! no rule of a workflow is in the browser.

use std::sync::Arc;

use axum::Extension;
use axum::extract::{FromRef, State};
use axum::response::Response;
use bb_app::boards::{
    BoardCard, BoardCell, BoardPage, BoardQuery, CARDS_MAX, CardMove, CellAsk, MoveRefused,
};
use bb_core::fields::FieldKey;
use bb_core::issue::StatusGroup;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa::openapi::schema::{Object, ObjectBuilder, Type};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session, answer_authorized,
};
use super::issues::IssueItem;
use super::saved_searches::BoardOptionsBody;
use super::search::{SearchGroupKeyBody, counted, group_key, view_search};
use crate::errors::{ApiError, ApiJson};
use crate::limits::Limits;
use crate::paging;

/// Ask for a board: a saved search or a search text, and the options of the view.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct BoardBody {
    /// The saved search the board is a view of, by its ID; or else `search_text`.
    #[serde(default)]
    saved_search: Option<String>,
    /// The search text, which need not be saved; used when `saved_search` is left out.
    #[serde(default)]
    search_text: Option<String>,
    /// The project whose board it is, by ID or short name: `project within KEY` is written into
    /// the text, the project and everything below it; left out, everywhere the person sees.
    #[serde(default)]
    project: Option<String>,
    /// *Only this project*: `project is KEY` in place of `project within KEY`.
    #[serde(default)]
    only_this_project: bool,
    /// The options of the view: the columns, the rows and the card limits. Left out, a saved
    /// search shows the ones it keeps, and a search text a column for each status.
    #[serde(default)]
    board: Option<BoardOptionsBody>,
    /// The `next_cursor` of one cell, for its next cards (*Show more*); left out, the whole board.
    #[serde(default)]
    more: Option<String>,
    /// How many cards a cell holds at most; left out, the most.
    #[serde(default)]
    #[schema(schema_with = cards_limit_schema)]
    limit: Option<u32>,
}

/// The number of cards of a cell as the API description says it, from `bb_app::boards`.
fn cards_limit_schema() -> Object {
    ObjectBuilder::new()
        .schema_type(Type::Integer)
        .minimum(Some(1))
        .maximum(Some(CARDS_MAX))
        .description(Some(format!(
            "How many cards a cell holds at most, from 1 to {CARDS_MAX}; any other number is refused with `request.invalid`."
        )))
        .build()
}

/// A row of a board.
#[derive(Serialize, ToSchema)]
pub(crate) struct BoardRowBody {
    /// What the row is, as the reader may see it (people shown alike are one row).
    key: SearchGroupKeyBody,
    /// How many issues the reader sees in it.
    issues: u64,
}

/// Why a card may not go to a column.
#[derive(Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MoveRefusedBody {
    /// The card's workflow has no status of the column (or none of its status group).
    NoSuchStatus,
    /// The workflow has it, but no change the reader may make leads there now.
    NoSuchChange,
    /// The reader may not change the field of the columns on this issue.
    CannotChange,
}

impl From<MoveRefused> for MoveRefusedBody {
    fn from(refused: MoveRefused) -> Self {
        match refused {
            MoveRefused::NoSuchStatus => Self::NoSuchStatus,
            MoveRefused::NoSuchChange => Self::NoSuchChange,
            MoveRefused::CannotChange => Self::CannotChange,
        }
    }
}

/// Where a card may go, or why not.
#[derive(Serialize, ToSchema)]
pub(crate) struct CardMoveBody {
    /// The column, by its place among the columns.
    column: usize,
    /// The status the card enters there (the `to` of the status change); `null` for a column of
    /// choices, whose move is the change of the field.
    #[schema(required = true)]
    to: Option<String>,
    /// The fields the change asks for first; empty when it needs none.
    needs: Vec<FieldKey>,
    /// Why the card may not go there; `null` when it may.
    #[schema(required = true)]
    refused: Option<MoveRefusedBody>,
}

impl From<&CardMove> for CardMoveBody {
    fn from(each: &CardMove) -> Self {
        Self {
            column: each.column,
            to: each.to.as_ref().map(ToString::to_string),
            needs: each.needs.clone(),
            refused: each.refused.map(MoveRefusedBody::from),
        }
    }
}

/// A card.
#[derive(Serialize, ToSchema)]
pub(crate) struct BoardCardBody {
    /// The issue, with what the reader may see of it in its project.
    #[schema(value_type = IssueItem)]
    issue: Value,
    /// The fields the card shows, in order (the layout *Card* of its type, as the reader may read
    /// it); the Issue ID and the title always show.
    fields: Vec<FieldKey>,
    /// Where it may go and why not, one for each column it is not in.
    moves: Vec<CardMoveBody>,
}

/// A column.
#[derive(Serialize, ToSchema)]
pub(crate) struct BoardColumnBody {
    /// What the column stands for as the search language writes it: the name of a status, the
    /// word of a status group, the name of a choice; `null` for the issues without a value.
    #[schema(required = true)]
    key: Option<String>,
    /// The status group of a column of a status group.
    #[schema(required = true)]
    status_group: Option<StatusGroup>,
    /// How many issues the reader sees in it.
    count: u64,
    /// Its card limit; `null` for none.
    #[schema(required = true)]
    limit: Option<u32>,
    /// Whether more cards than the limit are in it: nothing is refused.
    over: bool,
}

/// The cards of one column in one row.
#[derive(Serialize, ToSchema)]
pub(crate) struct BoardCellBody {
    /// The column, by its place.
    column: usize,
    /// The row, by its place; `null` on a board without rows.
    #[schema(required = true)]
    row: Option<usize>,
    /// How many issues the reader sees in the cell.
    count: u64,
    /// The first cards, in the order of the search (at the start: priority, then last changed).
    cards: Vec<BoardCardBody>,
    /// The cursor of the next cards of the cell (give it as `more`); `null` when there are none.
    #[schema(required = true)]
    next_cursor: Option<String>,
}

/// A board.
#[derive(Default, Serialize, ToSchema)]
pub(crate) struct BoardPageBody {
    /// The search text that was run: a project's board has its project written in.
    search_text: String,
    /// The columns, in order; empty columns stay.
    columns: Vec<BoardColumnBody>,
    /// The rows, in order, at most 20; empty without rows.
    rows: Vec<BoardRowBody>,
    /// Whether the search found more rows than the board shows.
    rows_truncated: bool,
    /// The cells that hold issues.
    cells: Vec<BoardCellBody>,
}

impl Masked for BoardPageBody {}

/// The name of the list the cursors of a board are sealed for: the person and the board (its
/// search, project, columns and rows), so a cursor opens for nobody else and for no other board.
fn list_of(person: bb_core::id::UserId, body: &BoardBody) -> String {
    let options = body.board.as_ref();
    bb_app::seal::list_name(
        "board",
        person,
        &[
            body.saved_search.as_deref(),
            body.search_text.as_deref(),
            body.project.as_deref(),
            body.only_this_project.then_some("only-this-project"),
            options.and_then(BoardOptionsBody::columns),
            options.and_then(BoardOptionsBody::rows),
        ],
    )
}

fn card_body(card: BoardCard) -> Result<BoardCardBody, ApiError> {
    Ok(BoardCardBody {
        issue: card
            .issue
            .map(|view| IssueItem::from(&view))
            .masked_value()
            .map_err(|error| ApiError::internal(&error))?,
        fields: card.fields,
        moves: card.moves.iter().map(CardMoveBody::from).collect(),
    })
}

/// The body of a board, each card's issue with what the reader may see of it.
fn page_body(
    page: BoardPage,
    person: bb_core::id::UserId,
    body: &BoardBody,
) -> Result<BoardPageBody, ApiError> {
    let list = list_of(person, body);
    let mut cells = Vec::with_capacity(page.cells.len());
    for cell in page.cells {
        let BoardCell {
            column,
            row,
            count,
            cards,
            next,
        } = cell;
        cells.push(BoardCellBody {
            column,
            row,
            count,
            cards: cards.into_iter().map(card_body).collect::<Result<_, _>>()?,
            next_cursor: next
                .map(|ask| paging::seal(&list, &ask.position()))
                .transpose()?,
        });
    }
    Ok(BoardPageBody {
        search_text: page.search_text,
        columns: page
            .columns
            .into_iter()
            .map(|each| BoardColumnBody {
                key: each.key,
                status_group: each.status_group,
                count: each.count,
                limit: each.limit,
                over: each.over,
            })
            .collect(),
        rows: page
            .rows
            .into_iter()
            .map(|row| BoardRowBody {
                key: group_key(row.key),
                issues: row.issues,
            })
            .collect(),
        rows_truncated: page.rows_truncated,
        cells,
    })
}

/// A board of a search: a column for each status (statuses of the same name merged across the
/// workflows of the issues found), status group or choice, with how many issues the reader sees in
/// each and its card limit (`over` when the count is above it, nothing refused); the rows; and the
/// first cards of each cell, in the order of the search, each with the fields of its layout *Card*
/// and where it may go (`moves`, from what its own workflow allows the reader). Restricted issues
/// are in no column and no count. With `more`, the next cards of one cell. It reads and counts
/// against the searches of the person; an access key reads it as its person may.
#[utoipa::path(
    post,
    path = "/api/v1/board",
    operation_id = "getBoard",
    tag = "board",
    request_body = BoardBody,
    responses(
        (status = 200, description = "A board.", body = BoardPageBody),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn get_board(
    State(auth): State<AuthHandle>,
    Extension(limits): Extension<Arc<Limits>>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<BoardBody>,
) -> Result<Response, ApiError> {
    let person = counted(&auth, &limits, &info, &session_token).await?;
    let more = body
        .more
        .as_deref()
        .map(|cursor| {
            let position = paging::open(&list_of(person, &body), cursor)?;
            CellAsk::of_position(&position)
                .ok_or_else(|| ApiError::new(bb_core::ErrorCode::BadCursor))
        })
        .transpose()?;
    let search = view_search(body.saved_search.as_deref(), body.search_text.as_deref())?;
    let page = auth
        .api()?
        .board(
            info,
            BoardQuery {
                session_token,
                search,
                project: body.project.clone(),
                only_this_project: body.only_this_project,
                options: body.board.clone().map(Into::into),
                more,
                limit: body.limit.unwrap_or(CARDS_MAX),
            },
        )
        .await?;
    // Each card's issue is masked with what the reader holds in its project; a failure to do so
    // stops the answer.
    let mut failed = None;
    let answer = page.map(|page| {
        page_body(page, person, &body).unwrap_or_else(|error| {
            failed = Some(error);
            BoardPageBody::default()
        })
    });
    if let Some(error) = failed {
        return Err(error);
    }
    answer_authorized(&answer)
}

pub(crate) fn routes<S>(limits: &Arc<Limits>) -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_board))
        .layer(Extension(Arc::clone(limits)))
}
