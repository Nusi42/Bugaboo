import type { ReactNode } from "react";

import { partOf } from "./parts.ts";
import { dataCellVariant, type DataCellVariant } from "./props.ts";
import { part } from "./tokens.ts";

/** Words of the HTML standard, not text for people. */
const scope = { column: "col", row: "row" } as const;

export interface DataTableProps {
  /** The name of the table for screen readers. */
  readonly label: string;
  /**
   * The names of the columns, in order. On a phone the head is not shown and each cell says its name itself. A column
   * whose name is only for screen readers (the buttons of a row) is `{ hidden: name }`.
   */
  readonly columns: readonly (string | { readonly hidden: string })[];
  /** The rows (`DataRow`). */
  readonly children: ReactNode;
}

/**
 * A table of records (the apps a person allowed, the keep and delete rules, the files of people's data): the columns
 * side by side from a tablet on, and on a phone each row a block under the other, whose cells say what they are. The one
 * table of the kind, so every list of records looks and reads the same.
 */
export function DataTable({ label, columns, children }: DataTableProps) {
  return (
    <table {...partOf(part.dataTable)} aria-label={label}>
      <thead {...partOf(part.dataTableHead)}>
        <tr {...partOf(part.dataTableRow)}>
          {columns.map((column) => (
            <th
              key={typeof column === "string" ? column : column.hidden}
              scope={scope.column}
              {...partOf(part.dataTableCell, { variants: ["head"] })}
            >
              {typeof column === "string" ? (
                column
              ) : (
                <span {...partOf(part.visuallyHidden)}>{column.hidden}</span>
              )}
            </th>
          ))}
        </tr>
      </thead>
      <tbody {...partOf(part.dataTableBody)}>{children}</tbody>
    </table>
  );
}

/** One row of a `DataTable`. */
export function DataRow({ children }: { readonly children: ReactNode }) {
  return <tr {...partOf(part.dataTableRow)}>{children}</tr>;
}

/** The first cell of a row, which names the record: the header of its row. */
export function DataRowHead({ children }: { readonly children: ReactNode }) {
  return (
    <th scope={scope.row} {...partOf(part.dataTableCell, { variants: [dataCellVariant.name] })}>
      {children}
    </th>
  );
}

export interface DataCellProps {
  /** What the cell is, said beside its words on a phone, where the head of the table is not shown; none for an action. */
  readonly label?: string;
  readonly variant?: DataCellVariant;
  readonly children: ReactNode;
}

/** A cell of a row of a `DataTable`. */
export function DataCell({ label, variant, children }: DataCellProps) {
  return (
    <td {...partOf(part.dataTableCell, { variants: [variant] })}>
      {label === undefined ? null : <span {...partOf(part.dataTableLabel)}>{label} </span>}
      {children}
    </td>
  );
}
