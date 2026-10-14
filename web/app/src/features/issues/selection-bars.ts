import type { ReactNode } from "react";

import type { IssueSelection } from "./use-selection.ts";

/** What a bar that acts on the ticked issues of a list gets. */
export interface SelectionBarProps {
  /** The issues that are ticked, by their IDs. */
  readonly selection: IssueSelection;
}

/** A bar with the place among the others. */
export interface SelectionBarEntry {
  /** Names the bar in a test and in the order. */
  readonly id: string;
  readonly order: number;
  readonly component: (props: SelectionBarProps) => ReactNode;
}

let bars: readonly SelectionBarEntry[] = [];

/**
 * Puts a bar under the list of issues that shows while some issues are ticked: changing many
 * issues at once (WP-08) is one. A bar with the same `id` replaces the old one.
 */
export function addSelectionBar(entry: SelectionBarEntry): void {
  const others = bars.filter((known) => known.id !== entry.id);
  bars = [...others, entry].toSorted((a, b) => a.order - b.order || a.id.localeCompare(b.id));
}

/** The bars of the ticked issues, in their order. */
export function selectionBars(): readonly SelectionBarEntry[] {
  return bars;
}
