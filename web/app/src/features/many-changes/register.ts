import { lazy, Suspense, createElement, type ReactNode } from "react";

import { t } from "@bugaboo/messages";

import {
  addRowAction,
  addSelectionBar,
  type RowActionInput,
  type RowActionState,
  type RowDialogProps,
  type SelectionBarProps,
} from "../issues/index.ts";

// Loaded when issues are first ticked, so the first page stays small (coding-standards section 6).
const LazyBar = lazy(async () => {
  const loaded = await import("./ManyChangeBar.tsx");
  return { default: loaded.ManyChangeBar };
});

const LazyRowChange = lazy(async () => {
  const loaded = await import("./RowStatusChange.tsx");
  return { default: loaded.RowStatusChange };
});

/** *Change status or priority…* of a row, which loads itself when it is first chosen. */
function RowChange(props: RowDialogProps): ReactNode {
  return createElement(Suspense, { fallback: null }, createElement(LazyRowChange, props));
}

/** *Change status or priority…* on a row: it opens the window of changing many issues for that one issue. */
function useChangeOnRow({ open }: RowActionInput): RowActionState {
  return {
    action: {
      id: "change-status",
      label: t("issues.row-change-status"),
      onAction: () => {
        open(RowChange);
      },
    },
    problem: null,
  };
}

/** The bar of changing many issues at once, which loads itself when it first shows. */
function Bar(props: SelectionBarProps): ReactNode {
  return createElement(Suspense, { fallback: null }, createElement(LazyBar, props));
}

/**
 * Puts the bar of changing many issues at once under the list of issues (WP-08), and *Change status or priority…* on
 * its rows.
 */
export function registerManyChanges(): void {
  addSelectionBar({ id: "many-changes", order: 0, component: Bar });
  addRowAction({ id: "change-status", order: 0, side: "end", useAction: useChangeOnRow });
}
