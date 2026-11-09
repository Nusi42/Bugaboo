import type { Schema } from "@bugaboo/api-client";

/** A status the issue can move to when the result is accepted. */
export interface AcceptChoice {
  readonly value: string;
  readonly label: string;
  /** The workflow proposes this status: the first of the group *Done* that it offers from the issue's status. */
  readonly proposed: boolean;
}

/**
 * The statuses the reviewer can move the issue to when they accept, from the status changes the
 * workflow offers from where the issue is: each status once, the ones of the group *Done* first, the
 * rest in the workflow's own order. The proposal is the first of the group *Done* in that order. This
 * is the one rule, written in `userDocs/agents/README.md`; *Accept by itself* (`move_after_accept` of
 * the server) uses the same list but moves only when it holds exactly one status of the group *Done*,
 * because nobody is asked there to choose between two (M7-F231 for a mark on the status that means finished).
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
  return [...done, ...rest].map((change, at) => ({
    value: change.to.key,
    label: change.to.name,
    proposed: at === 0 && change.to.status_group === "done",
  }));
}
