import { createContext, useContext } from "react";

import type { ZoomWord } from "../../shared/list-view.ts";
import type { DateFields } from "./date-fields.ts";
import type { Move } from "./moves.ts";
import type { BarDays, IssueLine } from "./lines.ts";
import type { Span } from "./scale.ts";
import type { TimelineGoal, TimelineRow } from "./timeline-data.ts";

/** What every part of the chart shares: where days and rows stand, and what a person does to a bar. */
export interface TimelineContextValue {
  readonly span: Span;
  readonly zoom: ZoomWord;
  /** The width of one day and the height of one row, in pixels, as the theme file draws them. */
  readonly dayPixels: number;
  readonly rowPixels: number;
  /** The date fields of the view; none while the own fields it names are read. */
  readonly fields: DateFields | undefined;
  /** The days a bar shows: the server's, or the ones a change asked for until the server answers. */
  readonly daysOf: (line: IssueLine) => BarDays | undefined;
  /** The goal an issue of the page is, with its due date and the days of its sub-issues. */
  readonly goalOf: (issue: string) => TimelineGoal | undefined;
  /** The issue whose bar is selected. */
  readonly selected: string | undefined;
  /** The issue whose warning is pointed at in the list: its bar is marked. */
  readonly marked: string | undefined;
  /** How many open plan warnings an issue has: its bar carries the sign. */
  readonly warnedOf: (issue: string) => number;
  readonly select: (issue?: string) => void;
  /** A bar was moved, or one of its ends, by `days`: a drag that ended, saved at once. */
  readonly onMove: (row: TimelineRow, move: Move, days: number) => void;
  /** A key moved a bar: the presses of a run are joined into one change (M8-F84). */
  readonly onKeyMove: (row: TimelineRow, move: Move, days: number) => void;
  /** The keys of a bar were let go: its run is saved after a short pause, unless a key comes. */
  readonly onRunPause: (issue: string) => void;
  /** The bar lost the focus: its run is saved now. */
  readonly onRunEnd: (issue: string) => void;
  /** A bar that may not be moved was asked to: the screen says why. */
  readonly onLocked: (row: TimelineRow) => void;
  /** A dependency was dragged from the dot of `first` onto the bar of `then`. */
  readonly onLink: (first: string, then: string) => void;
  /** An issue of *Not scheduled yet* was dropped on a day of the chart. */
  readonly onPlace: (issue: string, day: string) => void;
}

export const TimelineContext = createContext<TimelineContextValue | null>(null);

/** What the chart shares, for a part of it. */
export function useTimeline(): TimelineContextValue {
  const value = useContext(TimelineContext);
  if (value === null) {
    throw new Error("A part of the timeline is drawn outside the timeline.");
  }
  return value;
}
