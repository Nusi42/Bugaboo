/**
 * The WebAssembly of the browser (ADR-0011, ADR-0005): the same Rust the server runs, built by
 * `cargo xtask wasm` into `generated/` (never committed). This file is the one place that loads it.
 * It is imported with `import()` only when a preview, an editor or a sentence of a history or of a
 * notification needs it, so the module is not part of the first page (coding-standards section 6).
 */
import { markSafeHtml, type SafeHtml } from "@bugaboo/api-client";

import init, {
  change_sentence,
  file_sentence,
  initSync,
  length_on_clock,
  length_sentence,
  name_key,
  notification_sentences,
  read_length,
  read_time_entry,
  render,
  side_words,
} from "../generated/bb_wasm.js";
import type {
  Change as WordedChange,
  File as WordedFile,
  LengthRead,
  LengthTyped,
  Notice as WordedNotice,
  Said,
  Side as WordedSide,
} from "./wording.generated.ts";

export type {
  Change as WordedChange,
  File as WordedFile,
  LengthRead,
  LengthTyped,
  Notice as WordedNotice,
  Said,
  Side as WordedSide,
} from "./wording.generated.ts";

/** Where the module is. Vite turns this into the address of the built file. */
export const moduleLocation = new URL("../generated/bb_wasm_bg.wasm", import.meta.url);

/** Turns Markdown into safe HTML; `imageHosts` are the hosts of the setting `content.image-hosts`. */
export type MarkdownRenderer = (text: string, imageHosts: readonly string[]) => SafeHtml;

const renderer: MarkdownRenderer = (text, imageHosts) =>
  markSafeHtml(render(text, [...imageHosts]));

/**
 * The words of a change and of a notification, chosen by `bb-core` (`wording`), the code the server
 * words its e-mails with. The answer is message keys with the words for their placeholders; the
 * app looks the keys up in its own catalog.
 */
export interface Wording {
  /** The sentence of a change of one field: *Ben changed Priority from Normal to High*. */
  readonly changeSentence: (change: WordedChange) => Said;
  /** The sentence of a file that was added, deleted or brought back. */
  readonly fileSentence: (file: WordedFile) => Said;
  /** The sentences of a notification: one, or one for each field a change touched. */
  readonly notificationSentences: (notice: WordedNotice) => readonly Said[];
  /** One side of a change as people read it; `null` when the side holds no value. */
  readonly sideWords: (field: string, side: WordedSide) => Said | null;
  /** A length of time a person typed (`1:30`, `1.5`, `90m`), read as the server reads it: the minutes, or the sentence of the mistake. */
  readonly readLength: (typed: LengthTyped) => LengthRead;
  /** A length of one time entry a person typed: as `readLength`, at least a minute and at most a day. */
  readonly readTimeEntry: (typed: LengthTyped) => LengthRead;
  /** A length of whole minutes on a clock, as a grid and a running timer show it: `1:30`. */
  readonly lengthOnClock: (minutes: number) => string;
  /** A length of whole minutes in a sentence: *1 hour 30 minutes*. */
  readonly lengthSentence: (minutes: number) => Said;
}

/** Whether a value read from the answer of the module is words: a text, a list or a message. */
function isSaid(value: unknown): value is Said {
  if (typeof value === "string" || typeof value === "number") {
    return true;
  }
  if (Array.isArray(value)) {
    return value.every((part) => isSaid(part));
  }
  return (
    typeof value === "object" &&
    value !== null &&
    "key" in value &&
    typeof value.key === "string" &&
    "args" in value &&
    typeof value.args === "object"
  );
}

/**
 * What happens when the module refuses its input or gives words that are no words: the browser
 * and the Rust types differ, a defect. While the app is built for development, and under test,
 * it throws, so the defect is found at once. In production it reports the defect with
 * `console.error` (the end-to-end tests fail on that) and gives no words, so the rest of the page
 * keeps working.
 */
function broken(problem: unknown): never[] {
  if (import.meta.env.DEV) {
    throw problem instanceof Error ? problem : new Error(String(problem));
  }
  console.error("The wording of bb-core refused its input.", problem);
  return [];
}

/** The words in the answer of the module, which is JSON text; `null` for no words. */
function saidIn(json: string): Said | null {
  const parsed: unknown = JSON.parse(json);
  if (parsed === null || isSaid(parsed)) {
    return parsed;
  }
  broken(new Error("The wording of bb-core gave something that is no words."));
  return null;
}

/** The answer of the reader of lengths; anything else is a defect (see `broken`). */
function lengthReadIn(json: string): LengthRead {
  const parsed: unknown = JSON.parse(json);
  if (
    typeof parsed === "object" &&
    parsed !== null &&
    "minutes" in parsed &&
    (typeof parsed.minutes === "number" || parsed.minutes === null) &&
    "mistake" in parsed &&
    (parsed.mistake === null || isSaid(parsed.mistake))
  ) {
    return { minutes: parsed.minutes, mistake: parsed.mistake };
  }
  broken(new Error("The reader of lengths of bb-core gave something that is no answer."));
  return { minutes: null, mistake: null };
}

/** Asks the module; a refusal is a defect (see `broken`). */
function ask(question: () => string): string {
  try {
    return question();
  } catch (problem) {
    broken(problem);
    return "null";
  }
}

const wording: Wording = {
  changeSentence: (change) => saidIn(ask(() => change_sentence(JSON.stringify(change)))) ?? "",
  fileSentence: (file) => saidIn(ask(() => file_sentence(JSON.stringify(file)))) ?? "",
  notificationSentences: (notice) => {
    const parsed: unknown = JSON.parse(ask(() => notification_sentences(JSON.stringify(notice))));
    return Array.isArray(parsed) ? parsed.filter((part) => isSaid(part)) : [];
  },
  sideWords: (field, side) => saidIn(ask(() => side_words(JSON.stringify({ field, side })))),
  readLength: (typed) => lengthReadIn(ask(() => read_length(JSON.stringify(typed)))),
  readTimeEntry: (typed) => lengthReadIn(ask(() => read_time_entry(JSON.stringify(typed)))),
  lengthOnClock: (minutes) => length_on_clock(minutes),
  lengthSentence: (minutes) => saidIn(length_sentence(minutes)) ?? "",
};

let loading: Promise<void> | undefined;

/** Loads the module on the first call; every later call gets the same answer. */
function ready(): Promise<void> {
  loading ??= init({ module_or_path: moduleLocation }).then(() => {});
  // A failed load (the network) may be tried again the next time.
  loading.catch(() => {
    loading = undefined;
  });
  return loading;
}

/** Loads the module and gives the Markdown renderer. */
export function loadMarkdownRenderer(): Promise<MarkdownRenderer> {
  return ready().then(() => renderer);
}

/**
 * The key by which names are told apart, as the server tells them (`bb_core::names`): capitals and
 * the spaces inside a name do not make a different name. Two names with one key are one name.
 */
export type NameKey = (name: string) => string;

/** Loads the module and gives the rule of the server for the same name. */
export function loadNameKey(): Promise<NameKey> {
  return ready().then(() => name_key);
}

/** Loads the module and gives the wording of changes and notifications. */
export function loadWording(): Promise<Wording> {
  return ready().then(() => wording);
}

/** Starts the module from its bytes, without the network: for tests that run under Node. */
export function startMarkdownRenderer(read: () => BufferSource): MarkdownRenderer {
  initSync({ module: read() });
  return renderer;
}

/** Starts the module from its bytes, without the network: for tests that run under Node. */
export function startWording(read: () => BufferSource): Wording {
  initSync({ module: read() });
  return wording;
}

/** Starts the module from its bytes, without the network, and gives the rule of the same name: for tests that run under Node. */
export function startNameKey(read: () => BufferSource): NameKey {
  initSync({ module: read() });
  return name_key;
}
