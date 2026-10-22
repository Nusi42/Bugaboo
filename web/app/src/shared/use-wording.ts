import type { Wording } from "@bugaboo/wasm";
import { useEffect, useState } from "react";

/** What the shared wording has: the module is loading, it is there, or it could not be loaded. */
export type WordingState =
  | { readonly state: "loading" }
  | { readonly state: "failed" }
  | { readonly state: "ready"; readonly wording: Wording };

/** What has been loaded already, so a second sentence needs no waiting. */
let loaded: Wording | undefined;

/** The module of `bb-wasm`, fetched the first time it is asked for. */
export async function loadWording(): Promise<Wording> {
  const { loadWording: load } = await import("@bugaboo/wasm");
  loaded = await load();
  return loaded;
}

/**
 * The shared wording of changes and notifications (`bb-core`, through `bb-wasm`). The module is
 * loaded with `import()` the first time a sentence needs it, so a page without one never fetches
 * it; until then the caller shows a skeleton.
 */
export function useWording(): WordingState {
  const [state, setState] = useState<WordingState>(
    loaded === undefined ? { state: "loading" } : { state: "ready", wording: loaded },
  );
  useEffect(() => {
    let current = true;
    loadWording().then(
      (wording) => {
        if (current) {
          setState({ state: "ready", wording });
        }
      },
      () => {
        if (current) {
          setState({ state: "failed" });
        }
      },
    );
    return () => {
      current = false;
    };
  }, []);
  return state;
}
