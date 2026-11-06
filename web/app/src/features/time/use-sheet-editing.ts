import { t } from "@bugaboo/messages";
import type { SaveState } from "@bugaboo/ui";

import { planIssueCell, type BreakTimes, type WorkDayField } from "./edits.ts";
import type { Committed } from "./TimeCell.tsx";
import { applyEntryChange } from "./use-changes.ts";
import {
  problemOf,
  useDayCells,
  type EditingInput,
  type PendingReason,
  type Saving,
} from "./use-day-cells.ts";
import { useSaving } from "./use-saving.ts";

/** What typing into a cell does, as the cells and the window of the reason use it. */
export interface SheetEditing {
  /** *Saving…*, *Saved* or *Not saved yet*. */
  readonly state: SaveState;
  /** Counts up when a typed value is given up, so the cells show the saved values again. */
  readonly revision: number;
  /** The change of a day that waits for its reason. */
  readonly pending: PendingReason | undefined;
  readonly editDay: (
    date: string,
    field: WorkDayField,
    typed: number | BreakTimes | undefined,
  ) => Promise<Committed>;
  readonly editIssue: (
    issue: string,
    date: string,
    minutes: number | undefined,
  ) => Promise<Committed>;
  /** Saves the change that waited for its reason; the sentence of why not, or nothing. */
  readonly confirmReason: (reason: string) => Promise<string | undefined>;
  /** Gives up the change that waited for its reason: the cells show the saved values again. */
  readonly cancelReason: () => void;
}

/**
 * The cells of the issues: book, change or take back the time of a cell. A cell is planned in its
 * turn, on the entries the server has then, so it never books twice or changes an old version.
 */
function useIssueCells(
  saving: Pick<Saving, "inLine" | "saveNow">,
  entriesOf: EditingInput["entriesOf"],
) {
  return (issue: string, date: string, minutes: number | undefined): Promise<Committed> =>
    saving.inLine(async () => {
      const planned = planIssueCell({ issue, date, entries: entriesOf(issue, date), minutes });
      if (planned.kind === "several") {
        return { problem: t("time.sheet.several-entries") };
      }
      return planned.kind === "none"
        ? { unchanged: true }
        : problemOf(await saving.saveNow(() => applyEntryChange(planned)));
    });
}

/**
 * What typing into a cell does: the work-day cells plan new times of the day and save them (after
 * the reason, where one is asked for), the cells of an issue book, change or take back time. A
 * cell is told the sentence of what went wrong, and the page shows *Saving…* and *Saved*.
 */
export function useSheetEditing(input: EditingInput): SheetEditing {
  const saving = useSaving();
  const editIssue = useIssueCells(saving, input.entriesOf);
  const dayCells = useDayCells(input, saving);
  return { state: saving.state, editIssue, ...dayCells };
}
