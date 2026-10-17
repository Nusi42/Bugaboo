import { lazy, Suspense, createElement, type ReactNode } from "react";

import { addSelectionBar, type SelectionBarProps } from "../issues/index.ts";

// Loaded when issues are first ticked, so the first page stays small (coding-standards section 6).
const LazyBar = lazy(async () => {
  const loaded = await import("./ManyChangeBar.tsx");
  return { default: loaded.ManyChangeBar };
});

/** The bar of changing many issues at once, which loads itself when it first shows. */
function Bar(props: SelectionBarProps): ReactNode {
  return createElement(Suspense, { fallback: null }, createElement(LazyBar, props));
}

/** Puts the bar of changing many issues at once under the list of issues (WP-08). */
export function registerManyChanges(): void {
  addSelectionBar({ id: "many-changes", order: 0, component: Bar });
}
