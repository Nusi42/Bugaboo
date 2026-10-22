import type { Schema } from "@bugaboo/api-client";
import type { Wording, WordedFile } from "@bugaboo/wasm";

import { whoOf } from "./change-sentence.ts";
import { renderSaid } from "./said.ts";

/** What the sentence of a file needs of an item that says one: the history and a notification. */
export type FileWords = {
  readonly [Field in "by" | "file_event" | "file_name"]?:
    | Schema<"HistoryEntryItem">[Field]
    | undefined;
};

/** What happened to a file, as the shared wording takes it (`bb-core`, `wording::File`). */
export function wordedFile(entry: FileWords): WordedFile {
  return {
    who: whoOf(entry),
    event: entry.file_event ?? null,
    name: entry.file_name ?? null,
  };
}

/**
 * The sentence of a file that was added, deleted or brought back: *Ben added the file jam.txt*.
 * The words are chosen by `bb-core`, which words the e-mails too.
 */
export function fileSentence(wording: Wording, entry: FileWords): string {
  return renderSaid(wording.fileSentence(wordedFile(entry)));
}
