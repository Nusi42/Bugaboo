import { textByKey } from "@bugaboo/messages";
import type { Said } from "@bugaboo/wasm";

/**
 * The words the shared wording of `bb-core` chose, as text: each message key is looked up in the
 * catalog of the app, and the words for its placeholders are written first.
 */
export function renderSaid(said: Said): string {
  if (typeof said === "string") {
    return said;
  }
  if (!("key" in said)) {
    return said.map((part) => renderSaid(part)).join("");
  }
  const words: Record<string, string> = {};
  for (const [name, part] of Object.entries(said.args)) {
    words[name] = renderSaid(part);
  }
  return textByKey(said.key, words);
}
