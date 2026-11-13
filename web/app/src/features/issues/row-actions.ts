import { doNothing, type MenuAction, type SwipeSide } from "@bugaboo/ui";
import { createContext, type ReactNode } from "react";

import { orderedRegistry, type Ordered } from "../../shared/ordered-registry.ts";

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

/** A window that an action of a row opens (*Change status or priority…*). */
export type RowDialog = (props: RowDialogProps) => ReactNode;

/** What the hook of an action of a row is given. */
export interface RowActionInput {
  readonly issue: RowIssue;
  /**
   * The row was touched or focused: an action that needs a read of its own (whether the person follows the issue)
   * reads it from now on, not for every row the list shows.
   */
  readonly awake: boolean;
  /** Opens a window for the issue; the list holds it, so it stays open while the rows are drawn again. */
  readonly open: (dialog: RowDialog) => void;
}

/** The action of one row, if it has one now, and what the server refused it the last time, in words. */
export interface RowActionState {
  readonly action: MenuAction | undefined;
  readonly problem: string | null;
}

/**
 * An action that another area adds to every row of the issue list (*Follow*, *Change status or priority…*): a swipe on
 * a phone brings it in at its side, and the row's *More* holds it (UX §3.3).
 */
export interface RowActionEntry extends Ordered {
  readonly side: Exclude<SwipeSide, "none">;
  /**
   * A hook: the action for one row. Every row calls the hooks of every entry in the same order, which holds because the
   * areas add their entries when the app starts, before the first list is drawn.
   */
  readonly useAction: (input: RowActionInput) => RowActionState;
}

const entries = orderedRegistry<RowActionEntry>();

/** Adds an action to every row of the issue list; one with the same `id` replaces the old one. */
export const addRowAction = entries.add;

/** The actions added to the rows, in their order. */
export const rowActionEntries = entries.all;

/** A window of an action of a row that is open, and the issue it is for. */
export interface OpenRowDialog {
  readonly dialog: RowDialog;
  readonly issue: RowIssue;
}

/** Opens the window of an action of a row in the list around it (`RowDialogs`). */
export const RowDialogContext = createContext<(open: OpenRowDialog) => void>(doNothing);
