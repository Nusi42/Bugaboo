import type { ReactNode } from "react";

import { iconColumns, listColumns, type ListColumn } from "./list-columns.ts";
import { partOf } from "./parts.ts";
import { iconSize, type ListRowLook } from "./props.ts";
import { rowStates, type RowStateInput } from "./row-state.ts";
import type { StatusGroup } from "./status.ts";
import { StatusIcon } from "./StatusIcon.tsx";
import { ColumnHeaderCell, GridCell, RowBox, RowHeaderText, TableBox } from "./ListRoles.tsx";
import { part } from "./tokens.ts";

export interface ListGridProps {
  /** The name of the list, for screen readers. */
  readonly label: string;
  readonly children: ReactNode;
}

/** The table that holds the header, the groups and the rows of an issue list. */
export function ListGrid({ label, children }: ListGridProps) {
  return (
    <TableBox look={partOf(part.listGrid)} label={label}>
      {children}
    </TableBox>
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
    <RowBox look={partOf(part.listRow, { variants: ["head"] })}>
      {columns.map((column) => (
        <ColumnHeaderCell key={column} look={partOf(part.listRowCell, { variants: [column] })}>
          {iconColumns.includes(column) ? (
            <span {...partOf(part.visuallyHidden)}>{labels[column]}</span>
          ) : (
            labels[column]
          )}
        </ColumnHeaderCell>
      ))}
    </RowBox>
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
  /** The row of a phone or of the slim list has two lines, in the same order (R9). */
  readonly variant?: ListRowLook;
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
    <RowBox
      look={partOf(part.listRow, {
        variants: [variant],
        states: [...rowStates({ group, priority }), selected && "selected", current && "current"],
      })}
      selected={selected}
    >
      {columns.map((column) => (
        <GridCell key={column} look={partOf(part.listRowCell, { variants: [column] })}>
          {cells[column]}
        </GridCell>
      ))}
    </RowBox>
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
    <RowBox look={partOf(part.listGroup)}>
      <StatusIcon group={group} size={iconSize.small} />
      <RowHeaderText>{label}</RowHeaderText>
      <span {...partOf(part.listGroupCount)}>{count}</span>
    </RowBox>
  );
}
