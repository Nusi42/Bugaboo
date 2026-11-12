import { type Schema } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { personToday } from "../../shared/clock.ts";
import { addDays } from "../../shared/format.ts";
import { useRegion } from "../../shared/region.ts";
import { useNow } from "../../shared/use-now.ts";

/** A person's work times, as the server answers them for the reader. */
export type WorkTimes = Schema<"WorkTimesBody">;
/** One away time, as the reader sees it. */
export type Away = Schema<"AwayBody">;
/** One weekday of the usual work days. */
export type UsualDay = Schema<"UsualDayBody">;
/** The days of an away time, as a request names them. */
export type AwayDays = Schema<"AwayDaysBody">;

/** Whose work times a page shows: the person's own (`undefined`) or another person's, by ID. */
export type Whose = string | undefined;

/**
 * The days one read covers: a year, which is what the server takes in one read (`RANGE_DAYS_MAX`
 * is a year and a day). The page reads the year ahead and the year before.
 */
const YEAR_DAYS = 365;

/** A stretch of days to read, both ends included. */
interface Stretch {
  readonly from: string;
  readonly to: string;
}

/** The year from today on, and the year before today. */
export function yearsAround(today: string): { readonly coming: Stretch; readonly past: Stretch } {
  return {
    coming: { from: today, to: addDays(today, YEAR_DAYS - 1) },
    past: { from: addDays(today, -YEAR_DAYS), to: addDays(today, -1) },
  };
}

/**
 * The read of the work times of `whose` over a stretch: of the two routes, the one that is not about
 * `whose` is not asked.
 */
function useWorkTimesOver(whose: Whose, stretch: Stretch) {
  const query = { from: stretch.from, to: stretch.to };
  const ownOptions = api.query.queryOptions("get", "/api/v1/me/work-times", { params: { query } });
  const otherOptions = api.query.queryOptions("get", "/api/v1/people/{person}/work-times", {
    params: { path: { person: whose ?? "" }, query },
  });
  const own = useQuery({ ...ownOptions, enabled: whose === undefined });
  const other = useQuery({ ...otherOptions, enabled: whose !== undefined });
  return whose === undefined
    ? { read: own, key: ownOptions.queryKey }
    : { read: other, key: otherOptions.queryKey };
}

/** Whether what the cache holds is an answer of work times. */
export function isWorkTimes(value: unknown): value is WorkTimes {
  return typeof value === "object" && value !== null && "usual" in value && "away" in value;
}

/** What a page of work times reads: the year ahead (with everything else) and the away times of the year before. */
export interface WorkTimesRead {
  readonly coming: WorkTimes | undefined;
  readonly past: WorkTimes | undefined;
  readonly error: unknown;
  /** The key of the read of the year ahead, which the changes of the usual work days show in at once. */
  readonly comingKey: readonly unknown[];
}

/** The work times of `whose`: the year ahead and the year before, from the person's today. */
export function useWorkTimes(whose?: string): WorkTimesRead {
  const region = useRegion();
  const today = personToday(region, useNow());
  const { coming, past } = yearsAround(today);
  const ahead = useWorkTimesOver(whose, coming);
  const before = useWorkTimesOver(whose, past);
  return {
    coming: ahead.read.data,
    past: before.read.data,
    error: ahead.read.error ?? before.read.error,
    comingKey: ahead.key,
  };
}

/**
 * The away times as the page lists them: the coming ones first, from the next on, then the past
 * year's, the latest first. One that touches both years is listed once, with the coming ones.
 */
export function awayInOrder(
  coming: readonly Away[],
  past: readonly Away[],
): { readonly coming: readonly Away[]; readonly past: readonly Away[] } {
  const ahead = new Set(coming.map((away) => away.id));
  return {
    coming,
    past: past.filter((away) => !ahead.has(away.id)).toReversed(),
  };
}

/** The seven usual days with the one at `at` (Monday first) changed. */
export function withDay(
  usual: readonly UsualDay[],
  at: number,
  day: UsualDay,
): readonly UsualDay[] {
  return usual.map((each, index) => (index === at ? day : each));
}
