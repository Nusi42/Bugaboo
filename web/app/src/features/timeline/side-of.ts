import { listWord } from "../../shared/list-columns.ts";
import type { TimelineOptions } from "../../shared/list-view.ts";
import { keptDates, type DateFields, type KeptDates } from "./date-fields.ts";
import type { Pair } from "./dependencies.ts";
import type { BarDays, IssueLine, Line } from "./lines.ts";
import type { TimelineRow } from "./timeline-data.ts";
import type { TimelineSideProps } from "./TimelineSide.tsx";
import type { WarningsListProps } from "./warnings/WarningsList.tsx";

/** What the side of the chart is made from. */
export interface SideInput {
  /** The row of the selected bar, and its line on the chart. */
  readonly row: TimelineRow | undefined;
  readonly line: Line | undefined;
  /** Whether the selected issue has a copy among the lines (*Group by person*). */
  readonly copied: boolean;
  readonly fields: DateFields | undefined;
  readonly daysOf: (line: IssueLine) => BarDays | undefined;
  readonly options: TimelineOptions;
  /** The names of the fields the person may read, by their words. */
  readonly names: Readonly<Record<string, string>>;
  readonly tray: TimelineSideProps["tray"];
  /** The list of plan warnings while it is open. */
  readonly warnings: Omit<WarningsListProps, "onGive" | "onLonger"> | undefined;
  readonly changes: {
    readonly onSet: (row: TimelineRow, dates: KeptDates) => void;
    readonly onPair: (pair: Pair) => void;
  };
}

/** What the side of the chart shows: the selected bar, the tray, the names of the date fields. */
export function sideOf(input: SideInput): Omit<TimelineSideProps, "asked" | "onAsk" | "phone"> {
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
            copied: input.copied,
            days: line?.kind === "issue" ? input.daysOf(line) : undefined,
            mayMove: row.may_move && fields !== undefined,
          },
    tray: input.tray,
    warnings:
      input.warnings === undefined
        ? undefined
        : {
            ...input.warnings,
            // *Make it longer* keeps the start and saves the due date the warning names, through the
            // queue of the screen, with *Undo*.
            onLonger: (longer, due) => {
              const kept = fields === undefined ? undefined : keptDates(longer, fields);
              if (kept !== undefined) {
                input.changes.onSet(longer, { start: kept.start, end: due });
              }
            },
          },
    dateNames:
      fields === undefined ? undefined : { start: names[start] ?? start, end: names[end] ?? end },
    keptOf: (asked) => (fields === undefined ? undefined : keptDates(asked, fields)),
    onSet: input.changes.onSet,
    onPair: input.changes.onPair,
  };
}
