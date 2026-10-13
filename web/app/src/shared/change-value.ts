import type { Schema } from "@bugaboo/api-client";
import { isMessageKey, t } from "@bugaboo/messages";

import { listText } from "./list-text.ts";
import { ownValueText } from "./own-value-text.ts";
import { personNameInside } from "./person.ts";

/** A word of a status or a priority as people read it; any other text as it is. */
function wordOf(field: string, text: string): string {
  const key = field === "status" ? `status.${text}` : `priority.${text}`;
  return (field === "status" || field === "priority") && isMessageKey(key) ? t(key) : text;
}

/**
 * One side of a change as people read it: a word, names, people, a typed value, an issue or a
 * project; nothing when the side holds no value (so a list with nothing in it reads as no value).
 */
export function changeValueText(
  field: string,
  value: Schema<"ChangeValueBody"> | null | undefined,
): string | undefined {
  if (value === null || value === undefined) {
    return undefined;
  }
  if (value.shown === "text") {
    return wordOf(field, value.text);
  }
  if (value.shown === "names") {
    return value.names.length === 0 ? undefined : listText(value.names);
  }
  if (value.shown === "people") {
    return value.people.length === 0
      ? t("history.people-none")
      : listText(value.people.map((person) => personNameInside(person)));
  }
  if (value.shown === "typed") {
    return ownValueText(value.kind, value.value);
  }
  return value.shown === "issue" ? value.issue.key : value.project.name;
}
