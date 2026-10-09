/**
 * The WebAssembly of the browser (ADR-0011, ADR-0005): the same Rust the server runs, built by
 * `cargo xtask wasm` into `generated/` (never committed). This file is the one place that loads it.
 * It is imported with `import()` only when a preview or an editor opens, so the module is not part
 * of the first page (coding-standards section 6).
 */
import init, { render } from "../generated/bb_wasm.js";
import moduleUrl from "../generated/bb_wasm_bg.wasm?url";

/** Turns Markdown into safe HTML; `imageHosts` are the hosts of the setting `content.image-hosts`. */
export type MarkdownRenderer = (text: string, imageHosts: readonly string[]) => string;

let loading: Promise<MarkdownRenderer> | undefined;

/** Loads the module on the first call; every later call gets the same answer. */
export function loadMarkdownRenderer(): Promise<MarkdownRenderer> {
  loading ??= init({ module_or_path: moduleUrl }).then(
    () => (text, imageHosts) => render(text, [...imageHosts]),
  );
  // A failed load (the network) may be tried again the next time.
  loading.catch(() => {
    loading = undefined;
  });
  return loading;
}
