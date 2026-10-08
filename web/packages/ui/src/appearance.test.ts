import { describe, expect, it } from "vitest";

import {
  applyChoices,
  defaultAppearance,
  parseAppearance,
  storageSource,
  themeOf,
  type Appearance,
} from "./appearance.ts";
import type { KeyStore } from "./storage.ts";

const blocked = (): never => {
  throw new DOMException("blocked", "SecurityError");
};

/** Storage in memory; `failToWrite` makes saving fail, as blocked site data does. */
function memoryStore(failToWrite = false): KeyStore {
  const saved = new Map<string, string>();
  return {
    getItem: (key) => saved.get(key) ?? null,
    setItem: (key, value) => {
      if (failToWrite) {
        blocked();
      }
      saved.set(key, value);
    },
    removeItem: (key) => void saved.delete(key),
  };
}

/** A page that only remembers its attributes. */
function fakePage() {
  const attributes = new Map<string, string>();
  return {
    attributes,
    page: {
      setAttribute: (name: string, value: string) => void attributes.set(name, value),
      removeAttribute: (name: string) => void attributes.delete(name),
    },
  };
}

describe("thm_01 appearance storage", () => {
  it("falls back to the default when storage is empty, invalid or blocked", () => {
    expect(parseAppearance(null)).toEqual(defaultAppearance);
    expect(parseAppearance("not json")).toEqual(defaultAppearance);
    expect(parseAppearance('"neon"')).toEqual(defaultAppearance);
    expect(storageSource(blocked).read()).toEqual(defaultAppearance);
  });

  it("keeps the valid parts of a saved appearance and defaults the rest", () => {
    const saved = JSON.stringify({ theme: "paper", spacing: "wide", lessMotion: true, dark: "x" });
    expect(parseAppearance(saved)).toEqual({
      ...defaultAppearance,
      theme: "paper",
      lessMotion: true,
    });
  });

  it("remembers a change, tells the listeners and survives blocked storage", () => {
    const store = memoryStore();
    const source = storageSource(() => store);
    let heard = 0;
    const stop = source.subscribe(() => {
      heard += 1;
    });
    source.write({ ...defaultAppearance, theme: "midnight" });
    expect(heard).toBe(1);
    expect(storageSource(() => store).read()).toEqual({
      ...defaultAppearance,
      theme: "midnight",
    });
    stop();
    source.write(defaultAppearance);
    expect(heard).toBe(1);
    const blockedSource = storageSource(() => memoryStore(true));
    blockedSource.write({ ...defaultAppearance, theme: "dusk" });
    expect(blockedSource.read().theme).toBe("dusk");
  });
});

describe("thm_02 match my device with one light and one dark choice", () => {
  it("shows the light choice on a light device and the dark choice on a dark one", () => {
    const appearance: Appearance = { ...defaultAppearance, light: "paper", dark: "midnight" };
    expect(themeOf(appearance, false)).toBe("paper");
    expect(themeOf(appearance, true)).toBe("midnight");
    expect(themeOf({ ...appearance, theme: "dusk" }, false)).toBe("dusk");
  });
});

describe("thm_06 spacing, text size and motion", () => {
  it("puts each choice on the page as the generated CSS expects, and takes it off again", () => {
    const { attributes, page } = fakePage();
    applyChoices(
      { ...defaultAppearance, spacing: "compact", largerText: true, lessMotion: true },
      page,
    );
    expect(Object.fromEntries(attributes)).toEqual({
      "data-spacing": "compact",
      "data-text": "larger",
      "data-motion": "less",
    });
    applyChoices(defaultAppearance, page);
    expect(attributes.size).toBe(0);
  });
});
