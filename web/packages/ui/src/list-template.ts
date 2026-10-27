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
        return `minmax(max(${variable("size-col-title")}, ${variable("size-title-share")}), 1fr)`;
      }
      if (column === "value") {
        return `minmax(${variable("size-col-least")}, ${variable("size-col-value")})`;
      }
      const token = widthToken[column];
      return token === undefined ? variable("size-col-title") : variable(token);
    })
    .join(" ");
}

/**
 * The width the columns need together, in pixels: each column, the gaps between them and the room
 * the row keeps at its ends. A list narrower than this scrolls sideways, head and rows together.
 */
export function listMinWidth(columns: readonly ListColumnSpec[]): number {
  const widths = columns.map(({ column, width }) => {
    if (width !== undefined) {
      return width;
    }
    if (column === "title") {
      return Math.max(pixelsOf("size-col-title"), pixelsOf("size-title-share"));
    }
    return pixelsOf(
      column === "value" ? "size-col-least" : (widthToken[column] ?? "size-col-title"),
    );
  });
  const gaps = Math.max(0, columns.length - 1) * pixelsOf("space-2");
  return (
    widths.reduce((sum, each) => sum + each, 0) +
    gaps +
    2 * pixelsOf("pad-box") +
    pixelsOf("size-edge")
  );
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
  // A share of the window is as wide as that part of the window is now.
  return written.trim().endsWith("vw") ? (found * window.innerWidth) / 100 : found;
}
