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
  notification_sentences,
  render,
  side_words,
} from "../generated/bb_wasm.js";
import type {
  Change as WordedChange,
  File as WordedFile,
  Notice as WordedNotice,
  Said,
  Side as WordedSide,
} from "./wording.generated.ts";

export type {
  Change as WordedChange,
  File as WordedFile,
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
}

/** Whether a value read from the answer of the module is words: a text, a list or a message. */
function isSaid(value: unknown): value is Said {
  if (typeof value === "string") {
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
