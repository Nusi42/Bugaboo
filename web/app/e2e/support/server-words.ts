import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { repoRoot } from "../../../tooling/ports.ts";

/**
 * The words the server writes into the data of a new organization: the names of the starter
 * workflows and the texts of their buttons. The browser shows them as the API sends them and never
 * loads them, so a test reads them from the catalog the server reads (`messages/en.json`).
 */
const starterWords = [
  "workflow.simple",
  "workflow.bug",
  "workflow.development",
  "workflow-change.start-working",
  "workflow-change.stop-working",
  "workflow-change.mark-as-done",
  "workflow-change.reopen",
  "workflow-change.reopen-and-start",
  "workflow-change.send-to-testing",
  "workflow-change.back-to-work",
  "workflow-change.wont-fix",
  "workflow-change.ask-for-review",
] as const;

/** The key of a word the server writes. */
export type ServerWord = (typeof starterWords)[number];

let catalog: unknown;

/** The text of a word the server writes, in the language of the catalog. */
export function serverWord(key: ServerWord): string {
  catalog ??= JSON.parse(readFileSync(resolve(repoRoot, "messages/en.json"), "utf8"));
  const text: unknown =
    typeof catalog === "object" && catalog !== null ? Reflect.get(catalog, key) : undefined;
  if (typeof text !== "string") {
    throw new TypeError(`messages/en.json has no text ${key}.`);
  }
  return text;
}
