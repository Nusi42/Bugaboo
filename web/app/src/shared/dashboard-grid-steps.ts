import { gridColumns } from "@bugaboo/api-client";

/**
 * What the browser computed for the grid of a dashboard: its width, its gaps, the least height of a row and the height
 * of each row it draws now, as CSS lengths.
 */
export interface GridMeasure {
  readonly width: number;
  readonly columnGap: string;
  readonly rowHeight: string;
  readonly rowGap: string;
  /** The heights of the rows as drawn (`grid-template-rows`): a row grows with what its boxes hold. */
  readonly rows?: string | undefined;
}

/** How far one column and one row of the grid are on screen, in pixels, and how high each row is drawn. */
export interface GridSteps {
  readonly column: number;
  /** The least step of a row: its least height and the gap. */
  readonly row: number;
  readonly rowGap: number;
  /** The height of each row as drawn, from the top; a row past them has the least height. */
  readonly rows: readonly number[];
}

/** The first length in pixels in a CSS value the browser computed (`72px`, `minmax(72px, auto)`); 0 if none. */
function pixels(length: string): number {
  const found = /(-?\d+(?:\.\d+)?)px/u.exec(length);
  return found === null ? 0 : Number(found[1]);
}

/**
 * How far a column and a row of the grid are: a column of the width shared by the columns of the grid, a row of its
 * least height, each with its gap, and the height of each row as drawn. The one measure of the steps of a drag, which
 * the tests of the browser take too.
 */
export function stepsFrom(measure: GridMeasure): GridSteps {
  const columnGap = pixels(measure.columnGap);
  const rowGap = pixels(measure.rowGap);
  return {
    column: (measure.width + columnGap) / gridColumns,
    row: pixels(measure.rowHeight) + rowGap,
    rowGap,
    rows: (measure.rows ?? "")
      .split(" ")
      .map((each) => pixels(each))
      .filter((height) => height > 0),
  };
}

/**
 * How many rows a pointer went from the edge of row `from` by `distance` pixels down (or up): each row it passes by
 * more than half of its height, as drawn, with its gap. Rows grow with what they hold, so a row is not one step long.
 */
export function rowsCrossed(steps: GridSteps, from: number, distance: number): number {
  const down = distance >= 0;
  const heightOf = (row: number): number =>
    (steps.rows[row] ?? steps.row - steps.rowGap) + steps.rowGap;
  let left = Math.abs(distance);
  let crossed = 0;
  for (let row = down ? from : from - 1; row >= 0; row += down ? 1 : -1) {
    const height = heightOf(row);
    if (left < height / 2) {
      break;
    }
    left -= height;
    crossed += 1;
  }
  // No row crossed is 0 either way (not -0).
  return down || crossed === 0 ? crossed : -crossed;
}

/** The measure of the grid element as the browser draws it now. */
export function measureOf(grid: HTMLElement): GridMeasure {
  const style = getComputedStyle(grid);
  return {
    width: grid.clientWidth,
    columnGap: style.columnGap,
    rowHeight: style.gridAutoRows,
    rowGap: style.rowGap,
    rows: style.gridTemplateRows,
  };
}
