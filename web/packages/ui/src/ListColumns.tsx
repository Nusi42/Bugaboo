import { useEffect, useLayoutEffect, useRef, type ReactNode } from "react";

import { Icon } from "./Icon.tsx";
import { iconColumns, type ListColumn } from "./list-columns.ts";
import { listTemplate, type ListColumnSpec } from "./list-template.ts";
import { partOf } from "./parts.ts";
import { buttonType, iconSize } from "./props.ts";
import { part } from "./tokens.ts";
import { useColumnResize, type ColumnResize } from "./use-column-resize.ts";
import { variableOf } from "./variables.ts";

export interface ListFrameProps {
  readonly columns: readonly ListColumnSpec[];
  /** *Spacing: Compact*. */
  readonly compact: boolean;
  readonly children: ReactNode;
}

/**
 * Keeps the head over the rows: it scrolls sideways with them (the rows are the one box that
 * scrolls) and keeps the room of the bar the rows have, so the columns stand over each other.
 */
function keepHeadOverRows(frame: HTMLElement): () => void {
  const sync = (): void => {
    const rows = frame.querySelector<HTMLElement>(`[data-part="${part.listScroll}"]`);
    const head = frame.querySelector<HTMLElement>(`[data-part="${part.listHead}"]`);
    if (rows === null || head === null) {
      return;
    }
    head.scrollLeft = rows.scrollLeft;
    const gutter = `${String(rows.offsetWidth - rows.clientWidth)}px`;
    frame.style.setProperty(variableOf("list-gutter"), gutter);
  };
  frame.addEventListener("scroll", sync, true);
  const watching = new ResizeObserver(sync);
  watching.observe(frame);
  const changes = new MutationObserver(sync);
  changes.observe(frame, { childList: true, subtree: true });
  sync();
  return () => {
    frame.removeEventListener("scroll", sync, true);
    watching.disconnect();
    changes.disconnect();
  };
}

/** The box around the head and the rows of an issue list: it holds the columns they share. */
export function ListFrame({ columns, compact, children }: ListFrameProps) {
  const frame = useRef<HTMLDivElement>(null);
  const template = listTemplate(columns);
  useLayoutEffect(() => {
    frame.current?.style.setProperty(variableOf("list-columns"), template);
  }, [template]);
  useEffect(() => (frame.current === null ? undefined : keepHeadOverRows(frame.current)), []);
  return (
    <div ref={frame} {...partOf(part.listFrame, { variants: [compact && "compact"] })}>
      {children}
    </div>
  );
}

export interface ListCellProps {
  readonly column: ListColumn;
  /** The id of the head cell of its column: a screen reader hears the cell as a group named by it. */
  readonly headedBy?: string | undefined;
  /** The cell is a header of the head row (the box that ticks the rows shown). */
  readonly head?: boolean;
  readonly children?: ReactNode;
}

/** One cell of a row, in the column it belongs to (R9: the same things stand in the same places). */
export function ListCell({ column, headedBy, head = false, children }: ListCellProps) {
  return (
    <div
      {...partOf(part.listRowCell, { variants: [column] })}
      {...(head ? columnHeader : {})}
      {...(headedBy === undefined ? {} : { ...cellGroup, "aria-labelledby": headedBy })}
    >
      {children}
    </div>
  );
}

export interface ListColumnHeadProps {
  readonly column: ListColumn;
  /** The name of the column. */
  readonly label: string;
  /** Names the button for screen readers: *Sort by Title*. */
  readonly sortLabel: string;
  /** Which way an arrow beside the name points; none when the list is not sorted by the column. */
  readonly arrow: "up" | "down" | null;
  /** The id the cells below it name their column by. */
  readonly id: string;
  readonly onSort: () => void;
  readonly resize: ColumnResize | null;
}

/**
 * The head and the rows share one grid of boxes, so the roles of the head are given by attribute: a
 * column header names its column (and the way it is sorted), the edge between two of them is a
 * separator with a value, and a cell of a row is a group named by the header of its column.
 */
const columnHeader = { role: "columnheader" } as const;
const resizer = { role: "separator", "aria-orientation": "vertical", tabIndex: 0 } as const;
const cellGroup = { role: "group" } as const;

/** What a screen reader is told of the order a column sorts in. */
function sortOf(arrow: "up" | "down" | null): "ascending" | "descending" | "none" {
  if (arrow === null) {
    return "none";
  }
  return arrow === "up" ? "ascending" : "descending";
}

/** The name of a column above the rows: pressing it sorts by it, dragging its edge sets its width. */
export function ListColumnHead({
  column,
  label,
  sortLabel,
  arrow,
  id,
  onSort,
  resize,
}: ListColumnHeadProps) {
  const cell = useRef<HTMLDivElement>(null);
  const handle = useColumnResize(cell, resize ?? undefined);
  return (
    <div
      ref={cell}
      id={id}
      {...columnHeader}
      aria-sort={sortOf(arrow)}
      aria-label={label}
      {...partOf(part.listRowCell, { variants: [column] })}
    >
      <button
        type={buttonType.button}
        {...partOf(part.listSort)}
        aria-label={sortLabel}
        onClick={onSort}
      >
        {iconColumns.includes(column) ? null : (
          <span {...partOf(part.listSortLabel)} aria-hidden>
            {label}
          </span>
        )}
        {arrow === null ? null : <Icon name={arrow} size={iconSize.small} />}
      </button>
      {resize === null || iconColumns.includes(column) ? null : (
        <div
          {...resizer}
          aria-label={resize.label}
          aria-valuenow={handle.value.now}
          aria-valuemin={handle.value.least}
          aria-valuemax={handle.value.most}
          {...partOf(part.listHandle)}
          {...handle.events}
        />
      )}
    </div>
  );
}
