import { dataOf, type Schema } from "@bugaboo/api-client";
import { useQuery, useQueryClient } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { weekDaysOf, weekFirstOf } from "../../shared/clock.ts";
import { fieldList, itemsOf, pageSize, useAllPages, usePaged } from "../../shared/paged.ts";
import { useRegion } from "../../shared/region.ts";
import { timeKeys } from "../../shared/time-keys.ts";
import { useNow } from "../../shared/use-now.ts";
import { startersOf } from "./rows.ts";
import { todayOf } from "./sheet-model.ts";

type TimeEntry = Schema<"TimeEntryItem">;

/** The fields of an entry the timesheet shows: its issue, day, length, version and own fields. */
const ENTRY_FIELDS = fieldList<TimeEntry>([
  "id",
  "issue",
  "readable_id",
  "note",
  "billable",
  "issue_title",
  "date",
  "minutes",
  "own_fields",
  "version",
]);

/** The first date of the week of today, as the person counts weeks (what *This week* goes to). */
export function useThisWeek(): string {
  const region = useRegion();
  return weekFirstOf(todayOf(region, useNow()), region.firstDay);
}

/** The seven days of the week that starts on `first`, and its first and last date. */
export function rangeOf(first: string) {
  const days = weekDaysOf(first);
  return { days, from: first, to: days.at(-1) ?? first };
}

/**
 * The work days of a week: one's own, or another person's (*Working hours of others*), whose
 * route answers like a person that does not exist when the reader may not see them.
 */
export function useWorkDays(first: string, person: string | undefined) {
  const { from, to } = rangeOf(first);
  const own = useQuery(
    api.query.queryOptions(
      "get",
      "/api/v1/me/work-days",
      { params: { query: { from, to } } },
      { enabled: person === undefined },
    ),
  );
  const other = useQuery(
    api.query.queryOptions(
      "get",
      "/api/v1/people/{person}/work-days",
      { params: { path: { person: person ?? "" }, query: { from, to } } },
      { enabled: person !== undefined },
    ),
  );
  return person === undefined ? own : other;
}

/** One's own week compared with the time booked on issues (the footer), as the server works it out. */
export function useBookedWeek(first: string) {
  return api.query.useQuery("get", "/api/v1/me/work-days/booked", {
    params: { query: { date: first } },
  });
}

/** The person's own time entries of the week, every page of them. */
export function useWeekEntries(first: string) {
  const { from, to } = rangeOf(first);
  const list = usePaged({
    queryKey: timeKeys.entries(from, to),
    read: (after) =>
      api.client.GET("/api/v1/me/time-entries", {
        params: { query: { from, to, limit: pageSize.largest, fields: ENTRY_FIELDS, ...after } },
      }),
  });
  useAllPages(list);
  return {
    ...list,
    entries: itemsOf(list.data?.pages),
    complete: list.data !== undefined && !list.hasNextPage,
  };
}

/**
 * The entries of one issue on one day as the server has them now (the week read last), for a save
 * that is planned in its turn: what the row drew may be older than the save before it.
 */
export function useEntriesOnDay(
  first: string,
): (issue: string, date: string) => readonly TimeEntry[] {
  const client = useQueryClient();
  const { from, to } = rangeOf(first);
  return (issue, date) =>
    itemsOf(
      client.getQueryData<{ readonly pages: readonly { readonly items: readonly TimeEntry[] }[] }>(
        timeKeys.entries(from, to),
      )?.pages,
    ).filter((entry) => entry.issue === issue && entry.date === date);
}

/** The IDs of the starter fields (*Activity*, *Cost center*) by their marks, from the timer's answer. */
export function useStarters() {
  const timer = api.query.useQuery("get", "/api/v1/me/timer");
  return startersOf(timer.data?.starters);
}

/** The history of the one day that is open (it is read only when it is asked for). */
export function useDayHistory(date: string | undefined, person: string | undefined) {
  const found = useQuery({
    queryKey: timeKeys.history(person ?? "", date ?? ""),
    enabled: date !== undefined,
    queryFn: async () =>
      person === undefined
        ? dataOf(
            await api.client.GET("/api/v1/me/work-days/{date}/history", {
              params: { path: { date: date ?? "" } },
            }),
          )
        : dataOf(
            await api.client.GET("/api/v1/people/{person}/work-days/{date}/history", {
              params: { path: { person, date: date ?? "" } },
            }),
          ),
  });
  return found.data;
}
