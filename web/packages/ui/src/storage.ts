/**
 * Everything the app keeps in the storage of the browser, by what it is for. This is the one list
 * of keys: a key is never written anywhere else.
 */
export const storageKey = {
  appearance: "bugaboo.appearance",
  firstFrame: "bugaboo.first-frame",
  projectTree: "bugaboo.project-tree",
  recentChoices: "bugaboo.recent-choices",
  shortcutKeys: "bugaboo.shortcut-keys",
  sidebarNarrow: "bugaboo.sidebar-narrow",
} as const;

/** The name of something the app keeps in the storage of the browser. */
export type StorageKey = (typeof storageKey)[keyof typeof storageKey];

/** The storage of the browser, as far as the app needs it. */
export interface KeyStore {
  readonly getItem: (key: string) => string | null;
  readonly setItem: (key: string, value: string) => void;
  readonly removeItem: (key: string) => void;
}

/**
 * Where the keys are kept. Storage is passed as a getter because merely touching `localStorage`
 * can throw (blocked site data); a failing store is then the same as an empty one.
 */
export type StoreGetter = () => KeyStore;

/** The storage of the browser itself. */
export const browserStorage: StoreGetter = () => localStorage;

/** Whether the storage refused (blocked site data, no room): the one failure that is handled here. */
function refused(error: unknown): boolean {
  return error instanceof DOMException;
}

/** The value in a text of JSON; nothing when there is no text or it is not JSON. */
export function parseJson(text: string | null): unknown {
  if (text === null) {
    return undefined;
  }
  try {
    return JSON.parse(text);
  } catch (error) {
    if (error instanceof SyntaxError) {
      return undefined;
    }
    throw error;
  }
}

/** What was kept under the key; nothing when there is none or the storage is blocked. */
export function readStored(key: StorageKey, store: StoreGetter = browserStorage): string | null {
  try {
    return store().getItem(key);
  } catch (error) {
    if (refused(error)) {
      return null;
    }
    throw error;
  }
}

/** What was kept under the key as JSON; nothing when there is none, it is not JSON or the storage is blocked. */
export function readStoredJson(key: StorageKey, store: StoreGetter = browserStorage): unknown {
  return parseJson(readStored(key, store));
}

/** Keeps the text under the key. When the storage is blocked the choice lasts until the page is closed, which is all it was before. */
export function writeStored(
  key: StorageKey,
  text: string,
  store: StoreGetter = browserStorage,
): void {
  try {
    store().setItem(key, text);
  } catch (error) {
    if (!refused(error)) {
      throw error;
    }
  }
}

/** Removes what was kept under the key; nothing to do when the storage is blocked. */
export function forgetStored(key: StorageKey, store: StoreGetter = browserStorage): void {
  try {
    store().removeItem(key);
  } catch (error) {
    if (!refused(error)) {
      throw error;
    }
  }
}
