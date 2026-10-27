import type { Permissions } from "../../shared/permissions.ts";
import { createRegistry, useRegistry } from "./registry.ts";

/** What is shown of one thing quick search found. */
export interface QuickShown {
  /** Stable among the results of one source. */
  readonly id: string;
  /** What the person reads. */
  readonly title: string;
  /** A line under the title, such as where it sits. */
  readonly hint?: string;
}

/** What opening a result does: it goes to an address of the app, or it runs an action. */
export type QuickTarget =
  | { readonly path: string; readonly run?: never }
  | { readonly run: () => void; readonly path?: never };

/** One thing quick search found, with what happens when the person opens it. */
export type QuickResult = QuickShown & QuickTarget;

/** Who is asking: sources show only what this person may see. */
export interface QuickSearchAsker {
  readonly permissions: Permissions;
  /** Ends the session of this browser: the action *Sign out*. */
  readonly signOut: () => void;
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
  /**
   * Whether this source asks the server. Such a source is asked once the person has paused typing,
   * so a fast typist costs one request, not one for every key (M5-Q10); the others answer at once.
   */
  readonly paused?: boolean;
  /** Whether this text asks for this source first: issues, when the text looks like an issue ID (UX §6.2). */
  readonly firstFor?: (query: string) => boolean;
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
