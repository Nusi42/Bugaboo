import { createRegistry, useRegistry } from "./registry.ts";

/** One thing quick search found. */
export interface QuickResult {
  /** Stable among the results of one source. */
  readonly id: string;
  /** What the person reads. */
  readonly title: string;
  /** Where it goes: an address of the app. */
  readonly path: string;
  /** A line under the title, such as where it sits. */
  readonly hint?: string;
}

/** Who is asking: sources show only what this person may see. */
export interface QuickSearchAsker {
  readonly administrator: boolean;
}

/**
 * Somewhere quick search looks. The pages of the app are one source; projects come with WP-08,
 * and issues and actions with CFG-08 (M5), each by adding a source here. A source answers at
 * once or with a promise.
 */
export interface QuickSearchSource {
  readonly id: string;
  /** The heading above this source's results. */
  readonly heading: string;
  readonly search: (
    query: string,
    asker: QuickSearchAsker,
  ) => readonly QuickResult[] | Promise<readonly QuickResult[]>;
}

const sources = createRegistry<QuickSearchSource>();

/** Adds a source to quick search; returns the way to take it out again. */
export const addQuickSearchSource = sources.add;

export function useQuickSearchSources(): readonly QuickSearchSource[] {
  return useRegistry(sources);
}
