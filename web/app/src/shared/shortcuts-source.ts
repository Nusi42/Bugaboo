import { parseOverrides, type ShortcutOverrides } from "./shortcuts.ts";

/**
 * Where a person's own shortcut keys come from and go to. The app reads them from one source only,
 * so the place can change (this browser now, the server with *My settings*, WP-07) without
 * touching a screen.
 */
export interface ShortcutKeysSource {
  readonly read: () => ShortcutOverrides;
  readonly write: (next: ShortcutOverrides) => void;
  /** Calls `listener` after every change; returns the way to stop. */
  readonly subscribe: (listener: () => void) => () => void;
}

/** The storage of the browser, as far as the shortcut keys need it. */
export interface KeysStore {
  readonly getItem: (key: string) => string | null;
  readonly setItem: (key: string, value: string) => void;
}

const storageKey = "bugaboo.shortcut-keys";

/**
 * The keys kept on this device. Storage is passed as a getter because merely touching
 * `localStorage` can throw (blocked site data); then the keys last until the page is closed.
 */
export function storageKeysSource(getStore: () => KeysStore): ShortcutKeysSource {
  const listeners = new Set<() => void>();
  let current: ShortcutOverrides | null = null;
  return {
    read: () => {
      if (current === null) {
        try {
          current = parseOverrides(getStore().getItem(storageKey));
        } catch {
          current = {};
        }
      }
      return current;
    },
    write: (next) => {
      current = next;
      try {
        getStore().setItem(storageKey, JSON.stringify(next));
      } catch {
        // The keys then only last until the page is closed.
      }
      for (const listener of listeners) {
        listener();
      }
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

let source: ShortcutKeysSource = storageKeysSource(() => localStorage);

/** WP-07 calls this with a source that keeps the keys with the person's other settings. */
export function setShortcutKeysSource(next: ShortcutKeysSource): void {
  source = next;
}

/** The source the app reads the keys from. */
export function currentKeysSource(): ShortcutKeysSource {
  return source;
}
