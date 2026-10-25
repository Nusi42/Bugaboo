/**
 * The search language in the browser (ADR-0005, ADR-0011): the same Rust the server runs, built by
 * `cargo xtask wasm` into `generated/` (never committed) as a module of its own. This file is the
 * one place that loads it, and it is imported with `import()` when a search bar first opens, so
 * the module is not part of the first page and not of any page that has no search bar
 * (coding-standards section 6).
 * The calls are in `language.ts`; tests that run under Node start the module from its bytes with
 * `@bugaboo/wasm-search/testing`.
 */
import init from "../generated/bb_wasm_search.js";
import { type SearchLanguage, language } from "./language.ts";

export { inStringUnits, offsetsOf, type Offsets } from "./offsets.ts";
export type { SearchLanguage } from "./language.ts";
export type * from "./types.generated.ts";

/** Where the module is. Vite turns this into the address of the built file. */
export const moduleLocation = new URL("../generated/bb_wasm_search_bg.wasm", import.meta.url);

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
