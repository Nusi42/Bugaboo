/// <reference types="vite/client" />
import { type Formatters, IntlMessageFormat } from "intl-messageformat";

import type { MessageArguments, MessageKey } from "./generated/types.ts";
import { textOf } from "./registry.ts";
// The texts the first page shows. Every other part comes with the chunk of its area: that area's
// entry imports `@bugaboo/messages/parts/<area>` before anything of it runs.
import "./generated/shared.ts";

export type { MessageKey } from "./generated/types.ts";

/** English is the only catalog for now; more languages come with UX-07. Dates and their words follow it. */
export const locale = "en";

/**
 * The language tag that writes the numbers in the texts (`{count, number}`, the `#` of a plural): the
 * person's number format, so a count in a sentence looks like the count beside it (M5 exit F-A5).
 * `undefined` is the device's own. The plural keeps the rules of `locale`, whatever the numbers look like.
 */
let numbersLocale: string | undefined;

/** The number formats made so far, by their options; they are made again when the numbers change. */
const numberFormats = new Map<string, Intl.NumberFormat>();

/** Writes the numbers of every text the way `next` does (the app's region says which). */
export function setNumbersLocale(next: string | undefined): void {
  if (next !== numbersLocale) {
    numbersLocale = next;
    numberFormats.clear();
    formatters.clear();
  }
}

/** The formats a message uses: the numbers of the person, the plural and the dates of the catalog. */
const formatOf: Formatters = {
  getNumberFormat: (
    _locales?: string | readonly string[],
    options?: Readonly<Intl.NumberFormatOptions>,
  ) => {
    const key = JSON.stringify(options ?? {});
    let format = numberFormats.get(key);
    if (format === undefined) {
      format = new Intl.NumberFormat(numbersLocale, options);
      numberFormats.set(key, format);
    }
    return format;
  },
  getDateTimeFormat: (
    locales?: string | readonly string[],
    options?: Readonly<Intl.DateTimeFormatOptions>,
  ) => new Intl.DateTimeFormat(locales, options),
  getPluralRules: (
    locales?: string | readonly string[],
    options?: Readonly<Intl.PluralRulesOptions>,
  ) => new Intl.PluralRules(locales, options),
};

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
  const formatter = formatterFor(key);
  return formatter === undefined ? missing(key) : joined(formatter.format<string>(values[0]));
}

/** The formatter of `key`, or nothing when its part is not loaded. */
function formatterFor(key: MessageKey): IntlMessageFormat | undefined {
  let formatter = formatters.get(key);
  if (formatter === undefined) {
    const text = textOf(key);
    if (text === undefined) {
      return undefined;
    }
    formatter = new IntlMessageFormat(text, locale, undefined, { formatters: formatOf });
    formatters.set(key, formatter);
  }
  return formatter;
}

/** The text a formatter made, which is a list of parts only for a message with markup. */
function joined(result: string | readonly string[]): string {
  return Array.isArray(result) ? result.join("") : String(result);
}

/**
 * The text for a key that arrives as data, with the words for its placeholders: the message keys
 * that the shared wording of `bb-core` picks (`bb-wasm`). A key that is not loaded is the defect
 * `t()` reports. The words are checked by the Rust side, which builds them from the typed
 * messages of the same catalog.
 */
export function textByKey(key: string, values: Readonly<Record<string, string | number>>): string {
  if (!isMessageKey(key)) {
    return missing(key);
  }
  const formatter = formatterFor(key);
  return formatter === undefined ? missing(key) : joined(formatter.format<string>({ ...values }));
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
