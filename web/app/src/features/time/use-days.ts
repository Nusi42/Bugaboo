import type { Schema } from "@bugaboo/api-client";
import type { Wording } from "@bugaboo/wasm";
import { useState } from "react";

import { useRegion } from "../../shared/region.ts";
import { useNow } from "../../shared/use-now.ts";
import { changedDates, daysByDate, todayOf, warningsOfDays, type WorkDay } from "./sheet-model.ts";
import type { DaysProps } from "./sheet-props.ts";
import { useSheetEditing } from "./use-sheet-editing.ts";
import { rangeOf, useDayHistory, useEntriesOnDay, useWorkDays } from "./use-week.ts";

type History = Schema<"WorkDayHistoryBody">;
type WorkDaysPage = Schema<"WorkDaysBody">;

export interface DaysInput {
  readonly wording: Wording;
  readonly first: string;
  /** The ID of another person whose work days are shown; none for one's own week. */
  readonly person?: string | undefined;
  /** For another person's week: whether the reader chose *Change*. One's own week is always open. */
  readonly changing?: boolean | undefined;
}

/** What the rows of a week are drawn from. */
export interface DaysState {
  readonly days: DaysProps;
  readonly page: WorkDaysPage | undefined;
  /** Why the days could not be read, if they could not. */
  readonly error: unknown;
  readonly byDate: Readonly<Record<string, WorkDay>>;
  /** The history of the day that is open. */
  readonly history: History | undefined;
  /** The date whose history is open. */
  readonly shown: string | undefined;
  readonly setShown: (date: string | undefined) => void;
}

/** The days of a week as the rows draw them: what the server has, who may type, what changed. */
export function useDays(input: DaysInput): DaysState {
  const { wording, first, person, changing = false } = input;
  const region = useRegion();
  const now = useNow();
  const workDays = useWorkDays(first, person);
  const page = workDays.data;
  const byDate = daysByDate(page?.days);
  const today = todayOf(region, now);
  const entriesOf = useEntriesOnDay(first);
  const editing = useSheetEditing({
    person,
    region,
    today,
    dayOf: (date) => byDate[date],
    entriesOf,
  });
  const [shown, setShown] = useState<string | undefined>();
  const history = useDayHistory(shown, person);
  const mayChange = changing && page?.may_change === true && !page.totals_only;
  const days: DaysProps = {
    wording,
    region,
    days: rangeOf(first).days,
    today,
    dayOf: (date) => byDate[date],
    warned: warningsOfDays(rangeOf(first).days, byDate),
    editable: (date) => person === undefined || (mayChange && date in byDate),
    editing,
    worked: page?.totals?.worked,
    // The server says which days were changed with a reason.
    changed: changedDates(page?.days),
    onHistory: setShown,
  };
  return { days, page, error: workDays.error, byDate, history, shown, setShown };
}
