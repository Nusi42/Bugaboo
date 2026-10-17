import type { Schema } from "@bugaboo/api-client";

import type { MenuAction } from "@bugaboo/ui";

import type { IssueValues } from "./issue-values.ts";
import type { NewIssueValues } from "./new-issue-form.ts";
import { choiceIdOf } from "./status-choices.ts";

/** A status change the person may make now, as the issue answer lists it. */
export type OfferedChange = Schema<"StatusChangeRef">;

/** A field a status change needs, with the kind of input the server says it needs. */
export type NeededField = Schema<"NeededFieldBody">;

/** Whether a status change needs something filled in before it can be made. */
export function needsAsking(change: OfferedChange): boolean {
  return change.needs.length > 0;
}

/**
 * The entries of the status list: one for each status change the person may make now, in the words
 * of its button. A change that needs something filled in goes to `ask`; any other is made at once.
 */
export function statusActions(
  changes: readonly OfferedChange[],
  react: {
    readonly ask: (change: OfferedChange) => void;
    readonly make: (change: OfferedChange) => void;
  },
): readonly MenuAction[] {
  return changes.map((change) => ({
    id: choiceIdOf(change),
    label: change.button,
    onAction: () => {
      if (needsAsking(change)) {
        react.ask(change);
      } else {
        react.make(change);
      }
    },
  }));
}

/**
 * Whether the description is among what a status change needs. It is long text with a lock of its
 * own, so the small form cannot take it: the person writes it on the issue first.
 */
export function needsTheDescription(needs: readonly NeededField[]): boolean {
  return needs.some((need) => need.kind === "description");
}

/**
 * What the status change is sent with: the status, and only the values of the fields it asked for.
 * A field that was left empty is not sent, and the server names it (*Must be filled in*).
 */
export function askValuesOf(
  to: Schema<"StatusRef">,
  needs: readonly NeededField[],
  filled: NewIssueValues,
): IssueValues {
  const own = Object.fromEntries(
    needs
      .filter((need) => need.kind === "own" && need.field in filled.fields)
      .map((need) => [need.field, filled.fields[need.field]]),
  );
  const people = needs.some((need) => need.kind === "assigned-to") && filled.assignedTo.length > 0;
  return {
    status: to,
    ...(people ? { assignedTo: filled.assignedTo } : {}),
    ...(Object.keys(own).length > 0 ? { own } : {}),
  };
}
