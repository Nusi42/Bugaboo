import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { changeValueText } from "./change-value.ts";
import { fieldNameOf } from "./field-names.ts";
import { personName } from "./person.ts";

/**
 * What the sentence of a change needs of an item that says one: the history of an issue and a
 * notification both carry these fields, so both word a change the same way.
 */
export type ChangeWords = Pick<
  Schema<"HistoryEntryItem">,
  | "by"
  | "field"
  | "field_name"
  | "field_kind"
  | "before_text"
  | "after_text"
  | "before_value"
  | "after_value"
  | "before_names"
  | "after_names"
  | "before_people"
  | "after_people"
  | "before_issue"
  | "after_issue"
  | "before_project"
  | "after_project"
>;

/** Who did it, at the start of a sentence; Bugaboo itself when nobody is named. */
export function whoOf(entry: { readonly by?: ChangeWords["by"] | undefined }): string {
  return entry.by === undefined || entry.by === null ? t("history.system") : personName(entry.by);
}

/** The value of one side of a history entry in the one shape of a change's value, if it has any. */
function valueOf(entry: ChangeWords, side: "before" | "after"): Schema<"ChangeValueBody"> | null {
  const before = side === "before";
  const text = before ? entry.before_text : entry.after_text;
  const value = before ? entry.before_value : entry.after_value;
  const names = before ? entry.before_names : entry.after_names;
  const people = before ? entry.before_people : entry.after_people;
  const issue = before ? entry.before_issue : entry.after_issue;
  const project = before ? entry.before_project : entry.after_project;
  if (text !== undefined && text !== null) {
    return { shown: "text", text };
  }
  if (
    value !== undefined &&
    value !== null &&
    entry.field_kind !== undefined &&
    entry.field_kind !== null
  ) {
    return { shown: "typed", kind: entry.field_kind, value };
  }
  if (names !== undefined && names !== null) {
    return { shown: "names", names };
  }
  if (people !== undefined && people !== null) {
    return { shown: "people", people };
  }
  if (project !== undefined && project !== null) {
    return { shown: "project", project };
  }
  return issue === undefined || issue === null ? null : { shown: "issue", issue };
}

/** What one side of a change shows: a word, the people, or the issue; nothing for no value. */
export function sideOf(entry: ChangeWords, side: "before" | "after"): string | undefined {
  return changeValueText(entry.field ?? "", valueOf(entry, side));
}

/** The sentence of a change of one field: *Ben changed Priority from Normal to High*. */
export function changeSentence(entry: ChangeWords): string {
  const who = whoOf(entry);
  const field = fieldNameOf({ code: entry.field ?? "", name: entry.field_name });
  const before = sideOf(entry, "before");
  const after = sideOf(entry, "after");
  if (before !== undefined && after !== undefined) {
    return t("history.changed", { who, field, before, after });
  }
  if (after !== undefined) {
    // People are told after a colon: a field such as *Assigned to* ends in the word that *set … to* needs.
    return entry.after_people === undefined || entry.after_people === null
      ? t("history.set", { who, field, after })
      : t("history.set-people", { who, field, after });
  }
  return before === undefined
    ? t("history.changed-long-text", { who, field })
    : t("history.cleared", { who, field });
}
