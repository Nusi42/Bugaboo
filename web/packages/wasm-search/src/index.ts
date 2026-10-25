/**
 * The search language in the browser (ADR-0005, ADR-0011): the same Rust the server runs, built by
 * `cargo xtask wasm` into `generated/` (never committed) as a module of its own. This file is the
 * one place that loads it, and it is imported with `import()` when a search bar first opens, so
 * the module is not part of the first page and not of any page that has no search bar
 * (coding-standards section 6).
 *
 * The module speaks JSON and counts bytes of the UTF-8 text. Here the places are turned into places
 * in the string (`offsets.ts`), and an input the module refuses is a defect of the build, which
 * throws: the browser and the Rust types differ.
 */
import init, {
  check,
  initSync,
  own_field_word,
  parse,
  print,
  suggest,
} from "../generated/bb_wasm_search.js";
import { inStringUnits, offsetsOf } from "./offsets.ts";
import type { Answer, FieldCatalogue, Search, Suggestions } from "./types.generated.ts";

export { offsetsOf, type Offsets } from "./offsets.ts";
export type * from "./types.generated.ts";

/** Where the module is. Vite turns this into the address of the built file. */
export const moduleLocation = new URL("../generated/bb_wasm_search_bg.wasm", import.meta.url);

/** The search language, as the server has it. Places are in units of the string. */
export interface SearchLanguage {
  /** What a text comes to without the fields: its tree and canonical text, or its first mistake. */
  readonly parse: (text: string) => Answer;
  /** The same with the fields the person may read: all mistakes of the checker. */
  readonly check: (text: string, catalogue: FieldCatalogue) => Answer;
  /** The canonical text of a tree, which Simple search edits. */
  readonly print: (search: Search) => string;
  /** What may come next at the cursor, a place in the string. */
  readonly suggest: (text: string, cursor: number, catalogue: FieldCatalogue) => Suggestions;
  /** The word of an own field, made from its name by the one rule of the language. */
  readonly ownFieldWord: (name: string) => string;
}

function isAnswer(value: unknown): value is Answer {
  return typeof value === "object" && value !== null && "mistakes" in value && "search" in value;
}

function isSuggestions(value: unknown): value is Suggestions {
  return typeof value === "object" && value !== null && "items" in value && "replace" in value;
}

/** The answer of the module for `text`, with places in units of the string. */
function answerFor(text: string, json: string): Answer {
  const answer: unknown = inStringUnits(JSON.parse(json), offsetsOf(text));
  if (!isAnswer(answer)) {
    throw new Error("The search language gave something that is no answer.");
  }
  return answer;
}

const language: SearchLanguage = {
  parse: (text) => answerFor(text, parse(text)),
  check: (text, catalogue) => answerFor(text, check(text, JSON.stringify(catalogue))),
  print: (search) => print(JSON.stringify(search)),
  suggest: (text, cursor, catalogue) => {
    const offsets = offsetsOf(text);
    const found: unknown = inStringUnits(
      JSON.parse(suggest(text, offsets.inBytes(cursor), JSON.stringify(catalogue))),
      offsets,
    );
    if (!isSuggestions(found)) {
      throw new Error("The search language gave something that is no suggestions.");
    }
    return found;
  },
  ownFieldWord: own_field_word,
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

/** Loads the module and gives the search language. */
export function loadSearchLanguage(): Promise<SearchLanguage> {
  return ready().then(() => language);
}

/** Starts the module from its bytes, without the network: for tests that run under Node. */
export function startSearchLanguage(read: () => BufferSource): SearchLanguage {
  initSync({ module: read() });
  return language;
}
