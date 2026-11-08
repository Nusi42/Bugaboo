import type { ReactNode } from "react";

import type { SavedSearch } from "./saved-searches.ts";

/** What an action beside a saved search gets: the search it stands by. */
export interface SavedSearchActionProps {
  readonly search: SavedSearch;
}

/** A button another area puts beside a saved search (the standing rule of an agent, M7 WP-12). */
export interface SavedSearchAction {
  /** Names the action in a test and in the order. */
  readonly id: string;
  /** Actions are drawn in this order, then by `id`. */
  readonly order: number;
  readonly component: (props: SavedSearchActionProps) => ReactNode;
}

let entries: readonly SavedSearchAction[] = [];

/** Puts an action beside every saved search. An action with the same `id` replaces the old one. */
export function addSavedSearchAction(entry: SavedSearchAction): void {
  entries = [...entries.filter((known) => known.id !== entry.id), entry].toSorted(
    (a, b) => a.order - b.order || a.id.localeCompare(b.id),
  );
}

/** The actions beside a saved search, in the order they are drawn. */
export function savedSearchActions(): readonly SavedSearchAction[] {
  return entries;
}
