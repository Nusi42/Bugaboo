import type { NameKey } from "@bugaboo/wasm";
import { useEffect, useState } from "react";

/** The rule once it has been loaded, so the next editor has it at once. */
let loaded: NameKey | undefined;

async function load(): Promise<NameKey> {
  const { loadNameKey } = await import("@bugaboo/wasm");
  loaded = await loadNameKey();
  return loaded;
}

/**
 * The rule of the server for *the same name* (capitals and the spaces inside a name do not make a
 * different name), from the module of `@bugaboo/wasm`, which is loaded when an editor that needs
 * it opens (coding-standards section 6). Nothing until it is there.
 */
export function useNameKey(): NameKey | undefined {
  const [key, setKey] = useState<NameKey | undefined>(loaded);
  useEffect(() => {
    let live = true;
    if (loaded === undefined) {
      void load().then((found) => {
        if (live) {
          setKey(() => found);
        }
      });
    }
    return () => {
      live = false;
    };
  }, []);
  return key;
}
