import type { ThemeReader } from "@bugaboo/wasm";
import { useEffect, useState } from "react";

/** The rating once it has been loaded, so the next editor has it at once. */
let loaded: ThemeReader | undefined;

async function load(): Promise<ThemeReader> {
  const { loadThemeReader } = await import("@bugaboo/wasm");
  loaded = await loadThemeReader();
  return loaded;
}

/**
 * The readability of an own theme as the server rates a save (`bb-core`, through the module of
 * `@bugaboo/wasm`), loaded when the theme editor opens. Nothing until it is there; the server
 * checks again on save.
 */
export function useThemeReader(): ThemeReader | undefined {
  // A function in a state is its way of starting, so the reader is handed over inside one.
  const [reader, setReader] = useState<ThemeReader | undefined>(() => loaded);
  useEffect(() => {
    let live = true;
    if (loaded === undefined) {
      void (async () => {
        try {
          const found = await load();
          if (live) {
            setReader(() => found);
          }
        } catch {
          // The module could not be loaded: no rating on screen, and the server judges the save.
        }
      })();
    }
    return () => {
      live = false;
    };
  }, []);
  return reader;
}
