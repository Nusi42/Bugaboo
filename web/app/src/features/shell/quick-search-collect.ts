import type { QuickResult, QuickSearchAsker, QuickSearchSource } from "./quick-search-sources.ts";

/** The results of one source under its heading. */
export interface ResultGroup {
  /** The ID of the source; with the ID of a result it names the result among all of them. */
  readonly id: string;
  readonly heading: string;
  readonly results: readonly QuickResult[];
  /** The server has yet to answer for the words now typed: these rows are of older words. */
  readonly busy?: boolean;
}

/**
 * The name of the first result that can be chosen: the first of the groups that are not busy (the
 * rows of a busy group are of older words). Nothing when there is none.
 */
export function firstEnabled(groups: readonly ResultGroup[]): string | undefined {
  return groups
    .filter((group) => group.busy !== true)
    .flatMap((group) => group.results.map((result) => resultKey(group, result)))[0];
}

/**
 * What the server found for older words while it is asked again: every group keeps its place and is
 * marked busy, so its rows cannot be chosen; once the answer is in, nothing is marked.
 */
export function markStale(found: Collected, waiting: boolean): Collected {
  return waiting
    ? { ...found, groups: found.groups.map((group) => ({ ...group, busy: true })) }
    : found;
}

/** The names of the results that cannot be chosen: the rows of a group that is still busy. */
export function busyKeys(groups: readonly ResultGroup[]): readonly string[] {
  return groups
    .filter((group) => group.busy === true)
    .flatMap((group) => group.results.map((result) => resultKey(group, result)));
}

/** What the sources found, and whether one of them could not answer. */
export interface Collected {
  readonly groups: readonly ResultGroup[];
  /** At least one source failed; the others are shown all the same. */
  readonly failed: boolean;
}

/** The name of a result among the results of every source: two sources may use the same ID. */
export function resultKey(group: ResultGroup, result: QuickResult): string {
  return `${group.id}:${result.id}`;
}

/** The result with this name among the results of every group, if there is one. */
export function resultByKey(
  groups: readonly ResultGroup[],
  key: string | number,
): QuickResult | undefined {
  for (const group of groups) {
    for (const result of group.results) {
      if (resultKey(group, result) === key) {
        return result;
      }
    }
  }
  return undefined;
}

/**
 * Asks every source. A source that fails (or answers with something that is not a list) leaves its
 * own results out and nothing else: one broken source must not take quick search down.
 */
export async function collectResults(
  sources: readonly QuickSearchSource[],
  query: string,
  asker: QuickSearchAsker,
): Promise<Collected> {
  const answers = await Promise.allSettled(
    sources.map(async (source): Promise<ResultGroup> => ({
      id: source.id,
      heading: source.heading,
      results: await source.search(query, asker),
    })),
  );
  const groups: ResultGroup[] = [];
  let failed = false;
  for (const answer of answers) {
    if (answer.status === "fulfilled" && answer.value.results.length > 0) {
      groups.push(answer.value);
    } else if (answer.status === "rejected") {
      failed = true;
    }
  }
  return { groups, failed };
}

/**
 * The results of several rounds of asking as one list, in the order of the sources, except that a
 * source that asks to come first for this text (issues, for a text that looks like an issue ID)
 * does. A source that answered in none of the rounds has no group.
 */
export function arrange(
  sources: readonly QuickSearchSource[],
  query: string,
  ...rounds: readonly Collected[]
): Collected {
  const rank = (id: string): number => {
    const index = sources.findIndex((source) => source.id === id);
    return sources[index]?.firstFor?.(query) === true ? index - sources.length : index;
  };
  const groups = rounds.flatMap((round) => round.groups);
  return {
    groups: groups.toSorted((a, b) => rank(a.id) - rank(b.id)),
    failed: rounds.some((round) => round.failed),
  };
}
