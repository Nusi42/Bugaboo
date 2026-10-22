import type { Wording } from "@bugaboo/wasm";
import { useEffect, useSyncExternalStore } from "react";

/** What the shared wording has: the module is loading, it is there, or it could not be loaded. */
export type WordingState =
  | { readonly state: "loading" }
  | { readonly state: "failed" }
  | { readonly state: "ready"; readonly wording: Wording };

/** What the app knows of the module now. Every sentence on a page shares it. */
let current: WordingState = { state: "loading" };
let starting: Promise<unknown> | undefined;
const listeners = new Set<() => void>();

function become(next: WordingState): void {
  current = next;
  for (const listener of listeners) {
    listener();
  }
}

/** The module of `bb-wasm`, fetched the first time it is asked for. */
export async function loadWording(): Promise<Wording> {
  const { loadWording: load } = await import("@bugaboo/wasm");
  const wording = await load();
  become({ state: "ready", wording });
  return wording;
}

/** Starts the load once; a failure is the state *failed*, which *Try again* leaves. */
function start(): void {
  starting ??= loadWording()
    .catch(() => {
      become({ state: "failed" });
    })
    .finally(() => {
      starting = undefined;
    });
}

/** Tries the load again after it failed. */
export function retryWording(): void {
  become({ state: "loading" });
  start();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * The shared wording of changes and notifications (`bb-core`, through `bb-wasm`). The module is
 * loaded with `import()` the first time a sentence needs it, so a page without one never fetches
 * it; until then the caller shows a skeleton. The state is shared, so one *Try again* serves every
 * sentence on the page.
 */
export function useWording(): WordingState {
  const state = useSyncExternalStore(
    subscribe,
    () => current,
    () => current,
  );
  useEffect(() => {
    if (current.state === "loading") {
      start();
    }
  }, []);
  return state;
}
