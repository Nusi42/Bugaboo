import { dataOf, type Schema } from "@bugaboo/api-client";
import { keepPreviousData, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import type { CalendarOptions } from "../../shared/list-view.ts";
import { known } from "../../shared/present.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** The issues of a window of days as the server sends them. */
type CalendarPage = Schema<"CalendarPageBody">;

/** An issue on its day, as the server sends it. */
export type CalendarEntry = Schema<"CalendarEntryBody">;

/** The due date of a goal, a mark that is not dragged. */
export type GoalMark = Schema<"CalendarMarkBody">;

/** The ID of the issue of an entry. */
export function idOf(entry: CalendarEntry): string {
  return known(entry.issue.id) ?? "";
}

/** What a calendar is of: a search, where it looks, how its view is set, and the days on screen. */
export interface CalendarAsk {
  /** The canonical text of the search. */
  readonly text: string;
  /** The ID of the project it looks in; none looks in every project the person sees. */
  readonly within: string | undefined;
  readonly options: CalendarOptions;
  readonly first: string;
  readonly last: string;
}

/** The issues of the window of days, each on its day, and the goals' due dates as marks. */
export function useCalendarRead(ask: CalendarAsk) {
  const { options } = ask;
  return useQuery({
    queryKey: queryKeys.search.calendar(
      ask.text,
      ask.within ?? "",
      options.dates_field,
      ask.first,
      ask.last,
    ),
    placeholderData: keepPreviousData,
    queryFn: async (): Promise<CalendarPage> =>
      dataOf(
        await api.client.POST("/api/v1/calendar", {
          body: {
            search_text: ask.text,
            ...(ask.within === undefined ? {} : { project: ask.within }),
            calendar: options,
            first: ask.first,
            last: ask.last,
          },
        }),
      ),
  });
}

/** Reads the calendar again: dates changed. */
export function refreshCalendar(): Promise<void> {
  return queryClient.invalidateQueries({ queryKey: queryKeys.search.calendar() });
}
