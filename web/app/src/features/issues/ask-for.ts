import { fieldWords, type Schema } from "@bugaboo/api-client";

import type { IssueValues } from "./issue-values.ts";
import type { NewIssueValues } from "./new-issue-form.ts";

/** A status change the person may make now, as the issue answer lists it. */
export type OfferedChange = Schema<"StatusChangeRef">;

/** A field a status change asks for, by the kind of input it needs. */
export type AskedField =
  | { readonly kind: "description" }
  | { readonly kind: "assigned-to" }
  | { readonly kind: "own"; readonly field: string };

/** The key of a built-in field that is asked for in a small form. */
const assignedTo = "assigned-to" satisfies (typeof fieldWords)[number];
const description = "description" satisfies (typeof fieldWords)[number];

/** The key a field is asked for by: the word of a built-in field, or the ID of an own field. */
export function askKeyOf(asked: AskedField): string {
  return asked.kind === "own" ? asked.field : asked.kind;
}

/** Whether a status change needs something filled in before it can be made. */
export function needsAsking(change: OfferedChange): boolean {
  return change.needs.length > 0;
}

/** The fields a status change needs, each by the kind of input it takes. */
export function askedFieldsOf(needs: readonly string[]): readonly AskedField[] {
  return needs.map((key): AskedField => {
    if (key === description) {
      return { kind: "description" };
    }
    if (key === assignedTo) {
      return { kind: "assigned-to" };
    }
    return { kind: "own", field: key };
  });
}

/** Whether a key is the word of a built-in field: anything else is the ID of an own field. */
function isBuiltIn(key: string): boolean {
  return fieldWords.some((word) => word === key);
}

/**
 * What the status change is sent with: the status, and only the values of the fields it asked for.
 * A field that was left empty is not sent, and the server names it (*Must be filled in*).
 */
export function askValuesOf(
  to: Schema<"StatusRef">,
  needs: readonly string[],
  filled: NewIssueValues,
): IssueValues {
  const own = Object.fromEntries(
    needs
      .filter((key) => !isBuiltIn(key) && key in filled.fields)
      .map((key) => [key, filled.fields[key]]),
  );
  const people = needs.includes(assignedTo) && filled.assignedTo.length > 0;
  return {
    status: to,
    ...(people ? { assignedTo: filled.assignedTo } : {}),
    ...(Object.keys(own).length > 0 ? { own } : {}),
  };
}
