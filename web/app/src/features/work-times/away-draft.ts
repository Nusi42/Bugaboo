import { useState } from "react";

import type { Away, AwayDays } from "./work-times.ts";

/** How the first or the last day of an away time is taken: whole, or half. */
export type HalfChoice = "whole" | "half";

/** The values of the window of an away time while it is filled in, and the days they make. */
export interface AwayDraft {
  readonly first: string;
  readonly setFirst: (day: string) => void;
  readonly last: string;
  readonly setLast: (day: string) => void;
  readonly firstHalf: HalfChoice;
  readonly setFirstHalf: (choice: HalfChoice) => void;
  readonly lastHalf: HalfChoice;
  readonly setLastHalf: (choice: HalfChoice) => void;
  readonly note: string;
  readonly setNote: (note: string) => void;
  /** The days as the request names them; the note only when the window has one. */
  readonly days: (withNote: boolean) => AwayDays;
}

/** The values of the window, starting from the away time that is changed, or empty for a new one. */
export function useAwayDraft(changing: Away | undefined): AwayDraft {
  const [first, setFirst] = useState(changing?.first ?? "");
  const [last, setLast] = useState(changing?.last ?? "");
  const [firstHalf, setFirstHalf] = useState<HalfChoice>(
    changing?.first_afternoon_only === true ? "half" : "whole",
  );
  const [lastHalf, setLastHalf] = useState<HalfChoice>(
    changing?.last_morning_only === true ? "half" : "whole",
  );
  const [note, setNote] = useState(changing?.note ?? "");
  return {
    first,
    setFirst,
    last,
    setLast,
    firstHalf,
    setFirstHalf,
    lastHalf,
    setLastHalf,
    note,
    setNote,
    days: (withNote) => ({
      first,
      // One day is typed once: an empty *To* is the same day.
      last: last === "" ? first : last,
      first_afternoon_only: firstHalf === "half",
      last_morning_only: lastHalf === "half",
      ...(withNote ? { note: note.trim() === "" ? null : note } : {}),
    }),
  };
}
