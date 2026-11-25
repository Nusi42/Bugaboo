//! What the board reads take and answer (PLAN-06, ADR-0034, amendment of 2026-11-24).

use bb_core::fields::FieldKey;
use bb_core::issue::StatusGroup;
use bb_core::saved_search::BoardOptions;
use bb_core::workflow::StatusKey;
use bb_policy::{Authorized, Masked};
use serde::Serialize;

use crate::issues::IssueView;
use crate::search::{ShownKey, ViewSearch};

/// The most cards one cell of a board answers at once; *Show more* reads the next ones by the
/// cursor of the cell.
pub const CARDS_MAX: u32 = 50;

/// The most rows one board has: the rows past them are left out, and the answer says so
/// (`rows_truncated`).
pub const ROWS_MAX: usize = 20;

/// Ask for a board.
pub struct BoardQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The search the board shows.
    pub search: ViewSearch,
    /// The project whose board it is (it and the projects inside it), by ID or short name; none
    /// for everywhere the reader sees.
    pub project: Option<String>,
    /// *Only this project*: the project's board without its sub-projects (`project is KEY` in
    /// place of `project within KEY` in the text).
    pub only_this_project: bool,
    /// The options of the view; none for those the saved search keeps (the starting ones for a
    /// search text).
    pub options: Option<BoardOptions>,
    /// The one cell whose next cards are wanted (*Show more*); none for the whole board.
    pub more: Option<CellAsk>,
    /// How many cards a cell holds at most, 1 to [`CARDS_MAX`].
    pub limit: u32,
}

/// Where the next cards of one cell start: its column and row by their places, how many of its
/// cards were shown, and the mark of the options of the board they were shown with. The position
/// of a cursor: it holds no value of any issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CellAsk {
    /// The column, by its place.
    pub column: usize,
    /// The row, by its place; none on a board without rows.
    pub row: Option<usize>,
    /// How many cards of the cell come before.
    pub skip: u64,
    /// The mark of the options of the board in force ([`super::options_mark`]): a cursor of a
    /// saved board whose options changed since points at no cell.
    pub mark: u64,
}

impl CellAsk {
    /// The position as text (`2.-.50.<mark>`, `2.0.50.<mark>`).
    #[must_use]
    pub fn position(self) -> String {
        let row = self
            .row
            .map_or_else(|| "-".to_owned(), |row| row.to_string());
        format!("{}.{row}.{}.{}", self.column, self.skip, self.mark)
    }

    /// The position a text of [`CellAsk::position`] holds; none for any other text.
    #[must_use]
    pub fn of_position(text: &str) -> Option<Self> {
        let mut parts = text.split('.');
        let column = parts.next()?.parse().ok()?;
        let row = match parts.next()? {
            "-" => None,
            row => Some(row.parse().ok()?),
        };
        let skip = parts.next()?.parse().ok()?;
        let mark = parts.next()?.parse().ok()?;
        parts.next().is_none().then_some(Self {
            column,
            row,
            skip,
            mark,
        })
    }
}

/// A row of a board: what it stands for as the reader may see it, and how many issues of the
/// search it holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BoardRow {
    /// What the row is.
    pub key: ShownKey,
    /// How many issues the reader sees in it.
    pub issues: u64,
}

/// Why a card may not go to a column (the screen fades the column with this reason).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MoveRefused {
    /// The workflow of the card has no status of the column (*"WEB-139's workflow has no status
    /// Testing"*), or none of its status group.
    NoSuchStatus,
    /// The workflow has the status, but no change the reader may make leads there from where the
    /// card is now. Trying it answers the refusal of the status change with its reason.
    NoSuchChange,
    /// The reader may not change the field of the columns on this issue.
    CannotChange,
}

/// Where a card may go, or why not: one for each column the card is not in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CardMove {
    /// The column, by its place among the columns.
    pub column: usize,
    /// The status the card enters there, for columns of statuses and status groups; the move is
    /// that status change of its workflow (ADR-0031). None for a column of choices, whose move is
    /// the change of the field.
    pub to: Option<StatusKey>,
    /// The fields the change asks for first (*Ask for*, *Must be filled in*); empty when it needs
    /// none.
    pub needs: Vec<FieldKey>,
    /// Why the card may not go there; none when it may.
    pub refused: Option<MoveRefused>,
}

/// A card of a board.
#[derive(Debug, Serialize)]
pub struct BoardCard {
    /// The issue, with what the reader may see of it in its project.
    pub issue: Authorized<IssueView>,
    /// The fields its card shows, in order: those of the layout *Card* of its type that the reader
    /// may read (ADR-0027, amendment of 2026-11-24). The Issue ID and the title always show.
    pub fields: Vec<FieldKey>,
    /// Where it may go and why not, as its own workflow says.
    pub moves: Vec<CardMove>,
}

/// A column of a board.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BoardColumn {
    /// What the column stands for as the search language writes it: the name of a status, the
    /// word of a status group, the name of a choice; none for the column of the issues without a
    /// value. Its card limit and *Show more* name it so.
    pub key: Option<String>,
    /// The status group of a column of a status group.
    pub status_group: Option<StatusGroup>,
    /// How many issues the reader sees in it.
    pub count: u64,
    /// Its card limit, when it has one.
    pub limit: Option<u32>,
    /// Whether more cards than the limit are in it (the title turns orange); nothing is refused.
    pub over: bool,
}

/// The cards of one column in one row.
#[derive(Debug, Serialize)]
pub struct BoardCell {
    /// The column, by its place.
    pub column: usize,
    /// The row, by its place; none on a board without rows.
    pub row: Option<usize>,
    /// How many issues the reader sees in the cell.
    pub count: u64,
    /// The first cards, in the order of the search.
    pub cards: Vec<BoardCard>,
    /// Where the next cards of the cell start (*Show more*), if there are more.
    pub next: Option<CellAsk>,
}

/// A board: its columns with their counts and limits, its rows, and the first cards of each cell.
#[derive(Debug, Serialize)]
pub struct BoardPage {
    /// The search text that was run: a project's board has its project written in.
    pub search_text: String,
    /// The columns, in order; empty columns stay.
    pub columns: Vec<BoardColumn>,
    /// The rows, in order, at most [`ROWS_MAX`]; empty on a board without rows.
    pub rows: Vec<BoardRow>,
    /// Whether the search found more rows than [`ROWS_MAX`] and the rest were left out.
    pub rows_truncated: bool,
    /// The cells that hold issues, by column and row.
    pub cells: Vec<BoardCell>,
}

impl Masked for BoardPage {}

#[cfg(test)]
mod tests {
    use super::CellAsk;

    #[test]
    fn plan_06_the_position_of_a_cell_is_read_back_and_nothing_else_is() {
        for ask in [
            CellAsk {
                column: 2,
                row: None,
                skip: 50,
                mark: 7,
            },
            CellAsk {
                column: 0,
                row: Some(19),
                skip: 100,
                mark: u64::MAX,
            },
        ] {
            assert_eq!(CellAsk::of_position(&ask.position()), Some(ask));
        }
        for bad in ["", "1", "1.-", "1.-.5", "1.x.5.0", "1.-.5.6.7", "-1.-.5.0"] {
            assert_eq!(CellAsk::of_position(bad), None, "{bad}");
        }
    }
}
