//! The cells of a board (PLAN-06): the heads of the columns, the rows in the order the board shows
//! them, the counts of the cells, and the cards of each cell with where its next cards start.
//!
//! **Which rows** (architecture §7, *Views of a search*): the rows stand in the order the board
//! shows them, by the name the reader sees (people, projects, goals) or by the field's own order
//! (priorities, statuses, choices), never by an internal ID; the row of the issues without a value
//! (*No one*, *None*) stands first and always stays; past [`ROWS_MAX`] the rest are cut in that
//! order and the answer says so.

use bb_core::issue::Issue;
use bb_core::saved_search::CardLimit;

use super::columns::{Column, same_column};
use super::types::{BoardColumn, BoardRow, CellAsk, ROWS_MAX};
use crate::search::{CellAt, CellHit, GroupBy, GroupKey, ShownKey, Total, Totals, TotalsAsked};

/// The place of a row in the order the board shows: the place in the field's own order, then the
/// name the reader sees.
pub(super) type RowPlace = (u64, String);

/// The cards of a cell, read from the store, before what they show is read.
pub(super) struct CellFound {
    pub(super) column: usize,
    pub(super) row: Option<usize>,
    pub(super) count: u64,
    pub(super) issues: Vec<Issue>,
    pub(super) next: Option<CellAsk>,
}

/// The rows of a board: their groups as the store keys them, how the reader sees them, and the
/// grouping that made them, at most [`ROWS_MAX`] of them.
pub(super) struct RowsRead {
    pub(super) keys: Vec<GroupKey>,
    pub(super) shown: Vec<BoardRow>,
    pub(super) by: Option<GroupBy>,
    pub(super) truncated: bool,
}

impl RowsRead {
    /// A board without rows.
    pub(super) const fn none() -> Self {
        Self {
            keys: Vec::new(),
            shown: Vec::new(),
            by: None,
            truncated: false,
        }
    }
}

/// The count of one cell: its column, its row by its place, and how many issues it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CellCount {
    pub(super) column: usize,
    pub(super) row: Option<usize>,
    pub(super) issues: u64,
}

/// One group of the totals of a board with rows: the row as shown and its total, and the keys of
/// the row and of the column as the store gave them.
pub(super) type RowGroup = ((ShownKey, Total), (GroupKey, Option<GroupKey>));

/// The totals asked of a search grouped by one field, without sums.
pub(super) fn grouped_by(word: &str) -> TotalsAsked {
    TotalsAsked {
        by: Some(GroupBy {
            word: word.to_owned(),
            people: Vec::new(),
            page: None,
            then: None,
        }),
        sums: Vec::new(),
        all: false,
    }
}

/// The column a value of the field of the columns is in.
pub(super) fn column_at(columns: &[Column], key: &GroupKey) -> Option<usize> {
    columns
        .iter()
        .position(|column| column.groups.contains(key))
}

/// The heads of the columns: what each stands for, its count of what the reader sees (the groups
/// of the search by the field of the columns, each value in one column) and its soft card limit
/// (M10-Q2): `over` says the count is above it, and nothing is refused.
pub(super) fn column_heads(
    columns: &[Column],
    totals: &Totals,
    limits: &[CardLimit],
) -> Vec<BoardColumn> {
    columns
        .iter()
        .map(|column| {
            let count = totals
                .groups
                .iter()
                .filter(|group| column.groups.contains(&group.key))
                .map(|group| group.total.issues)
                .sum();
            let limit = limits
                .iter()
                .find(|each| same_column(each.column.as_deref(), column.key.as_deref()))
                .map(|each| each.limit);
            BoardColumn {
                key: column.key.clone(),
                status_group: column.status_group,
                count,
                limit,
                over: limit.is_some_and(|limit| count > u64::from(limit)),
            }
        })
        .collect()
}

/// The counts of the cells of a board without rows: those of the columns.
pub(super) fn column_counts(heads: &[BoardColumn]) -> Vec<CellCount> {
    heads
        .iter()
        .enumerate()
        .filter(|(_, head)| head.count > 0)
        .map(|(column, head)| CellCount {
            column,
            row: None,
            issues: head.count,
        })
        .collect()
}

/// The rows of a board and the counts of its cells from the totals grouped by the rows and then by
/// the columns: a group whose value of the columns is in no column takes no row and no cell; the
/// rows in the order the board shows (`place`), the empty row first, at most [`ROWS_MAX`].
pub(super) fn rows_and_counts(
    groups: Vec<RowGroup>,
    (columns, by): (&[Column], Option<GroupBy>),
    place: impl Fn(&GroupKey, &ShownKey) -> RowPlace,
) -> (RowsRead, Vec<CellCount>) {
    let mut rows: Vec<(GroupKey, BoardRow)> = Vec::new();
    let mut cells: Vec<(GroupKey, usize, u64)> = Vec::new();
    for ((shown, total), (row, then)) in groups {
        let Some(column) = then.as_ref().and_then(|then| column_at(columns, then)) else {
            continue;
        };
        match rows.iter_mut().find(|(key, _)| *key == row) {
            Some((_, each)) => each.issues = each.issues.saturating_add(total.issues),
            None => rows.push((
                row.clone(),
                BoardRow {
                    key: shown,
                    issues: total.issues,
                },
            )),
        }
        cells.push((row, column, total.issues));
    }
    rows.sort_by_cached_key(|(key, shown)| (*key != GroupKey::Empty, place(key, &shown.key)));
    let truncated = rows.len() > ROWS_MAX;
    rows.truncate(ROWS_MAX);
    let (keys, shown): (Vec<GroupKey>, Vec<BoardRow>) = rows.into_iter().unzip();
    let counts = cells
        .into_iter()
        .filter_map(|(row, column, issues)| {
            let at = keys.iter().position(|key| *key == row)?;
            Some(CellCount {
                column,
                row: Some(at),
                issues,
            })
        })
        .collect();
    let read = RowsRead {
        keys,
        shown,
        by,
        truncated,
    };
    (read, merged_counts(counts))
}

/// The counts of the cells with the counts of the same cell put together: a column of several
/// statuses of one name is one cell.
fn merged_counts(counts: Vec<CellCount>) -> Vec<CellCount> {
    let mut merged: Vec<CellCount> = Vec::with_capacity(counts.len());
    for count in counts {
        match merged
            .iter_mut()
            .find(|each| each.column == count.column && each.row == count.row)
        {
            Some(each) => each.issues = each.issues.saturating_add(count.issues),
            None => merged.push(count),
        }
    }
    merged
}

/// The cells of the counts, each with its cards from `hits` and where its next cards start; for
/// *Show more* only the one cell asked for.
pub(super) fn cells_found(
    counts: Vec<CellCount>,
    mut hits: Vec<CellHit>,
    rows: &RowsRead,
    more: Option<CellAsk>,
) -> Vec<CellFound> {
    let skip = more.map_or(0, |ask| ask.skip);
    let mut found = Vec::new();
    for count in counts {
        if more.is_some_and(|ask| ask.column != count.column || ask.row != count.row) {
            continue;
        }
        let at = CellAt {
            column: count.column,
            row: count.row.and_then(|row| rows.keys.get(row).cloned()),
        };
        let (mine, others): (Vec<CellHit>, Vec<CellHit>) =
            hits.into_iter().partition(|hit| hit.at == at);
        hits = others;
        let issues: Vec<Issue> = mine.into_iter().map(|hit| hit.issue).collect();
        let shown = skip.saturating_add(u64::try_from(issues.len()).unwrap_or(u64::MAX));
        found.push(CellFound {
            column: count.column,
            row: count.row,
            count: count.issues,
            next: (shown < count.issues).then_some(CellAsk {
                column: count.column,
                row: count.row,
                skip: shown,
            }),
            issues,
        });
    }
    found
}

#[cfg(test)]
mod tests {
    use bb_core::issue::Priority;

    use super::*;
    use crate::search::Total;

    fn column(keys: &[GroupKey]) -> Column {
        Column {
            key: None,
            status_group: None,
            groups: keys.to_vec(),
        }
    }

    fn group(row: GroupKey, then: GroupKey, issues: u64) -> RowGroup {
        (
            (
                ShownKey::Other(row.clone()),
                Total {
                    issues,
                    sums: Vec::new(),
                },
            ),
            (row, Some(then)),
        )
    }

    #[test]
    fn plan_06_rows_stand_in_the_order_of_the_field_with_the_empty_row_first_and_cut_after() {
        let open = GroupKey::Text("open".to_owned());
        let columns = [column(std::slice::from_ref(&open))];
        let priority = |each: Priority| GroupKey::Priority(each);
        let mut groups = vec![
            group(priority(Priority::Low), open.clone(), 1),
            group(GroupKey::Empty, open.clone(), 2),
            group(priority(Priority::Critical), open.clone(), 3),
            // A value of the columns that is in no column takes no row.
            group(
                priority(Priority::High),
                GroupKey::Text("gone".to_owned()),
                4,
            ),
        ];
        let order = |key: &GroupKey, _: &ShownKey| -> RowPlace {
            let at = Priority::LIST_ORDER
                .iter()
                .position(|each| *key == GroupKey::Priority(*each))
                .unwrap_or_default();
            (u64::try_from(at).unwrap_or_default(), String::new())
        };
        let (rows, counts) = rows_and_counts(groups.clone(), (&columns, None), order);
        assert_eq!(
            rows.keys,
            [
                GroupKey::Empty,
                priority(Priority::Critical),
                priority(Priority::Low)
            ]
        );
        assert!(!rows.truncated);
        assert_eq!(counts.iter().map(|count| count.issues).sum::<u64>(), 6);
        // Past the most rows the rest are cut in the order shown, and the empty row stays.
        groups = (0..30)
            .map(|at| group(GroupKey::Whole(at), open.clone(), 1))
            .chain(std::iter::once(group(GroupKey::Empty, open.clone(), 1)))
            .collect();
        let by_number = |key: &GroupKey, _: &ShownKey| -> RowPlace {
            match key {
                GroupKey::Whole(number) => {
                    (u64::try_from(*number).unwrap_or_default(), String::new())
                }
                _ => (0, String::new()),
            }
        };
        let (rows, counts) = rows_and_counts(groups, (&columns, None), by_number);
        assert!(rows.truncated);
        assert_eq!(rows.keys.len(), ROWS_MAX);
        assert_eq!(rows.keys[0], GroupKey::Empty);
        assert_eq!(rows.keys[ROWS_MAX - 1], GroupKey::Whole(18));
        assert_eq!(counts.len(), ROWS_MAX);
    }
}
