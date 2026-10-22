import { fieldWords, type Conflict } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import type { Wording } from "@bugaboo/wasm";

import { renderSaid } from "../said.ts";
import { loadWording } from "../use-wording.ts";
import { formatAgo } from "../format.ts";
import { listText } from "../list-text.ts";
import { personNameInside, someoneName } from "../person.ts";

/** A value as people read it, or `undefined` when it has no short words (a long text, a list). */
function valueText(
  field: string,
  value: unknown,
  wording: Wording | undefined,
): string | undefined {
  if (typeof value !== "string") {
    return undefined;
  }
  if (field === "status" || field === "priority") {
    // The word of a status or a priority is chosen by `bb-core`; where it is not loaded, the code stands.
    const said = wording?.sideWords(field, { shown: "text", text: value });
    return said === null || said === undefined ? value : renderSaid(said);
  }
  // A long text is not quoted in a sentence: *Show changes* shows it.
  return field === "description" ? undefined : value;
}

/** One change as a part of a sentence: *Priority to Critical*, or only *the description*. */
function changeText(
  field: string,
  current: Conflict["current"],
  wording: Wording | undefined,
): string | undefined {
  const known = fieldWords.find((word) => word === field);
  if (known === undefined) {
    return undefined;
  }
  const name = t(`issue-field.${known}`);
  const value = valueText(known, current[known], wording);
  return value === undefined
    ? t("conflict.field-only", { field: name })
    : t("conflict.change", { field: name, value });
}

/** Who changed it first, in the middle of a sentence: a name, *a Team member* or *someone*. */
export function changedByName(conflict: Conflict): string {
  return conflict.changed_by === null || conflict.changed_by === undefined
    ? someoneName("inside")
    : personNameInside(conflict.changed_by);
}

/**
 * The sentence for a short value that somebody else changed first: *"Not saved: Ben changed Priority
 * to Critical 2 minutes ago."* The person is named as the reader may see them, the fields are the
 * ones the reader may see, and a long text is named without its words (ADR-0019, UX §4.4).
 */
export function conflictSentence(conflict: Conflict, nowMs: number, wording?: Wording): string {
  const who = changedByName(conflict);
  const at = conflict.changed_at ?? null;
  const timed = at !== null;
  const ago = at === null ? "" : formatAgo(at, nowMs);
  const changes = conflict.fields.flatMap(
    (field) => changeText(field, conflict.current, wording) ?? [],
  );
  return changes.length === 0
    ? t("conflict.sentence-plain", { who, timed: String(timed), ago })
    : t("conflict.sentence", { who, changes: listText(changes), timed: String(timed), ago });
}

/**
 * Like [`conflictSentence`], with the shared wording loaded first; where it cannot be loaded, the
 * sentence names a status or a priority by its code.
 */
export async function conflictSentenceWhenLoaded(
  conflict: Conflict,
  nowMs: number,
): Promise<string> {
  try {
    return conflictSentence(conflict, nowMs, await loadWording());
  } catch {
    return conflictSentence(conflict, nowMs);
  }
}
