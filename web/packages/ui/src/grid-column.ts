/** The attribute that names the column of a cell in a grid; `gridColumnSelector` finds its neighbors. */
export const GRID_COLUMN = "data-column";

/** The cells under each other in one column of the grid, in the order of the page. */
export function gridColumnSelector(column: number): string {
  return `input[${GRID_COLUMN}="${column}"]`;
}
