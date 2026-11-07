import { useState } from "react";

import { personToday } from "../../shared/clock.ts";
import { useRegion } from "../../shared/region.ts";
import { startingValues, type EntryValues } from "./entry-values.ts";
import type { EntryDefaults } from "./use-entry-defaults.ts";

/** What *Add time* has been given so far. */
export interface AddTimeState {
  /** The date the time belongs to (`2027-01-14`); the person's today at first. */
  readonly date: string;
  readonly setDate: (date: string) => void;
  /** The length in whole minutes; none until one is typed. */
  readonly minutes: number | null;
  readonly setMinutes: (minutes: number | null) => void;
  readonly values: EntryValues;
  readonly setValues: (values: EntryValues) => void;
  /** Whether there is a date and a length to book. */
  readonly ready: boolean;
}

/** The values of *Add time*: today, no length yet, and the defaults of the issue. */
export function useAddTimeState(defaults: EntryDefaults): AddTimeState {
  const region = useRegion();
  const [date, setDate] = useState(() => personToday(region, Date.now()));
  const [minutes, setMinutes] = useState<number | null>(null);
  const [values, setValues] = useState<EntryValues>(() => startingValues(defaults));
  return {
    date,
    setDate,
    minutes,
    setMinutes,
    values,
    setValues,
    ready: minutes !== null && minutes > 0 && date !== "",
  };
}
