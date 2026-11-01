import type { Schema } from "@bugaboo/api-client";
import type { Wording, WordedTime } from "@bugaboo/wasm";

import { whoOf } from "./change-sentence.ts";
import { renderSaid } from "./said.ts";

/** What the sentence of time needs of an item that says one: who did it, what happened and how long. */
export type TimeWords = {
  readonly [Field in "by" | "time_event" | "minutes"]?:
    | Schema<"HistoryEntryItem">[Field]
    | undefined;
};

/** What happened to time booked on an issue, as the shared wording takes it (`bb-core`, `wording::Time`). */
export function wordedTime(entry: TimeWords): WordedTime {
  return {
    who: whoOf(entry),
    what: { event: entry.time_event ?? null, minutes: entry.minutes ?? null },
  };
}

/**
 * The sentence of time that was booked, changed or deleted: *Ana booked 1h 30m*. The words and the
 * length are chosen by `bb-core`, which words the e-mails too; without the length (a reader who may
 * not read *Time spent*) it says only that time was booked.
 */
export function timeSentence(wording: Wording, entry: TimeWords): string {
  return renderSaid(wording.timeSentence(wordedTime(entry)));
}
