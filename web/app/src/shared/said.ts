import { textByKey } from "@bugaboo/messages";
import type { Said } from "@bugaboo/wasm";

/**
 * The words the shared wording of `bb-core` chose, as text: each message key is looked up in the
 * catalog of the app, and the words for its placeholders are written first. A count stays a number
 * for its placeholder, so a plural chooses by it; a text of digits stays text.
 */
export function renderSaid(said: Said): string {
  if (typeof said === "string") {
    return said;
  }
  if (typeof said === "number") {
    return String(said);
  }
  if (!("key" in said)) {
    return said.map((part) => renderSaid(part)).join("");
  }
  const words: Record<string, string | number> = {};
  for (const [name, part] of Object.entries(said.args)) {
    words[name] = typeof part === "number" ? part : renderSaid(part);
  }
  return textByKey(said.key, words);
}
