import type { ReactNode } from "react";

import type { IssueView } from "./slots.ts";

/** What the window of an entry of *More* gets. */
export interface MoreDialogProps {
  /** The issue as the person sees it. */
  readonly issue: IssueView;
  /** Closes the window. */
  readonly onClose: () => void;
  /** Tells the page what refused the window's work: the window is gone by then, the page says it. */
  readonly onProblem: (error: unknown) => void;
}

/**
 * An entry of *More* that a package adds to the issue page (like a block of a slot): its words and
 * the window it opens. The page owns the menu and the place of the window.
 */
export interface MoreEntry {
  /** Names the entry in a test and in the order. */
  readonly id: string;
  readonly order: number;
  /** The words of the entry, asked for when the menu is drawn. */
  readonly label: () => string;
  /** The window the entry opens. */
  readonly dialog: (props: MoreDialogProps) => ReactNode;
}

let entries: readonly MoreEntry[] = [];

/** Adds an entry to *More*; one with the same `id` replaces the old one. */
export function addMoreEntry(entry: MoreEntry): void {
  const others = entries.filter((known) => known.id !== entry.id);
  entries = [...others, entry].toSorted((a, b) => a.order - b.order || a.id.localeCompare(b.id));
}

/** The entries added to *More*, in their order. */
export function moreEntries(): readonly MoreEntry[] {
  return entries;
}
