import { fieldWords, priorityWords, statusWords, type Conflict } from "@bugaboo/api-client";
import { locale, t } from "@bugaboo/messages";

import { formatAgo } from "../format.ts";
import { personName, someoneName } from "../person.ts";

/** A value as people read it, or `undefined` when it has no short words (a long text, a list). */
function valueText(field: string, value: unknown): string | undefined {
  if (typeof value !== "string") {
    return undefined;
  }
  if (field === "status") {
    const status = statusWords.find((word) => word === value);
    return status === undefined ? undefined : t(`status.${status}`);
  }
  if (field === "priority") {
    const priority = priorityWords.find((word) => word === value);
    return priority === undefined ? undefined : t(`priority.${priority}`);
  }
  // A long text is not quoted in a sentence: *Show changes* shows it.
  return field === "description" ? undefined : value;
}

/** One change as a part of a sentence: *Priority to Critical*, or only *the description*. */
function changeText(field: string, current: Conflict["current"]): string | undefined {
  const known = fieldWords.find((word) => word === field);
  if (known === undefined) {
    return undefined;
  }
  const name = t(`issue-field.${known}`);
  const value = valueText(known, current[known]);
  return value === undefined
    ? t("conflict.field-only", { field: name })
    : t("conflict.change", { field: name, value });
}

/** A list as a part of a sentence: "A", "A and B", "A, B and C". */
function listOf(items: readonly string[]): string {
  return new Intl.ListFormat(locale, { style: "long", type: "conjunction" }).format(items);
}

/**
 * The sentence for a short value that somebody else changed first: *"Ben changed Priority to
 * Critical 2 minutes ago."* The person is named as the reader may see them, the fields are the
 * ones the reader may see, and a long text is named without its words (ADR-0019, UX §4.4).
 */
export function conflictSentence(conflict: Conflict, nowMs: number): string {
  const who =
    conflict.changed_by === null || conflict.changed_by === undefined
      ? someoneName("start")
      : personName(conflict.changed_by);
  const ago =
    conflict.changed_at === null || conflict.changed_at === undefined
      ? ""
      : formatAgo(conflict.changed_at, nowMs);
  const changes = conflict.fields.flatMap((field) => changeText(field, conflict.current) ?? []);
  return changes.length === 0
    ? t("conflict.sentence-plain", { who, ago })
    : t("conflict.sentence", { who, changes: listOf(changes), ago });
}
