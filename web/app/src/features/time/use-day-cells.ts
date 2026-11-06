import { t } from "@bugaboo/messages";
import { doNothing } from "@bugaboo/ui";
import { useLayoutEffect, useRef, useState } from "react";

import { codeOf, messageOf } from "../../shared/errors.ts";
import { type Region } from "../../shared/region.ts";
import {
  hasTimes,
  type BreakTimes,
  planWorkDayEdit,
  reasonIsNeeded,
  type TimeEntry,
  type Times,
  type WorkDayField,
} from "./edits.ts";
import { clockText, endOf, startOf, zoneOfDay, type WorkDay } from "./sheet-model.ts";
import type { Committed } from "./TimeCell.tsx";
import { useDayChange } from "./use-changes.ts";
import type { Saved } from "./use-saving.ts";

/** What the server answers when a change needs a reason it was not given. */
const REASON_NEEDED = "working-hours.reason-needed";

/** A change of a work day that waits for its reason. */
export interface PendingReason {
  readonly date: string;
  readonly version: number;
  readonly times: Times;
  /** For a start or an end: what it was and what it becomes, as the person's clock writes them. */
  readonly what?: Change;
}

/** One time of the day that changes. */
export interface Change {
  readonly field: "start" | "end";
  readonly before: string;
  readonly after: string;
}

export interface EditingInput {
  /** The other person whose days are shown, or none for one's own. */
  readonly person: string | undefined;
  readonly region: Region;
  readonly today: string;
  readonly dayOf: (date: string) => WorkDay | undefined;
  /** The entries of an issue on a day as the server has them now. */
  readonly entriesOf: (issue: string, date: string) => readonly TimeEntry[];
}

/** What the cells need of the queue of saves: to put a save in line and to wait for the line. */
export interface Saving {
  readonly inLine: <Result>(job: () => Promise<Result>) => Promise<Result>;
  readonly saveNow: (work: () => Promise<void>) => Promise<Saved>;
}

/** How a save went, as a cell is told. */
export function problemOf(saved: Saved): Committed {
  return saved.ok ? {} : { problem: messageOf(saved.error) };
}

/** Answers once the screen has been drawn with what the server answered last. */
function rendered(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => {
      resolve();
    });
  });
}

/** The starts typed for days that cannot run, by date (minutes after midnight): they wait for their ends. */
type Held = Readonly<Record<string, number>>;

/** The starts that wait, less the one of `date`. */
function withoutDate(held: Held, date: string): Held {
  return Object.fromEntries(
    Object.keys(held)
      .filter((each) => each !== date)
      .map((each) => [each, held[each] ?? 0]),
  );
}

/** What `planDay` knows besides the day: where the person is and which starts wait for an end. */
type Around = Pick<EditingInput, "region" | "dayOf" | "today"> & { readonly held: Held };

/** What a typed cell comes to: a refusal to tell the cell, a start to hold, or the change to save. */
type Planned =
  | { readonly refused: Committed }
  | { readonly change: PendingReason }
  | { readonly held: number };

/** Places what was typed on the day; the browser only places it, the server decides what it means. */
function planDay(
  input: Around,
  date: string,
  field: WorkDayField,
  typed: number | BreakTimes | undefined,
): Planned {
  const current = input.dayOf(date);
  // A cleared *Breaks* cell is no breaks; a start or an end cannot be taken away, a day is not deleted.
  const breaks = field === "breaks" ? (Array.isArray(typed) ? typed : []) : undefined;
  const minutes = typeof typed === "number" ? typed : undefined;
  if (breaks === undefined && minutes === undefined) {
    return { refused: { problem: t("time.sheet.cannot-clear") } };
  }
  const zone = zoneOfDay(current, input.region);
  const canRun = date === input.today;
  const held = input.held[date];
  const planned = planWorkDayEdit({
    day: current,
    date,
    zone,
    field,
    minutes: minutes ?? 0,
    breaks,
    canRun,
    held,
  });
  if ("problem" in planned) {
    return { refused: { problem: t(planned.problem) } };
  }
  if ("held" in planned) {
    return planned;
  }
  return hasTimes(current, planned.times)
    ? { refused: { unchanged: true } }
    : {
        change: {
          date,
          version: current?.version ?? 0,
          times: planned.times,
          ...whatChanges(field, current, planned.times, { zone, region: input.region }),
        },
      };
}

/** The start or the end that changes, as the person's clock writes it; nothing for the breaks. */
function whatChanges(
  field: WorkDayField,
  current: WorkDay | undefined,
  times: Times,
  clock: { readonly zone: string; readonly region: Region },
): { readonly what?: Change } {
  if (field === "breaks") {
    return {};
  }
  const isStart = field === "start";
  const before = isStart ? startOf(current) : endOf(current);
  const after = isStart ? times.periods[0]?.start : times.periods.at(-1)?.end;
  return {
    what: {
      field,
      before: clockText(before, clock.zone, clock.region),
      after: clockText(after, clock.zone, clock.region),
    },
  };
}

/** The change that waits for its reason, and the starts that wait for their ends. */
function useWaiting() {
  const [pending, setPending] = useState<PendingReason | undefined>();
  const [revision, setRevision] = useState(0);
  // Read and written inside the turns of the queue, so a turn sees what the one before left.
  const held = useRef<Held>({});
  // Settles the turn that waits for the reason: the line of saves holds until then.
  const settle = useRef<(answer: Committed) => void>(doNothing);
  const done = (answer: Committed): void => {
    setPending(undefined);
    settle.current(answer);
    settle.current = doNothing;
  };
  return {
    pending,
    revision,
    held,
    /** Shows the window for this change and answers when it is saved or given up. */
    ask: (change: PendingReason) =>
      new Promise<Committed>((resolve) => {
        settle.current = resolve;
        setPending(change);
      }),
    /** Gives up what waits: the cells show the saved values again. */
    giveUp: () => {
      held.current = {};
      setRevision((was) => was + 1);
      done({});
    },
    /** The change was saved: only the start waiting for that day is done with. */
    finish: (date: string) => {
      held.current = withoutDate(held.current, date);
      done({});
    },
  };
}

/** What a typed work-day cell does: plan it, hold a start, save it, or ask for the reason first. */
function useEditDay(input: EditingInput, saving: Saving) {
  const { inLine, saveNow } = saving;
  const waiting = useWaiting();
  const day = useDayChange(input.person);
  const latest = useRef(input);
  useLayoutEffect(() => {
    latest.current = input;
  });
  const save = (change: PendingReason, reason?: string): Promise<Saved> =>
    saveNow(() => day.save({ ...change, reason }));
  const saveOrAsk = async (change: PendingReason, now: EditingInput): Promise<Committed> => {
    const own = now.person === undefined;
    if (reasonIsNeeded({ own, date: change.date, today: now.today })) {
      return waiting.ask(change);
    }
    const saved = await save(change);
    return !saved.ok && codeOf(saved.error) === REASON_NEEDED
      ? waiting.ask(change)
      : problemOf(saved);
  };
  const editDay = (
    date: string,
    field: WorkDayField,
    minutes: number | BreakTimes | undefined,
  ): Promise<Committed> =>
    inLine(async () => {
      await rendered();
      const now = latest.current;
      const plan = planDay({ ...now, held: waiting.held.current }, date, field, minutes);
      if ("refused" in plan) {
        return plan.refused;
      }
      if ("held" in plan) {
        waiting.held.current = { ...waiting.held.current, [date]: plan.held };
        return { note: t("time.sheet.start-held") };
      }
      waiting.held.current = withoutDate(waiting.held.current, date);
      return saveOrAsk(plan.change, now);
    });
  return { editDay, save, waiting };
}

/**
 * The work-day cells: a typed cell plans new times of the day and saves them, or asks for the
 * reason first where one is needed (a past day, or the day of somebody else; the server decides,
 * and when it answers that a reason is needed the window opens for that change). A start typed for
 * a day that cannot run waits for its end, so a past day is saved whole. A change that waits for
 * its reason holds the line of saves, so what is typed after it is made on what the server has.
 */
export function useDayCells(input: EditingInput, saving: Saving) {
  const { editDay, save, waiting } = useEditDay(input, saving);
  const confirmReason = async (reason: string): Promise<string | undefined> => {
    if (waiting.pending === undefined) {
      return undefined;
    }
    const saved = await save(waiting.pending, reason);
    if (saved.ok) {
      waiting.finish(waiting.pending.date);
      return undefined;
    }
    return messageOf(saved.error);
  };
  return {
    revision: waiting.revision,
    pending: waiting.pending,
    editDay,
    confirmReason,
    cancelReason: waiting.giveUp,
  };
}
