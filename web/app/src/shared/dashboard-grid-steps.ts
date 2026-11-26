import { gridColumns } from "@bugaboo/api-client";

/** What the browser computed for the grid of a dashboard: its width, and its gaps and row height as CSS lengths. */
export interface GridMeasure {
  readonly width: number;
  readonly columnGap: string;
  readonly rowHeight: string;
  readonly rowGap: string;
}

/** How far one column and one row of the grid are on screen, in pixels. */
export interface GridSteps {
  readonly column: number;
  readonly row: number;
}

/** A length the browser computed, in pixels (`72px`); none it cannot read. */
function pixels(length: string): number {
  return Number(length.replace("px", "")) || 0;
}

/**
 * How far a column and a row of the grid are: a column of the width shared by the columns of the grid, a row of its
 * height, each with its gap. The one measure of the steps of a drag, which the tests of the browser take too.
 */
export function stepsFrom(measure: GridMeasure): GridSteps {
  const columnGap = pixels(measure.columnGap);
  return {
    column: (measure.width + columnGap) / gridColumns,
    row: pixels(measure.rowHeight) + pixels(measure.rowGap),
  };
}

/** The measure of the grid element as the browser draws it now. */
export function measureOf(grid: HTMLElement): GridMeasure {
  const style = getComputedStyle(grid);
  return {
    width: grid.clientWidth,
    columnGap: style.columnGap,
    rowHeight: style.gridAutoRows,
    rowGap: style.rowGap,
  };
}
