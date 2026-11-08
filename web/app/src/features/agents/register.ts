import { createElement, lazy, Suspense } from "react";

import { addSavedSearchAction, type SavedSearchActionProps } from "../search/index.ts";

// Loaded when a saved search is first opened, so the first page stays small (coding-standards section 6).
const LazyButton = lazy(async () => {
  const loaded = await import("./RuleButton.tsx");
  return { default: loaded.RuleButton };
});

/** Puts *Make a standing rule* beside every saved search (UX §4.17). Called once when the app starts. */
export function registerAgents(): void {
  addSavedSearchAction({
    id: "standing-rule",
    order: 10,
    component: (props: SavedSearchActionProps) =>
      createElement(Suspense, { fallback: null }, createElement(LazyButton, props)),
  });
}
