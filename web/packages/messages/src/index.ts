import { IntlMessageFormat } from "intl-messageformat";

import type { MessageArguments, MessageKey } from "./generated/types.ts";
import { textOf } from "./registry.ts";
// The texts the first page shows. Every other part comes with the chunk of its area: that area's
// entry imports `@bugaboo/messages/parts/<area>` before anything of it runs.
import "./generated/shared.ts";

export type { MessageKey } from "./generated/types.ts";

/** English is the only catalog for now; more languages come with UX-07. Dates and numbers follow it. */
export const locale = "en";

type ValuesOf<K extends MessageKey> = K extends keyof MessageArguments
  ? [values: MessageArguments[K]]
  : [];

const formatters = new Map<MessageKey, IntlMessageFormat>();

/**
 * The text for `key`. This is the only way the app shows words: no literal text in components.
 * The values a message needs are checked at compile time.
 */
export function t<K extends MessageKey>(key: K, ...values: ValuesOf<K>): string {
  let formatter = formatters.get(key);
  if (formatter === undefined) {
    const text = textOf(key);
    if (text === undefined) {
      // Never show a person a key: a part that is not loaded is a defect in the build, found by the
      // tests (TXT-CATALOG checks that every area imports its part).
      throw new Error(`The text ${key} is not loaded: its area does not import its message part.`);
    }
    formatter = new IntlMessageFormat(text, locale);
    formatters.set(key, formatter);
  }
  const result = formatter.format<string>(values[0]);
  return Array.isArray(result) ? result.join("") : result;
}

/**
 * Whether `text` is the key of a message that is loaded. The server may name a key that this
 * version of the app does not have yet, so a key from outside is checked before it is used. Code
 * that looks keys up this way lives in the area whose part holds them, or the key is on the first
 * page, because no code names it as a string.
 */
export function isMessageKey(text: string): text is MessageKey {
  return textOf(text) !== undefined;
}
