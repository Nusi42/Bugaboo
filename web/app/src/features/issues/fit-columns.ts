import { pixelsOf } from "@bugaboo/ui";

import { staysColumns, cellOf, type ColumnCell } from "../../shared/list-columns.ts";

/**
 * The width, in pixels, each kind of column needs to show its name and its words whole. A column of
 * a field that has no such room is dropped (see `columnsThatFit`) before it is squeezed to a stump.
 */
const leastOf: Readonly<Record<ColumnCell, () => number>> = {
  status: () => pixelsOf("size-col-icon"),
  key: () => pixelsOf("size-col-key"),
  // The title is what people read: a third more than the least of a column of a field.
  title: () => (pixelsOf("size-col-title") * 4) / 3,
  priority: () => pixelsOf("size-col-priority") + pixelsOf("size-icon-small"),
  type: () => pixelsOf("size-col-project"),
  labels: () => pixelsOf("size-col-labels"),
  person: () => pixelsOf("size-col-person"),
  project: () => pixelsOf("size-col-project"),
  value: () => pixelsOf("size-col-value"),
  changed: () => pixelsOf("size-col-changed"),
};

/** The widths the list needs at the least for these columns: the tick box, the columns, the gaps, the edge and the padding. */
export function leastWidth(words: readonly string[], ticks: boolean): number {
  const cells = words.map((word) => leastOf[cellOf(word)]());
  const count = cells.length + (ticks ? 1 : 0);
  const box = ticks ? pixelsOf("size-control") : 0;
  const gaps = Math.max(0, count - 1) * pixelsOf("space-3");
  const around = pixelsOf("size-edge") + 2 * pixelsOf("pad-box");
  return cells.reduce((sum, width) => sum + width, box + gaps + around);
}

/**
 * The columns that fit in `room` pixels (UX §4.2): where the list has no room for all of them at
 * their least, the columns of fields are dropped from the right, one by one, until it has; the
 * status, the ID, the title and the priority stay. Where even those do not fit the list scrolls.
 * No room measured yet keeps every column.
 */
export function columnsThatFit(
  words: readonly string[],
  room: number | undefined,
  ticks: boolean,
): readonly string[] {
  if (room === undefined) {
    return words;
  }
  const kept = [...words];
  for (let at = kept.length - 1; at >= 0 && leastWidth(kept, ticks) > room; at -= 1) {
    if (!staysColumns.has(kept[at] ?? "")) {
      kept.splice(at, 1);
    }
  }
  return kept;
}
