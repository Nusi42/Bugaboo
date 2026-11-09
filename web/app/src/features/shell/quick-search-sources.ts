import type { RowStateInput } from "@bugaboo/ui";
import type { ReactNode } from "react";

import type { Permissions } from "../../shared/permissions.ts";
import { createRegistry, useRegistry } from "./registry.ts";

/**
 * How a result that is an issue looks like a row of the issue list (R9): the group of its status and
 * its priority tint and edge the row; the cells are the ones the list draws, which the issues area
 * makes, so the shell holds no second drawing of them.
 */
export interface QuickIssue extends RowStateInput {
  /** The status icon, the ID and the type icon, before the title. */
  readonly lead: ReactNode;
  /** The priority, after the title. */
  readonly end: ReactNode;
}

/** What is shown of one thing quick search found. */
export interface QuickShown {
  /** Stable among the results of one source. */
  readonly id: string;
  /** What the person reads. */
  readonly title: string;
  /** A line under the title, such as where it sits. */
  readonly hint?: string;
  /** The ID of the issue the result is, shown before its title as in every row of issues. */
  readonly issueId?: string;
  /** The issue the result is, drawn as a row of the list; none where the answer did not carry what the row needs. */
  readonly issue?: QuickIssue;
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
