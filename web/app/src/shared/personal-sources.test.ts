import { ApiError } from "@bugaboo/api-client";
import { currentSource, defaultAppearance } from "@bugaboo/ui";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { saveProgress } from "./change-my-settings.ts";
import { mySettingsKey } from "./my-settings.ts";
import { forgetDeviceChoices, installPersonalSources } from "./personal-sources.ts";
import { queryClient } from "./query-client.ts";
import { currentKeysSource } from "./shortcuts-source.ts";
import { leaveSession } from "./session.ts";

/** A storage of the browser that is only a map, so a test can look at what was left in it. */
function fakeStorage(): Storage {
  const kept = new Map<string, string>();
  return {
    get length() {
      return kept.size;
    },
    clear: () => {
      kept.clear();
    },
    getItem: (key) => kept.get(key) ?? null,
    key: (index) => [...kept.keys()][index] ?? null,
    removeItem: (key) => {
      kept.delete(key);
    },
    setItem: (key, value) => {
      kept.set(key, value);
    },
  };
}

let storage: Storage;

beforeEach(() => {
  storage = fakeStorage();
  vi.stubGlobal("localStorage", storage);
  vi.stubGlobal("sessionStorage", fakeStorage());
  queryClient.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("cfg_04 the person's choices never reach the storage of the browser", () => {
  it("keeps a change in the page, and says not saved, while the server gives no settings", async () => {
    installPersonalSources();
    await queryClient
      .query({
        queryKey: mySettingsKey(),
        queryFn: () => Promise.reject(new ApiError(500, undefined)),
      })
      .catch(() => {
        // The answer is the failure the test wants in the cache.
      });
    currentSource().write({ ...defaultAppearance, theme: "midnight" });
    expect(currentSource().read().theme).toBe("midnight");
    expect(saveProgress.read().state).toBe("not-saved");
    expect(storage.length).toBe(0);
  });

  it("gives the same keys at every read while the server gives no settings", async () => {
    // A screen that reads them with useSyncExternalStore draws again without end otherwise.
    installPersonalSources();
    await queryClient
      .query({
        queryKey: mySettingsKey(),
        queryFn: () => Promise.reject(new ApiError(500, undefined)),
      })
      .catch(() => {
        // The answer is the failure the test wants in the cache.
      });
    const keys = currentKeysSource();
    expect(keys.read()).toBe(keys.read());
  });

  it("removes what the device kept before the settings lived on the server", () => {
    storage.setItem("bugaboo.appearance", "{}");
    storage.setItem("bugaboo.shortcut-keys", "{}");
    forgetDeviceChoices();
    expect(storage.length).toBe(0);
  });

  it("leaves nothing behind when the settings change while the page changes at sign-out", async () => {
    installPersonalSources();
    await leaveSession(() => {
      // An answer of the server that comes in while the sign-in page opens.
      queryClient.setQueryData(mySettingsKey(), { settings: [] });
      return Promise.resolve();
    });
    expect(storage.length).toBe(0);
  });
});
