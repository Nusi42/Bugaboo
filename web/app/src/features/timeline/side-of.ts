import { listWord } from "../../shared/list-columns.ts";
import type { TimelineOptions } from "../../shared/list-view.ts";
import { keptDates, type DateFields, type KeptDates } from "./date-fields.ts";
import type { Pair } from "./dependencies.ts";
import type { BarDays, IssueLine, Line } from "./lines.ts";
import type { TimelineRow } from "./timeline-data.ts";
import type { TimelineSideProps } from "./TimelineSide.tsx";

/** What the side of the chart is made from. */
export interface SideInput {
  /** The row of the selected bar, and its line on the chart. */
  readonly row: TimelineRow | undefined;
  readonly line: Line | undefined;
  readonly fields: DateFields | undefined;
  readonly daysOf: (line: IssueLine) => BarDays | undefined;
  readonly options: TimelineOptions;
  /** The names of the fields the person may read, by their words. */
  readonly names: Readonly<Record<string, string>>;
  readonly tray: TimelineSideProps["tray"];
  readonly changes: {
    readonly onSet: (row: TimelineRow, dates: KeptDates) => void;
    readonly onPair: (pair: Pair) => void;
  };
}

/** What the side of the chart shows: the selected bar, the tray, the names of the date fields. */
export function sideOf(input: SideInput): Omit<TimelineSideProps, "asked" | "onAsk"> {
  const { row, line, fields, names, options } = input;
  const [start, end] = [
    options.start_field ?? listWord.startDate,
    options.end_field ?? listWord.dueDate,
  ];
  return {
    selected:
      row === undefined
        ? undefined
        : {
            row,
            days: line?.kind === "issue" ? input.daysOf(line) : undefined,
            mayMove: row.may_move && fields !== undefined,
          },
    tray: input.tray,
    dateNames:
      fields === undefined ? undefined : { start: names[start] ?? start, end: names[end] ?? end },
    keptOf: (asked) => (fields === undefined ? undefined : keptDates(asked, fields)),
    onSet: input.changes.onSet,
    onPair: input.changes.onPair,
  };
}
