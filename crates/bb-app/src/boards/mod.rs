//! Boards (PLAN-06, PRJ-06, ADR-0034 and ADR-0027, amendments of 2026-11-24): a saved search or a
//! search text shown as columns of statuses, status groups or choices, with cards.
//!
//! - **The columns** (`columns.rs`) are values of the field of the columns: statuses of the same
//!   name are one column across the workflows of the issues found (M10-Q1).
//! - **The read** (`read.rs`) prepares the search once and reads the counts of the columns, the
//!   counts of every cell and the first cards of every cell in a statement each, all with the
//!   reader's rights; a card limit is soft (M10-Q2) and the cards follow the sort of the search
//!   (M10-Q3).
//! - **A card** shows the fields of the layout *Card* of its type (M10-Q4) and says where it may
//!   go (`moves.rs`), from the status changes its workflow allows the reader. The move itself is
//!   the status change or the field change that exists; no rule of a workflow is written here.

mod cells;
mod columns;
mod moves;
mod read;
mod types;

pub use cells::options_mark;
pub(crate) use columns::{columns_by, named_by_limits};
pub use read::board;
pub use types::*;
