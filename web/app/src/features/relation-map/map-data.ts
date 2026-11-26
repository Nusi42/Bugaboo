import { dataOf } from "@bugaboo/api-client";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import type { MapAnswer } from "./layout.ts";

/**
 * Which relations a map walks and draws (*Kinds shown*, UX §4.15): the kinds of relation turned off
 * by their IDs, and whether *part of* is on. Nothing turned off is every kind, also one made later.
 */
export interface KindsShown {
  readonly hidden: readonly string[];
  readonly partOf: boolean;
}

/** Every kind shown, *part of* too: how a map starts. */
export const everyKind: KindsShown = { hidden: [], partOf: true };

/** *Kinds shown* of a map that keeps them while it is open, starting with every kind. */
export function useKindsShown(): readonly [KindsShown, (shown: KindsShown) => void] {
  return useState(everyKind);
}

/** The IDs of the kinds the server is asked to walk; none for every kind. */
function shownIds(shown: KindsShown, kinds: readonly string[]): readonly string[] | undefined {
  return shown.hidden.length === 0 ? undefined : kinds.filter((id) => !shown.hidden.includes(id));
}

/**
 * How many steps away a map walks: 1, 2 or 3, or none for *Everything*, which is the most the
 * search language walks (M10-Q11); the server knows that number, the screen does not.
 */
export type StepsAway = 1 | 2 | 3 | undefined;

/** The steps a person can choose, nearest first; the last is *Everything*. */
export const stepChoices: readonly StepsAway[] = [1, 2, 3, undefined];

/** What the map of an issue is asked for. */
export interface IssueMapAsk {
  /** The readable ID of the start. */
  readonly issue: string;
  readonly steps: StepsAway;
  readonly shown: KindsShown;
  /** The IDs of every kind of relation, once they are read; none until then. */
  readonly kinds: readonly string[] | undefined;
}

/** The map of an issue. A changed ask keeps the picture before it until the new answer is in. */
export function useIssueMap({ issue, steps, shown, kinds }: IssueMapAsk) {
  const ids = kinds === undefined ? undefined : shownIds(shown, kinds);
  // Turned off kinds can be named only once the kinds are read.
  const ready = shown.hidden.length === 0 || kinds !== undefined;
  return useQuery({
    queryKey: queryKeys.search.map(issue, steps ?? null, ids?.join(",") ?? null, shown.partOf),
    enabled: ready,
    placeholderData: keepPreviousData,
    queryFn: async (): Promise<MapAnswer> =>
      dataOf(
        await api.client.GET("/api/v1/issues/{issue}/map", {
          params: {
            path: { issue },
            query: {
              ...(steps === undefined ? {} : { steps }),
              ...(ids === undefined ? {} : { kinds: ids.join(",") }),
              ...(shown.partOf ? {} : { part_of: false }),
            },
          },
        }),
      ),
  });
}

/** What the map of a search is asked for. */
export interface SearchMapAsk {
  /** The canonical text of the search. */
  readonly text: string;
  /** The project it looks in (and the projects inside it); none for every project. */
  readonly within: string | undefined;
  readonly shown: KindsShown;
  readonly kinds: readonly string[] | undefined;
}

/** The map of a search: its results and the relations between them. */
export function useSearchMap({ text, within, shown, kinds }: SearchMapAsk) {
  const ids = kinds === undefined ? undefined : shownIds(shown, kinds);
  const ready = shown.hidden.length === 0 || kinds !== undefined;
  return useQuery({
    queryKey: queryKeys.search.map("", text, within ?? null, ids?.join(",") ?? null, shown.partOf),
    enabled: ready,
    placeholderData: keepPreviousData,
    queryFn: async (): Promise<MapAnswer> =>
      dataOf(
        await api.client.POST("/api/v1/map", {
          body: {
            search_text: text,
            ...(within === undefined ? {} : { project: within }),
            ...(ids === undefined ? {} : { kinds: [...ids] }),
            ...(shown.partOf ? {} : { part_of: false }),
          },
        }),
      ),
  });
}
