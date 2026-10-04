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
