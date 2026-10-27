import { useEffect, useLayoutEffect, useRef, type ReactNode } from "react";

import { Icon } from "./Icon.tsx";
import type { ColumnResize } from "./use-column-resize.ts";
import { iconColumns, type ListColumn } from "./list-columns.ts";
import { listMinWidth, listTemplate, type ListColumnSpec } from "./list-template.ts";
import { CellGroup, ColumnHeaderCell } from "./ListRoles.tsx";
import { partOf } from "./parts.ts";
import { buttonType, iconSize } from "./props.ts";
import { part } from "./tokens.ts";
import { variableOf } from "./variables.ts";

export interface ListFrameProps {
  readonly columns: readonly ListColumnSpec[];
  /** *Spacing: Compact*. */
  readonly compact: boolean;
  /** The columns have the widths they were given (a head over rows), not the card of a phone. */
  readonly fixed: boolean;
  readonly children: ReactNode;
}

/**
 * Keeps the room of the bar the rows have at the end of the head, so the columns stand over each
 * other. Only the box of the rows and the first thing in it are measured, not what is drawn in it.
 */
function keepGutterOfRows(frame: HTMLElement): () => void {
  let box: HTMLElement | null = null;
  let gutter = "";
  const sync = (): void => {
    if (box === null) {
      return;
    }
    const now = `${String(box.offsetWidth - box.clientWidth)}px`;
    if (now !== gutter) {
      gutter = now;
      frame.style.setProperty(variableOf("list-gutter"), now);
    }
  };
  const watching = new ResizeObserver(sync);
  const attach = (): void => {
    const found = frame.querySelector<HTMLElement>(`[data-part="${part.listScroll}"]`);
    if (found === box) {
      return;
    }
    watching.disconnect();
    box = found;
    if (box !== null) {
      watching.observe(box);
      if (box.firstElementChild !== null) {
        watching.observe(box.firstElementChild);
      }
    }
    sync();
  };
  const changes = new MutationObserver(attach);
  changes.observe(frame, { childList: true, subtree: true });
  attach();
  return () => {
    watching.disconnect();
    changes.disconnect();
  };
}

/** The box around the head and the rows of an issue list: it holds the columns they share. */
export function ListFrame({ columns, compact, fixed, children }: ListFrameProps) {
  const frame = useRef<HTMLDivElement>(null);
  const template = listTemplate(columns);
  const needed = fixed ? `${String(listMinWidth(columns))}px` : "0px";
  useLayoutEffect(() => {
    frame.current?.style.setProperty(variableOf("list-columns"), template);
    frame.current?.style.setProperty(variableOf("list-width"), needed);
  }, [template, needed]);
  useEffect(() => (frame.current === null ? undefined : keepGutterOfRows(frame.current)), []);
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
  const look = partOf(part.listRowCell, { variants: [column] });
  if (head) {
    return <ColumnHeaderCell look={look}>{children}</ColumnHeaderCell>;
  }
  if (headedBy === undefined) {
    return <div {...look}>{children}</div>;
  }
  return (
    <CellGroup look={look} labelledBy={headedBy}>
      {children}
    </CellGroup>
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
  return (
    <ColumnHeaderCell
      resize={resize === null || iconColumns.includes(column) ? null : resize}
      id={id}
      sort={sortOf(arrow)}
      label={label}
      look={partOf(part.listRowCell, { variants: [column] })}
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
    </ColumnHeaderCell>
  );
}
