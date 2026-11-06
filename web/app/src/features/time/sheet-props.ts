import type { Wording } from "@bugaboo/wasm";

import type { Region } from "../../shared/region.ts";
import type { WarnedDay, WorkDay } from "./sheet-model.ts";
import type { SheetEditing } from "./use-sheet-editing.ts";
import type { IssueRow } from "./rows.ts";

/** How many columns the names of the rows take (an issue, its *Activity* and its *Cost center*). */
export const NAME_COLUMNS = 3;

/** What the rows of the work day need to be drawn and typed into, for a computer and for a phone. */
export interface DaysProps {
  readonly wording: Wording;
  readonly region: Region;
  /** The seven dates of the week, in order. */
  readonly days: readonly string[];
  /** The person's date of today in their zone. */
  readonly today: string;
  readonly dayOf: (date: string) => WorkDay | undefined;
  /** The sentences of the warnings of the days, as the server words them. */
  readonly warned: readonly WarnedDay[];
  /** Whether the work-day cells of a date take a value (a read-only view, or *Totals only*, says no). */
  readonly editable: (date: string) => boolean;
  readonly editing: SheetEditing;
  /** The time worked in the week, as the server adds it up. */
  readonly worked: number | undefined;
  /** The dates whose day was changed with a reason: they show *Changed*. */
  readonly changed: readonly string[];
  readonly onHistory: (date: string) => void;
}

/** The comparison of the week with the time booked on issues (one's own week only). */
export interface BookedDay {
  readonly booked: number;
  readonly notBooked: number;
}

/** What the time on issues need besides the days. */
export interface IssuesProps {
  readonly rows: readonly IssueRow[];
  /** Why the issue the person picked could not be added; none while it could. */
  readonly addProblem?: string | undefined;
  readonly activityName: string;
  readonly costCenterName: string;
  readonly booked: Readonly<Record<string, BookedDay>>;
  /** The week, as the server adds up the figures of its days. */
  readonly week: BookedDay | undefined;
  readonly onAdd: (key: string) => void;
}
