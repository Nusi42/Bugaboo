/// <reference types="vite/client" />
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

/** The keys whose missing text was reported already: once per key is enough. */
const reported = new Set<string>();

/**
 * What `t()` does for a text whose part is not loaded (ADR-0028): a defect of the build, which
 * the person must never see as a key. While the app is built for development, and under test,
 * it throws, so the defect is found at once. In production it reports the key once with
 * `console.error` (the end-to-end tests fail on that) and shows nothing, so the rest of the page
 * keeps working. The vitest tests cannot find a missing part import: they load every part
 * (`everything.ts`). TXT-CATALOG checks that each area imports its part first.
 */
function missing(key: string): string {
  const problem = `The text ${key} is not loaded: its area does not import its message part.`;
  if (import.meta.env.DEV) {
    throw new Error(problem);
  }
  if (!reported.has(key)) {
    reported.add(key);
    console.error(problem);
  }
  return "";
}

/**
 * The text for `key`. This is the only way the app shows words: no literal text in components.
 * The values a message needs are checked at compile time.
 */
export function t<K extends MessageKey>(key: K, ...values: ValuesOf<K>): string {
  let formatter = formatters.get(key);
  if (formatter === undefined) {
    const text = textOf(key);
    if (text === undefined) {
      return missing(key);
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
