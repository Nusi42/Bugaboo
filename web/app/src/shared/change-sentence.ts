import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import type { Wording, WordedChange, WordedSide } from "@bugaboo/wasm";

import { fieldNameOf } from "./field-names.ts";
import { regionWords } from "./own-value-text.ts";
import { personName, personNameInside } from "./person.ts";
import { renderSaid } from "./said.ts";
import { factorText } from "./time-factor-text.ts";

/**
 * What the sentence of a change needs of an item that says one: the history of an issue and a
 * notification both carry these fields, so both word a change the same way.
 */
export interface ChangeWords {
  readonly by?: Schema<"HistoryEntryItem">["by"];
  /** The AI app the person worked through, when one made the change. */
  readonly app?: string | null | undefined;
  /** The agent that did it, when one did; `by` is then the person responsible for it. */
  readonly agent?: Schema<"HistoryEntryItem">["agent"];
  readonly change?: Schema<"FieldChangeBody"> | null | undefined;
}

/**
 * Who did it, at the start of a sentence; Bugaboo itself when nobody is named. A change an agent made
 * reads "Fixer, agent of Ada Brandt", and only for a reader who is told the person behind it (ADR-0035:
 * otherwise the agent is not named and the sentence says what it says of the person); a change made
 * through an AI app reads "Ada Brandt via Claude Code" (AI-23). The one place that words it, for the
 * history, the notices and the sentences of files and time.
 */
export function whoOf(entry: Pick<ChangeWords, "by" | "app" | "agent">): string {
  const person =
    entry.by === undefined || entry.by === null ? t("person.bugaboo") : personName(entry.by);
  if (entry.agent !== undefined && entry.agent !== null && entry.by?.shown === "name") {
    return t("person.agent-of", { agent: entry.agent.name, person });
  }
  return entry.app === undefined || entry.app === null || entry.app === ""
    ? person
    : t("person.via-app", { who: person, app: entry.app });
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
  if (value.shown === "time-factor") {
    return { shown: "plain", text: factorText(value.percent) };
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
