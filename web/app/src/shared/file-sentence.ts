import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { whoOf } from "./change-sentence.ts";

/** What the sentence of a file needs of an item that says one: the history and a notification. */
export type FileWords = {
  readonly [Field in "by" | "file_event" | "file_name"]?:
    | Schema<"HistoryEntryItem">[Field]
    | undefined;
};

/** The sentences of each thing that can happen to a file: with its name, and without it. */
const fileMessages = {
  added: ["history.file-added", "history.file-added-unnamed"],
  deleted: ["history.file-deleted", "history.file-deleted-unnamed"],
  restored: ["history.file-restored", "history.file-restored-unnamed"],
} as const satisfies Record<Schema<"FileEvent">, readonly [string, string]>;

/**
 * The sentence of a file that was added, deleted or brought back: *Ben added the file jam.txt*.
 * Once the file is gone for good it has no name and the sentence says *a file*. An event this
 * version does not know is no "added": it says only that a file changed. The history of an issue
 * and a notification word a file the same way.
 */
export function fileSentence(entry: FileWords): string {
  const who = whoOf(entry);
  if (entry.file_event === undefined || entry.file_event === null) {
    return t("history.file-changed-unnamed", { who });
  }
  const [named, unnamed] = fileMessages[entry.file_event];
  return entry.file_name === undefined || entry.file_name === null
    ? t(unnamed, { who })
    : t(named, { who, name: entry.file_name });
}
