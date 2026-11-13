import { dataOf, type Schema } from "@bugaboo/api-client";
import { keepPreviousData, useInfiniteQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import type { TimelineOptions } from "../../shared/list-view.ts";
import { pageSize } from "../../shared/paged.ts";
import { known } from "../../shared/present.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import type { Span } from "./scale.ts";

/** A row of the timeline as the server sends it: a group, an issue, its bar. */
export type TimelineRow = Schema<"TimelineRowBody">;

/** The ID of the issue of a row. */
export function idOf(row: TimelineRow): string {
  return known(row.issue.id) ?? "";
}

/** A page of the timeline as the server sends it. */
type TimelinePage = Schema<"TimelinePageBody">;

/** A goal of the page: its due date and the days of its sub-issues. */
export type TimelineGoal = Schema<"TimelineGoalBody">;

/** A plan warning of an issue of the page, as the server words it for the reader. */
export type PlanWarning = Schema<"PlanWarningBody">;

/** What tells a warning from the others: an issue has at most one of each kind. */
export function warningKey(warning: PlanWarning): string {
  return `${warning.issue}/${warning.kind}`;
}

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
  /** The warnings of the issues read so far, each once. */
  readonly warnings: readonly PlanWarning[];
  /** How many issues of the search wait in *Not scheduled yet*. */
  readonly notScheduled: number;
  /** Whether the server planned the bars by effort (never for an access key). */
  readonly byEffort: boolean;
  /** What the parts of the chart look up in it: a goal, and the days of a person's row. */
  readonly lookups: {
    readonly goalOf: (issue: string) => TimelineGoal | undefined;
    /** Under *Group by person*: the away days and holidays of a person's row, by their ID. */
    readonly personDaysOf: (person: string) => readonly Schema<"HatchedDayBody">[];
  };
}

/** The pages put together: the rows in order, every arrow and goal once. */
export function readOf(pages: readonly TimelinePage[] | undefined): TimelineRead {
  const all = pages ?? [];
  const goals = new Map<string, TimelineGoal>();
  const seen = new Set<string>();
  const dependencies: Schema<"TimelineDependencyBody">[] = [];
  const warnings = new Map<string, PlanWarning>();
  const peopleDays = new Map<string, readonly Schema<"HatchedDayBody">[]>();
  for (const page of all) {
    for (const row of page.people_days) {
      if (row.person.shown === "name") {
        peopleDays.set(row.person.id, row.hatched);
      }
    }
    for (const warning of page.warnings) {
      warnings.set(warningKey(warning), warning);
    }
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
    warnings: [...warnings.values()],
    notScheduled: all[0]?.not_scheduled ?? 0,
    byEffort: all[0]?.planned_by_effort ?? false,
    lookups: {
      goalOf: (issue) => goals.get(issue),
      personDaysOf: (person) => peopleDays.get(person) ?? [],
    },
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
