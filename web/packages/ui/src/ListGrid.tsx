import type { ReactNode } from "react";

import { iconColumns, listColumns, type ListColumn } from "./list-columns.ts";
import { partOf } from "./parts.ts";
import { iconSize, type listRowLook } from "./props.ts";
import { rowStates, type RowStateInput } from "./row-state.ts";
import type { StatusGroup } from "./status.ts";
import { StatusIcon } from "./StatusIcon.tsx";
import { part } from "./tokens.ts";

/** Words of the ARIA standard, not text for people. */
const role = {
  table: "table",
  row: "row",
  cell: "cell",
  columnHeader: "columnheader",
  rowHeader: "rowheader",
} as const;

export interface ListGridProps {
  /** The name of the list, for screen readers. */
  readonly label: string;
  readonly children: ReactNode;
}

/** The table that holds the header, the groups and the rows of an issue list. */
export function ListGrid({ label, children }: ListGridProps) {
  return (
    <div {...partOf(part.listGrid)} role={role.table} aria-label={label}>
      {children}
    </div>
  );
}

export interface ListHeadProps {
  /** The name of every column. The columns with only an icon are named for screen readers only. */
  readonly labels: Readonly<Record<ListColumn, string>>;
  /** The columns the list has, in the order they stand; all of them without it. */
  readonly columns?: readonly ListColumn[];
}

/** The names of the columns above the rows: the same cells, so they line up. */
export function ListHead({ labels, columns = listColumns }: ListHeadProps) {
  return (
    <div {...partOf(part.listRow, { variants: ["head"] })} role={role.row}>
      {columns.map((column) => (
        <div
          key={column}
          {...partOf(part.listRowCell, { variants: [column] })}
          role={role.columnHeader}
        >
          {iconColumns.includes(column) ? (
            <span {...partOf(part.visuallyHidden)}>{labels[column]}</span>
          ) : (
            labels[column]
          )}
        </div>
      ))}
    </div>
  );
}

export interface ListRowProps extends RowStateInput {
  /** What stands in each column. A column without content stays, so the others keep their place. */
  readonly cells: Readonly<Partial<Record<ListColumn, ReactNode>>>;
  /** The columns the list has, in the order they stand; all of them without it. */
  readonly columns?: readonly ListColumn[];
  /** The row is ticked. */
  readonly selected?: boolean;
  /** The issue of this row is open next to the list. */
  readonly current?: boolean;
  /** The row of a phone has two lines, in the same order (R9). */
  readonly variant?: typeof listRowLook.phone;
}

/** One issue: its cells in the fixed columns, tinted by its status group and edged by its priority (R4). */
export function ListRow({
  cells,
  columns = listColumns,
  group,
  priority,
  selected = false,
  current = false,
  variant,
}: ListRowProps) {
  return (
    <div
      {...partOf(part.listRow, {
        variants: [variant],
        states: [...rowStates({ group, priority }), selected && "selected", current && "current"],
      })}
      role={role.row}
      aria-selected={selected}
      {...(current ? { "aria-current": true } : {})}
    >
      {columns.map((column) => (
        <div key={column} {...partOf(part.listRowCell, { variants: [column] })} role={role.cell}>
          {cells[column]}
        </div>
      ))}
    </div>
  );
}

export interface ListGroupProps {
  readonly group: StatusGroup;
  /** The name of the group of statuses. */
  readonly label: string;
  /** How many issues the group holds. */
  readonly count: number;
}

/** The header of a group of rows (*Group by status*): the group's icon and color, its name and its count. */
export function ListGroup({ group, label, count }: ListGroupProps) {
  return (
    <div {...partOf(part.listGroup)} role={role.row}>
      <StatusIcon group={group} size={iconSize.small} />
      <span role={role.rowHeader}>{label}</span>
      <span {...partOf(part.listGroupCount)}>{count}</span>
    </div>
  );
}
