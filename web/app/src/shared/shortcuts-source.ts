import { settingDefaults, settingKeys } from "@bugaboo/api-client";
import {
  browserStorage,
  forgetStored,
  readStored,
  storageKey,
  writeStored,
  type StoreGetter,
} from "@bugaboo/ui";

import type { MySettings } from "./my-settings.ts";
import { textOf } from "./setting-values.ts";
import { parseOverrides, type ShortcutOverrides } from "./shortcuts.ts";

/** The keys the person chose for shortcuts. */
export function shortcutKeysOf(settings?: MySettings): ShortcutOverrides {
  return parseOverrides(textOf(settings, settingKeys.shortcutsKeys, settingDefaults.shortcutsKeys));
}

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

/**
 * The keys kept on this device. Storage is passed as a getter because merely touching
 * `localStorage` can throw (blocked site data); then the keys last until the page is closed.
 */
export function storageKeysSource(getStore: StoreGetter = browserStorage): ShortcutKeysSource {
  const listeners = new Set<() => void>();
  let current: ShortcutOverrides | null = null;
  return {
    read: () => {
      current ??= parseOverrides(readStored(storageKey.shortcutKeys, getStore));
      return current;
    },
    write: (next) => {
      current = next;
      writeStored(storageKey.shortcutKeys, JSON.stringify(next), getStore);
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

/** Removes the keys this device kept (the ones of the pages before sign-in). */
export function forgetStoredKeys(): void {
  forgetStored(storageKey.shortcutKeys);
}

let source: ShortcutKeysSource = storageKeysSource();

/** WP-07 calls this with a source that keeps the keys with the person's other settings. */
export function setShortcutKeysSource(next: ShortcutKeysSource): void {
  source = next;
}

/** The source the app reads the keys from. */
export function currentKeysSource(): ShortcutKeysSource {
  return source;
}
