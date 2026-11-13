import { Alert, alertTone, sidesOf, type MenuAction, type SwipeSide } from "@bugaboo/ui";
import { useContext, useEffect, type ReactNode } from "react";

import { swipeMore, type RowSwipe } from "../../shared/use-row-swipe.tsx";
import {
  RowDialogContext,
  rowActionEntries,
  type RowActionEntry,
  type RowActionInput,
  type RowActionState,
} from "./row-actions.ts";

/** An entry with what its hook says for this row. */
type EntryState = readonly [RowActionEntry, RowActionState];

/** Calls the hook of one entry for the row, at the top of a component of its own, and hands on what it says. */
function OneEntry({
  entry,
  input,
  children,
}: {
  readonly entry: RowActionEntry;
  readonly input: RowActionInput;
  readonly children: (state: RowActionState) => ReactNode;
}) {
  return children(entry.useAction(input));
}

/** Calls the hook of every entry, one component each, and hands on what they all say, in the order of the entries. */
function AllEntries({
  entries,
  input,
  children,
}: {
  readonly entries: readonly RowActionEntry[];
  readonly input: RowActionInput;
  readonly children: (states: readonly EntryState[]) => ReactNode;
}) {
  const [first, ...rest] = entries;
  if (first === undefined) {
    return children([]);
  }
  return (
    <OneEntry key={first.id} entry={first} input={input}>
      {(state) => (
        <AllEntries entries={rest} input={input}>
          {(states) => children([[first, state], ...states])}
        </AllEntries>
      )}
    </OneEntry>
  );
}

/** The actions of the row by side, *More* with the side a swipe opened, and what the server refused, on the row. */
function RowActionsView({
  states,
  row,
  what,
  onSides,
}: {
  readonly states: readonly EntryState[];
  readonly row: RowSwipe;
  readonly what: string;
  readonly onSides: (sides: readonly SwipeSide[]) => void;
}) {
  const of = (side: RowActionEntry["side"]): MenuAction[] =>
    states.flatMap(([entry, state]) =>
      entry.side === side && state.action !== undefined ? [state.action] : [],
    );
  const actions = { start: of("start"), end: of("end") };
  const sides = sidesOf(actions);
  // The row's swipe opens only the sides that have actions now (a side waits for its read); the row keeps the sides
  // it has when they are the same, so this draws nothing again.
  useEffect(() => {
    onSides(sides);
  }, [sides, onSides]);
  return (
    <>
      {swipeMore(row, actions, what)}
      {states.flatMap(([entry, state]) =>
        state.problem === null
          ? []
          : [
              <Alert key={entry.id} tone={alertTone.danger}>
                {state.problem}
              </Alert>,
            ],
      )}
    </>
  );
}

export interface IssueRowActionsProps {
  readonly input: Omit<RowActionInput, "open">;
  readonly row: RowSwipe;
  /** Tells the row which sides a swipe may open; it keeps its own list while they are the same. */
  readonly onSides: (sides: readonly SwipeSide[]) => void;
}

/**
 * The actions of a row of the issue list on a phone (UX §3.3), as the areas added them (*Follow* at the start,
 * *Change status or priority…* at the end): a swipe brings in the first of a side, *More* holds them all.
 */
export function IssueRowActions({ input, row, onSides }: IssueRowActionsProps) {
  const openDialog = useContext(RowDialogContext);
  const full: RowActionInput = {
    ...input,
    open: (dialog) => {
      openDialog({ dialog, issue: input.issue });
    },
  };
  return (
    <AllEntries entries={rowActionEntries()} input={full}>
      {(states) => (
        <RowActionsView states={states} row={row} what={input.issue.key} onSides={onSides} />
      )}
    </AllEntries>
  );
}
