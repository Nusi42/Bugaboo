import type { ReactNode } from "react";

import { orderedRegistry, type Ordered } from "../../shared/ordered-registry.ts";
import type { SavedSearch } from "./saved-searches.ts";

/** What an action beside a saved search gets: the search it stands by. */
export interface SavedSearchActionProps {
  readonly search: SavedSearch;
}

/** A button another area puts beside a saved search (the standing rule of an agent, M7 WP-12). */
export interface SavedSearchAction extends Ordered {
  readonly component: (props: SavedSearchActionProps) => ReactNode;
}

const entries = orderedRegistry<SavedSearchAction>();

/** Puts an action beside every saved search. An action with the same `id` replaces the old one. */
export const addSavedSearchAction = entries.add;

/** The actions beside a saved search, in the order they are drawn. */
export const savedSearchActions = entries.all;
