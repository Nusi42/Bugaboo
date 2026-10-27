import type { ListColumn } from "./list-columns.ts";
import type { TokenName } from "./tokens.ts";

/** A column of the issue list and the width a person gave it; none is the width the theme file gives. */
export interface ListColumnSpec {
  readonly column: ListColumn;
  readonly width?: number | undefined;
}

/** The token of the usual width of each column; the title takes the room that is left. */
const widthToken = {
  select: "size-control",
  status: "size-col-icon",
  key: "size-col-key",
  project: "size-col-project",
  type: "size-col-icon",
  title: undefined,
  labels: "size-col-labels",
  priority: "size-col-priority",
  person: "size-col-person",
  value: "size-col-value",
  changed: "size-col-changed",
  action: "size-col-icon",
} as const satisfies Readonly<Record<ListColumn, TokenName | undefined>>;

/** The columns whose width is a most: they are narrower where the list has no room for it. */
const shrinks: ReadonlySet<ListColumn> = new Set([
  "project",
  "labels",
  "priority",
  "person",
  "value",
  "changed",
]);

function variable(token: TokenName): string {
  return `var(--${token})`;
}

/**
 * The columns of the head and of every row as one `grid-template-columns`: the width a person gave
 * a column, or the one the theme file gives its kind, and the title takes what is left.
 */
export function listTemplate(columns: readonly ListColumnSpec[]): string {
  return columns
    .map(({ column, width }) => {
      if (width !== undefined) {
        return `${String(width)}px`;
      }
      if (column === "title") {
        return `minmax(${variable("size-col-title")}, 1fr)`;
      }
      const token = widthToken[column];
      if (token === undefined) {
        return variable("size-col-title");
      }
      // The columns of fields give up room, down to the least, before the title or another column does.
      return shrinks.has(column)
        ? `minmax(${variable("size-col-least")}, ${variable(token)})`
        : variable(token);
    })
    .join(" ");
}

/** The size in pixels of a length token of the theme file, as the page has it; none where there is no page. */
export function pixelsOf(token: TokenName): number {
  if (typeof document === "undefined") {
    return 0;
  }
  const written = getComputedStyle(document.documentElement).getPropertyValue(`--${token}`);
  const found = Number(/[\d.]+/u.exec(written)?.[0]);
  if (Number.isNaN(found)) {
    return 0;
  }
  return found;
}
