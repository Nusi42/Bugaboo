import type { QuickResult, QuickSearchAsker, QuickSearchSource } from "./quick-search-sources.ts";

/** The results of one source under its heading. */
export interface ResultGroup {
  /** The ID of the source; with the ID of a result it names the result among all of them. */
  readonly id: string;
  readonly heading: string;
  readonly results: readonly QuickResult[];
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
