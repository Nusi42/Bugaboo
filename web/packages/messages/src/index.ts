import { IntlMessageFormat } from "intl-messageformat";

import { messages, type MessageArguments, type MessageKey } from "./generated.ts";

export type { MessageKey } from "./generated.ts";

/** English is the only catalog for now; more languages come with UX-07. */
const locale = "en";

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
    formatter = new IntlMessageFormat(messages[key], locale);
    formatters.set(key, formatter);
  }
  const result = formatter.format<string>(values[0]);
  return Array.isArray(result) ? result.join("") : result;
}

/**
 * Whether `text` is the key of a message in this catalog. The server may name a key that this
 * version of the app does not have yet, so a key from outside is checked before it is used.
 */
export function isMessageKey(text: string): text is MessageKey {
  return Object.hasOwn(messages, text);
}
