import { lengthInPixels } from "./css-length.ts";
import type { ListColumn } from "./list-columns.ts";
import type { TokenName } from "./tokens.ts";

/** A column of the issue list and the width a person gave it; none is the width the theme file gives. */
export interface ListColumnSpec {
  readonly column: ListColumn;
  readonly width?: number | undefined;
}

/**
 * The tokens whose sum is the width each column is drawn at: the one place that says it (the
 * template of the list and the count of the columns that fit both read it). The title has a least
 * width and takes the room that is left; the priority holds its name and the arrow of the sort; a
 * column of a field holds its width and the edge that sets it (24 px, 44 on a finger), so its name is
 * not cut where there is room.
 */
const widthTokens = {
  select: ["size-control"],
  status: ["size-col-icon"],
  key: ["size-col-key", "size-target-compact"],
  project: ["size-col-project", "size-target-compact"],
  type: ["size-col-project", "size-target-compact"],
  title: ["size-col-title-least"],
  labels: ["size-col-labels", "size-target-compact"],
  priority: ["size-col-priority", "size-icon-small"],
  person: ["size-col-person", "size-target-compact"],
  value: ["size-col-value", "size-target-compact"],
  changed: ["size-col-changed", "size-target-compact"],
  action: ["size-col-icon"],
} as const satisfies Readonly<Record<ListColumn, readonly TokenName[]>>;

function variable(token: TokenName): string {
  return `var(--${token})`;
}

/** The width of a column as a CSS length: its tokens added. */
function lengthOf(column: ListColumn): string {
  const tokens = widthTokens[column];
  return tokens.length === 1
    ? variable(tokens[0])
    : `calc(${tokens.map((token) => variable(token)).join(" + ")})`;
}

/**
 * The columns of the head and of every row as one `grid-template-columns`: the width a person gave
 * a column, or the one the theme file gives its kind, and the title takes what is left. The ID
 * column holds the longest ID that is drawn (`keyChars` characters of the code font).
 */
export function listTemplate(columns: readonly ListColumnSpec[], keyChars = 0): string {
  return columns
    .map(({ column, width }) => {
      if (width !== undefined) {
        return `${String(width)}px`;
      }
      if (column === "title") {
        // The title is what people read: the columns of fields give way before it.
        return `minmax(${lengthOf(column)}, 1fr)`;
      }
      if (column === "key" && keyChars > 0) {
        // The ID is as wide as the longest one drawn, and not narrower than its head needs.
        return `max(${lengthOf(column)}, calc(${variable("size-code-char")} * ${String(keyChars)}))`;
      }
      // A column has its width and the title the room that is left; where there is not room for the columns, the fit drops those of fields (listNeeds).
      return lengthOf(column);
    })
    .join(" ");
}

/**
 * The size in pixels of a length token of the theme file, as the page has it (the density and the
 * size of the text counted, and the sizes of a touch screen); none where there is no page.
 */
export function pixelsOf(token: TokenName): number {
  if (typeof document === "undefined") {
    return 0;
  }
  const style = getComputedStyle(document.documentElement);
  return lengthInPixels(`var(--${token})`, (name) => style.getPropertyValue(name));
}

/** The width in pixels of the tokens of a column added, as the page has them. */
function pixelsOfColumn(column: ListColumn): number {
  return widthTokens[column].reduce((sum, token) => sum + pixelsOf(token), 0);
}

/**
 * The width in pixels a list needs for these columns where none gives way: the width each is
 * drawn at (the one a person gave it, or the ID as wide as the longest ID drawn), the gaps of the
 * row, the edge and the padding. The list counts it to see which columns fit. The gap (`space-3`),
 * the edge (`size-edge`) and the padding (`pad-box`) are the spacing of `list-row` in the theme
 * file: change them there and here together.
 */
export function listNeeds(columns: readonly ListColumnSpec[], keyChars = 0): number {
  const widths = columns.map(({ column, width }) => {
    if (width !== undefined) {
      return width;
    }
    const usual = pixelsOfColumn(column);
    return column === "key" ? Math.max(usual, pixelsOf("size-code-char") * keyChars) : usual;
  });
  const gaps = Math.max(0, columns.length - 1) * pixelsOf("space-3");
  const around = pixelsOf("size-edge") + 2 * pixelsOf("pad-box");
  return widths.reduce((sum, width) => sum + width, gaps + around);
}
