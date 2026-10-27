import { useLayoutEffect, useRef, type ReactNode } from "react";

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

/** The box around the head and the rows of an issue list: it holds the columns they share. */
export function ListFrame({ columns, compact, children }: ListFrameProps) {
  const frame = useRef<HTMLDivElement>(null);
  const template = listTemplate(columns);
  useLayoutEffect(() => {
    frame.current?.style.setProperty(variableOf("list-columns"), template);
  }, [template]);
  return (
    <div ref={frame} {...partOf(part.listFrame, { variants: [compact && "compact"] })}>
      {children}
    </div>
  );
}

export interface ListCellProps {
  readonly column: ListColumn;
  readonly children?: ReactNode;
}

/** One cell of a row, in the column it belongs to (R9: the same things stand in the same places). */
export function ListCell({ column, children }: ListCellProps) {
  return <div {...partOf(part.listRowCell, { variants: [column] })}>{children}</div>;
}

export interface ListColumnHeadProps {
  readonly column: ListColumn;
  /** The name of the column. */
  readonly label: string;
  /** Names the button for screen readers: *Sort by Title*. */
  readonly sortLabel: string;
  /** Which way an arrow beside the name points; none when the list is not sorted by the column. */
  readonly arrow: "up" | "down" | null;
  readonly onSort: () => void;
  readonly resize: ColumnResize | null;
}

/** The name of a column above the rows: pressing it sorts by it, dragging its edge sets its width. */
export function ListColumnHead({
  column,
  label,
  sortLabel,
  arrow,
  onSort,
  resize,
}: ListColumnHeadProps) {
  const cell = useRef<HTMLDivElement>(null);
  const handle = useColumnResize(cell, resize ?? undefined);
  return (
    <div ref={cell} {...partOf(part.listRowCell, { variants: [column] })}>
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
        <button
          type={buttonType.button}
          {...partOf(part.listHandle)}
          aria-label={resize.label}
          {...handle}
        />
      )}
    </div>
  );
}
