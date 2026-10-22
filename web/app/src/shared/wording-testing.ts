import { startWording } from "@bugaboo/wasm";

import moduleAsData from "../../../packages/wasm/generated/bb_wasm_bg.wasm?inline";

const BASE64_MARK = ";base64,";

/** The bytes of a module that Vite inlined as a data address. */
function bytesOf(address: string): Uint8Array<ArrayBuffer> {
  const text = atob(address.slice(address.indexOf(BASE64_MARK) + BASE64_MARK.length));
  return Uint8Array.from(text, (character) => character.codePointAt(0) ?? 0);
}

/**
 * The shared wording (`bb-core`, through `bb-wasm`) started from the built module, for tests. The
 * module comes inline, so no test reads a file. `cargo xtask wasm` builds the module first.
 */
export const testWording = startWording(() => bytesOf(moduleAsData));
