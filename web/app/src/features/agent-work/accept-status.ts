import type { Schema } from "@bugaboo/api-client";

/** A status the issue can move to when the result is accepted. */
export interface AcceptChoice {
  readonly value: string;
  readonly label: string;
}

/**
 * The statuses the reviewer can move the issue to when they accept, from the status changes the
 * workflow offers: each status once, the ones of the group *Done* first (the workflow's proposal),
 * the rest in the workflow's own order.
 */
export function acceptChoices(
  changes: readonly Schema<"StatusChangeRef">[] | null | undefined,
): readonly AcceptChoice[] {
  const seen = new Set<string>();
  const unique = (changes ?? []).filter((change) => {
    if (seen.has(change.to.key)) {
      return false;
    }
    seen.add(change.to.key);
    return true;
  });
  const done = unique.filter((change) => change.to.status_group === "done");
  const rest = unique.filter((change) => change.to.status_group !== "done");
  return [...done, ...rest].map((change) => ({ value: change.to.key, label: change.to.name }));
}
