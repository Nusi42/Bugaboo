import { textByKey } from "@bugaboo/messages";
import type { Mistake } from "@bugaboo/wasm-search";

/**
 * The words of a mistake of a search, in plain words. The search module makes the key of a mistake
 * (`search.mistake.*`), so no code names it as a string; it is named here by a template, which is how
 * the message catalog learns that only this feature shows these texts: they load with the search and
 * stay off the first page (ADR-0028). A key from another area is looked up as it is.
 */
export function mistakeText(mistake: Mistake): string {
  const [, name] = /^search\.mistake\.(.+)$/u.exec(mistake.key) ?? [];
  return textByKey(name === undefined ? mistake.key : `search.mistake.${name}`, mistake.args);
}
