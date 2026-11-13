import type { ReactNode } from "react";

import { orderedRegistry, type Ordered } from "../../shared/ordered-registry.ts";
import type { IssueView } from "./slots.ts";

/** What the window of an entry of *More* gets. */
export interface MoreDialogProps {
  /** The issue as the person sees it. */
  readonly issue: IssueView;
  /** Closes the window. */
  readonly onClose: () => void;
}

/**
 * An entry of *More* that a package adds to the issue page (like a block of a slot): its words and
 * the window it opens. The page owns the menu and the place of the window.
 */
export interface MoreEntry extends Ordered {
  /** The words of the entry, asked for when the menu is drawn. */
  readonly label: () => string;
  /** The window the entry opens. */
  readonly dialog: (props: MoreDialogProps) => ReactNode;
}

const entries = orderedRegistry<MoreEntry>();

/** Adds an entry to *More*; one with the same `id` replaces the old one. */
export const addMoreEntry = entries.add;

/** The entries added to *More*, in their order. */
export const moreEntries = entries.all;
