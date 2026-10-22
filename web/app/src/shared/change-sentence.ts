import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import type { Wording, WordedChange, WordedSide } from "@bugaboo/wasm";

import { fieldNameOf } from "./field-names.ts";
import { regionWords } from "./own-value-text.ts";
import { personName, personNameInside } from "./person.ts";
import { renderSaid } from "./said.ts";

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
  return entry.by === undefined || entry.by === null ? t("person.bugaboo") : personName(entry.by);
}

/** The value of one side of an item in the one shape of a change's value, if it has any. */
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

/**
 * One side of a change as the shared wording takes it. What depends on the region (a number, a day,
 * a moment) and the names of people are written here; the choice of words is `bb-core`'s.
 */
export function wordedSideOf(
  value: Schema<"ChangeValueBody"> | null | undefined,
): WordedSide | null {
  if (value === null || value === undefined) {
    return null;
  }
  if (value.shown === "text") {
    return { shown: "text", text: value.text };
  }
  if (value.shown === "names") {
    return { shown: "names", names: value.names };
  }
  if (value.shown === "people") {
    return { shown: "people", people: value.people.map((person) => personNameInside(person)) };
  }
  if (value.shown === "typed") {
    return {
      shown: "typed",
      kind: value.kind,
      value: value.value,
      worded: regionWords(value.kind, value.value) ?? null,
    };
  }
  return { shown: "plain", text: value.shown === "project" ? value.project.name : value.issue.key };
}

/** A change of one field as the shared wording takes it. */
export function wordedChange(entry: ChangeWords): WordedChange {
  return {
    who: whoOf(entry),
    field: entry.field ?? "",
    field_name: fieldNameOf({ code: entry.field ?? "", name: entry.field_name }),
    before: wordedSideOf(valueOf(entry, "before")),
    after: wordedSideOf(valueOf(entry, "after")),
  };
}

/**
 * The sentence of a change of one field: *Ben changed Priority from Normal to High*. The words
 * are chosen by `bb-core`, the code that words the e-mails of the server.
 */
export function changeSentence(wording: Wording, entry: ChangeWords): string {
  return renderSaid(wording.changeSentence(wordedChange(entry)));
}
