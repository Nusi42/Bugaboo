import type { Packer } from "@bugaboo/wasm";
import { useEffect, useState } from "react";

/** The packing that was loaded, shared by every dashboard of the page. */
let loaded: Packer | null = null;

/** Loads the WebAssembly module and gives its packing. */
async function loadIt(): Promise<Packer> {
  const wasm = await import("@bugaboo/wasm");
  const packer = await wasm.loadPacker();
  loaded = packer;
  return packer;
}

/**
 * The packing of the server for the boxes of a dashboard (`bb_core::dashboard::pack`, through the WebAssembly module),
 * loaded when a dashboard is first changed: until it is there, nothing can be placed (`null`). A failed load is tried
 * again the next time a dashboard is changed.
 */
export function usePacker(wanted: boolean): Packer | null {
  const [packer, setPacker] = useState<Packer | null>(loaded);
  useEffect(() => {
    if (!wanted || packer !== null) {
      return () => {};
    }
    let live = true;
    void loadIt().then((ready: Packer) => {
      if (live) {
        setPacker(() => ready);
      }
    });
    return () => {
      live = false;
    };
  }, [wanted, packer]);
  return packer;
}
