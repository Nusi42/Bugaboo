import { dataOf, type Schema } from "@bugaboo/api-client";
import { keepPreviousData, useInfiniteQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import type { TimelineOptions } from "../../shared/list-view.ts";
import { pageSize } from "../../shared/paged.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import type { Span } from "./scale.ts";

/** A row of the timeline as the server sends it: a group, an issue, its bar. */
export type TimelineRow = Schema<"TimelineRowBody">;

/** A page of the timeline as the server sends it. */
type TimelinePage = Schema<"TimelinePageBody">;

/** A goal of the page: its due date and the days of its sub-issues. */
export type TimelineGoal = Schema<"TimelineGoalBody">;

/** What a timeline is of: a search, where it looks, how its view is set, and the days on screen. */
export interface TimelineAsk {
  /** The canonical text of the search. */
  readonly text: string;
  /** The ID of the project it looks in; none looks in every project the person sees. */
  readonly within: string | undefined;
  readonly options: TimelineOptions;
  /** The search word of the field the rows are grouped by, if the timeline offers it. */
  readonly groupBy: string | null;
  readonly span: Span;
}

/** Which part of the timeline: the rows with their bars, or the tray *Not scheduled yet*. */
type Part = Schema<"TimelinePartBody">;

function keyOf(ask: TimelineAsk, part: Part) {
  const { options } = ask;
  return queryKeys.search.timeline(
    part,
    ask.text,
    ask.within ?? "",
    options.start_field,
    options.end_field,
    options.plan_by_effort ?? false,
    ask.groupBy ?? "",
    ask.span.first,
    ask.span.last,
  );
}

/** The pages of one part of the timeline, a page at a time: `fetchNextPage` is *Show more*. */
function useTimelinePart(ask: TimelineAsk, part: Part) {
  return useInfiniteQuery({
    queryKey: keyOf(ask, part),
    placeholderData: keepPreviousData,
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam }: { readonly pageParam: string | null }) =>
      dataOf(
        await api.client.POST("/api/v1/timeline", {
          body: {
            search_text: ask.text,
            ...(ask.within === undefined ? {} : { project: ask.within }),
            timeline: ask.options,
            ...(ask.groupBy === null ? {} : { group_by: ask.groupBy }),
            first: ask.span.first,
            last: ask.span.last,
            part,
            limit: pageSize.usual,
            ...(pageParam === null ? {} : { cursor: pageParam }),
          },
        }),
      ),
    getNextPageParam: (last: TimelinePage) => last.next_cursor ?? null,
  });
}

/** What the pages read so far hold together. */
export interface TimelineRead {
  readonly rows: readonly TimelineRow[];
  readonly dependencies: readonly Schema<"TimelineDependencyBody">[];
  readonly goals: ReadonlyMap<string, TimelineGoal>;
  /** How many issues of the search wait in *Not scheduled yet*. */
  readonly notScheduled: number;
}

/** The pages put together: the rows in order, every arrow and goal once. */
export function readOf(pages: readonly TimelinePage[] | undefined): TimelineRead {
  const all = pages ?? [];
  const goals = new Map<string, TimelineGoal>();
  const seen = new Set<string>();
  const dependencies: Schema<"TimelineDependencyBody">[] = [];
  for (const page of all) {
    for (const goal of page.goals) {
      goals.set(goal.id, goal);
    }
    for (const dependency of page.dependencies) {
      const key = `${dependency.first}>${dependency.then}`;
      if (!seen.has(key)) {
        seen.add(key);
        dependencies.push(dependency);
      }
    }
  }
  return {
    rows: all.flatMap((page) => page.rows),
    dependencies,
    goals,
    notScheduled: all[0]?.not_scheduled ?? 0,
  };
}

/** The rows of the timeline with their bars, the arrows and the goals. */
export function useTimelineRows(ask: TimelineAsk) {
  return useTimelinePart(ask, "rows");
}

/** The issues of the search that wait in *Not scheduled yet*. */
export function useNotScheduled(ask: TimelineAsk) {
  return useTimelinePart(ask, "not-scheduled");
}

/** Reads the timeline and its tray again: dates or dependencies changed. */
export function refreshTimeline(): Promise<void> {
  return queryClient.invalidateQueries({ queryKey: queryKeys.search.timeline() });
}
