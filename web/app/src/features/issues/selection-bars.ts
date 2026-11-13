import type { ReactNode } from "react";

import { orderedRegistry, type Ordered } from "../../shared/ordered-registry.ts";
import type { IssueSelection } from "./use-selection.ts";

/** What a bar that acts on the ticked issues of a list gets. */
export interface SelectionBarProps {
  /** The issues that are ticked, by their IDs. */
  readonly selection: IssueSelection;
  /** The project whose issues are listed: its short name. */
  readonly project: string;
}

/** A bar with the place among the others. */
export interface SelectionBarEntry extends Ordered {
  readonly component: (props: SelectionBarProps) => ReactNode;
}

const bars = orderedRegistry<SelectionBarEntry>();

/**
 * Puts a bar under the list of issues that shows while some issues are ticked: changing many
 * issues at once (WP-08) is one. A bar with the same `id` replaces the old one.
 */
export const addSelectionBar = bars.add;

/** The bars of the ticked issues, in their order. */
export const selectionBars = bars.all;
