/**
 * For tests that run under Node: the module started from its bytes, without the network. The
 * product loads it with `loadSearchLanguage` and nothing else.
 */
import { initSync } from "../generated/bb_wasm_search.js";
import { type SearchLanguage, language } from "./language.ts";

/** Starts the module from its bytes. */
export function startSearchLanguage(read: () => BufferSource): SearchLanguage {
  initSync({ module: read() });
  return language;
}
