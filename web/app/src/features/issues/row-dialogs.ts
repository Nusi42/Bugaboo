import { doNothing } from "@bugaboo/ui";
import { createContext, type ReactNode } from "react";

/** The issue of a row of the list, as an action of the row needs it. */
export interface RowIssue {
  readonly id: string;
  /** The readable ID (`WEB-142`). */
  readonly key: string;
  /** The short name of the project the issue is in. */
  readonly project: string;
}

/** What the window of an action of a row gets. */
export interface RowDialogProps {
  readonly issue: RowIssue;
  /** Closes the window. */
  readonly onClose: () => void;
}

/**
 * An action of a row of the issue list that opens a window, which another area adds (changing many issues at once
 * adds *Change status…*): a swipe on a phone brings it in at the row's end, and the row's *More* holds it (UX §3.3).
 */
export interface RowDialogEntry {
  /** Names the action in a test and in the order. */
  readonly id: string;
  readonly order: number;
  /** The words of the action, asked for when it is drawn. */
  readonly label: () => string;
  /** The window the action opens. */
  readonly dialog: (props: RowDialogProps) => ReactNode;
}

/** The window of an action of a row that is open, and the issue it is for. */
export interface OpenRowDialog {
  readonly entry: RowDialogEntry;
  readonly issue: RowIssue;
}

/**
 * Opens the window of an action of a row. The list holds the window, not the row, so it stays open while the rows are
 * drawn again or scrolled away (`RowDialogs`).
 */
export const RowDialogContext = createContext<(open: OpenRowDialog) => void>(doNothing);

let entries: readonly RowDialogEntry[] = [];

/** Adds an action to every row of the issue list; one with the same `id` replaces the old one. */
export function addRowDialog(entry: RowDialogEntry): void {
  const others = entries.filter((known) => known.id !== entry.id);
  entries = [...others, entry].toSorted((a, b) => a.order - b.order || a.id.localeCompare(b.id));
}

/** The actions added to the rows, in their order. */
export function rowDialogs(): readonly RowDialogEntry[] {
  return entries;
}
