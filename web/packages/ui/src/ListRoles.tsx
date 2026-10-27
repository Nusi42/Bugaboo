import { useRef, type ReactNode } from "react";

import { partOf, type PartAttributes } from "./parts.ts";
import { part } from "./tokens.ts";
import {
  useColumnResize,
  type ColumnResize,
  type HandleEvents,
  type HandleValue,
} from "./use-column-resize.ts";

// The head and the cells of the issue list are boxes in a grid of divs that is tied to the rows of
// the virtualized GridList (UX §4.2, TEST-A11Y): table, tr, th and hr cannot be used, because the
// head must share the `grid-template-columns` of the rows and a cell stands in a row of a GridList.
// So each role is given once, here, as an attribute, and every use goes through these components.

/** The roles, named once: they are not text. */
const tableRole = "table";
const rowRole = "row";
const headerRole = "columnheader";
const groupRole = "group";
const separatorRole = "separator";

/** The head of the list: a table of one row of column headers. */
export function HeadTable({
  look,
  label,
  children,
}: {
  readonly look: PartAttributes;
  readonly label: string;
  readonly children: ReactNode;
}) {
  return (
    // Role by attribute, not table/tr/th/hr: UX §4.2 and TEST-A11Y: a virtualized grid of divs cannot use table; the head shares the grid columns of the rows of the GridList
    <div {...look} role={tableRole} aria-label={label}>
      {children}
    </div>
  );
}

/** The one row of the head. */
export function HeadRow({
  look,
  children,
}: {
  readonly look: PartAttributes;
  readonly children: ReactNode;
}) {
  return (
    // Role by attribute, not table/tr/th/hr: UX §4.2 and TEST-A11Y: a virtualized grid of divs cannot use tr; the head row shares the grid columns of the rows of the GridList
    <div {...look} role={rowRole}>
      {children}
    </div>
  );
}

/** A column header: the name of a column and the order it sorts in. */
export function ColumnHeaderCell({
  look,
  id,
  label,
  sort,
  resize,
  children,
}: {
  readonly look: PartAttributes;
  readonly id?: string | undefined;
  readonly label?: string | undefined;
  readonly sort?: "ascending" | "descending" | "none" | undefined;
  /** How the width is changed with the edge; none where the column has no edge. */
  readonly resize?: ColumnResize | null | undefined;
  readonly children: ReactNode;
}) {
  const cell = useRef<HTMLDivElement>(null);
  const handle = useColumnResize(cell, resize ?? undefined);
  return (
    // Role by attribute, not table/tr/th/hr: UX §4.2 and TEST-A11Y: a virtualized grid of divs cannot use th; a header cell stands in the grid columns of the rows of the GridList
    <div {...look} ref={cell} id={id} role={headerRole} aria-sort={sort} aria-label={label}>
      {children}
      {resize === null || resize === undefined ? null : (
        <ColumnSeparator
          look={partOf(part.listHandle)}
          label={resize.label}
          value={handle.value}
          events={handle.events}
        />
      )}
    </div>
  );
}

/** A cell of a row, named by the header of its column. */
export function CellGroup({
  look,
  labelledBy,
  children,
}: {
  readonly look: PartAttributes;
  readonly labelledBy: string;
  readonly children: ReactNode;
}) {
  return (
    // Role by attribute, not table/tr/th/hr: UX §4.2 and TEST-A11Y: a cell of a row of the virtualized GridList is a div in the grid columns of the row, and fieldset or details would change how it is laid out and read
    <div {...look} role={groupRole} aria-labelledby={labelledBy}>
      {children}
    </div>
  );
}

const vertical = "vertical";

/** The edge between two column headers: a separator whose value is the width. */
export function ColumnSeparator({
  look,
  label,
  value,
  events,
}: {
  readonly look: PartAttributes;
  readonly label: string;
  readonly value: HandleValue;
  readonly events: HandleEvents;
}) {
  return (
    <div
      {...look}
      {...events}
      // Role by attribute, not table/tr/th/hr: UX §4.2 and TEST-A11Y: a separator that is focused and has a value is a widget (the width of a column), which hr cannot be
      role={separatorRole}
      aria-orientation={vertical}
      tabIndex={0}
      aria-label={label}
      aria-valuenow={value.now}
      aria-valuemin={value.least}
      aria-valuemax={value.most}
    />
  );
}
