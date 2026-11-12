import { lazy, Suspense, createElement, type ReactNode } from "react";

import { t } from "@bugaboo/messages";

import {
  addRowDialog,
  addSelectionBar,
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

/** *Change status…* of a row, which loads itself when it is first chosen. */
function RowChange(props: RowDialogProps): ReactNode {
  return createElement(Suspense, { fallback: null }, createElement(LazyRowChange, props));
}

/** The bar of changing many issues at once, which loads itself when it first shows. */
function Bar(props: SelectionBarProps): ReactNode {
  return createElement(Suspense, { fallback: null }, createElement(LazyBar, props));
}

/** Puts the bar of changing many issues at once under the list of issues (WP-08), and *Change status…* on its rows. */
export function registerManyChanges(): void {
  addSelectionBar({ id: "many-changes", order: 0, component: Bar });
  addRowDialog({
    id: "change-status",
    order: 0,
    label: () => t("issues.row-change-status"),
    dialog: RowChange,
  });
}
