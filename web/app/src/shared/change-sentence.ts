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
export interface ChangeWords {
  readonly by?: Schema<"HistoryEntryItem">["by"];
  readonly change?: Schema<"FieldChangeBody"> | null | undefined;
}

/** Who did it, at the start of a sentence; Bugaboo itself when nobody is named. */
export function whoOf(entry: { readonly by?: ChangeWords["by"] | undefined }): string {
  return entry.by === undefined || entry.by === null ? t("person.bugaboo") : personName(entry.by);
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
  return {
    shown: "plain",
    text: value.shown === "project" ? value.project.name : value.issue.readable_id,
  };
}

/** A change of one field as the shared wording takes it. */
export function wordedChange(entry: ChangeWords): WordedChange {
  const code = entry.change?.field ?? "";
  return {
    who: whoOf(entry),
    field: code,
    field_name: fieldNameOf({ code, name: entry.change?.name }),
    before: wordedSideOf(entry.change?.before),
    after: wordedSideOf(entry.change?.after),
  };
}

/**
 * The sentence of a change of one field: *Ben changed Priority from Normal to High*. The words
 * are chosen by `bb-core`, the code that words the e-mails of the server.
 */
export function changeSentence(wording: Wording, entry: ChangeWords): string {
  return renderSaid(wording.changeSentence(wordedChange(entry)));
}
