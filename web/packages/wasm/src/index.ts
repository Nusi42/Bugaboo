/**
 * The WebAssembly of the browser (ADR-0011, ADR-0005): the same Rust the server runs, built by
 * `cargo xtask wasm` into `generated/` (never committed). This file is the one place that loads it.
 * It is imported with `import()` only when a preview or an editor opens, so the module is not part
 * of the first page (coding-standards section 6).
 */
import { markSafeHtml, type SafeHtml } from "@bugaboo/api-client";

import init, { initSync, render } from "../generated/bb_wasm.js";

/** Where the module is. Vite turns this into the address of the built file. */
export const moduleLocation = new URL("../generated/bb_wasm_bg.wasm", import.meta.url);

/** Turns Markdown into safe HTML; `imageHosts` are the hosts of the setting `content.image-hosts`. */
export type MarkdownRenderer = (text: string, imageHosts: readonly string[]) => SafeHtml;

const renderer: MarkdownRenderer = (text, imageHosts) =>
  markSafeHtml(render(text, [...imageHosts]));

let loading: Promise<MarkdownRenderer> | undefined;

/** Loads the module on the first call; every later call gets the same answer. */
export function loadMarkdownRenderer(): Promise<MarkdownRenderer> {
  loading ??= init({ module_or_path: moduleLocation }).then(() => renderer);
  // A failed load (the network) may be tried again the next time.
  loading.catch(() => {
    loading = undefined;
  });
  return loading;
}

/** Starts the module from its bytes, without the network: for tests that run under Node. */
export function startMarkdownRenderer(read: () => BufferSource): MarkdownRenderer {
  initSync({ module: read() });
  return renderer;
}
