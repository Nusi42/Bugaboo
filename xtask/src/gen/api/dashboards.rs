//! The grid and the limits of dashboards and the words of the sums of a total (CFG-03, QRY-08), as
//! constants for the browser client.

use super::TS_HEADER;

/// The grid and the limits of dashboards (ADR-0050, decision 3), from the linked `bb-core` that
/// owns them: the columns of the grid, the least and the most size of a box and the most boxes of
/// one dashboard, and the words of the sums of a total that the boxes and the lists ask for. The
/// screen places boxes within them and does not write them a second time.
pub(super) fn dashboards() -> String {
    use bb_core::dashboard::{
        BOX_HEIGHT_MAX, BOX_HEIGHT_MIN, BOX_WIDTH_MIN, BOXES_MAX, GRID_COLUMNS,
    };
    format!(
        "{TS_HEADER}/** The columns of the grid of a dashboard (`GRID_COLUMNS`). */
export const gridColumns = {GRID_COLUMNS};
/** The narrowest a box is, in columns (`BOX_WIDTH_MIN`). */
export const boxWidthLeast = {BOX_WIDTH_MIN};
/** The lowest a box is, in rows (`BOX_HEIGHT_MIN`). */
export const boxHeightLeast = {BOX_HEIGHT_MIN};
/** The highest a box is, in rows (`BOX_HEIGHT_MAX`). */
export const boxHeightMost = {BOX_HEIGHT_MAX};
/** The most boxes one dashboard holds (`BOXES_MAX`). */
export const boxesMost = {BOXES_MAX};
/** The words of the sums a total adds up (`bb_core::totals`); an own field of numbers is `number` and its word. */
export const sumWords = {{
  count: \"{count}\",
  estimated: \"{estimated}\",
  timeLeft: \"{left}\",
  timeSpent: \"{spent}\",
  number: \"{number}\",
}} as const;
",
        count = bb_core::totals::COUNT,
        estimated = bb_core::totals::ESTIMATED,
        left = bb_core::totals::TIME_LEFT,
        spent = bb_core::totals::TIME_SPENT,
        number = bb_core::totals::NUMBER_SUM,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cfg_03_the_grid_and_the_sums_come_from_the_core() {
        let written = dashboards();
        assert!(written.contains(&format!(
            "export const gridColumns = {};",
            bb_core::dashboard::GRID_COLUMNS
        )));
        assert!(written.contains(&format!("timeSpent: \"{}\"", bb_core::totals::TIME_SPENT)));
    }
}
