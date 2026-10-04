import { describe, expect, it } from "vitest";

import {
  defaultThemeChoice,
  isThemeChoice,
  readStoredChoice,
  resolveTheme,
  storeChoice,
} from "./theme.ts";

describe("thm_01 theme choice", () => {
  it("matches the device: Daylight on light, Dusk on dark", () => {
    expect(resolveTheme("device", false)).toBe("daylight");
    expect(resolveTheme("device", true)).toBe("dusk");
  });

  it("keeps an explicit theme whatever the device does", () => {
    expect(resolveTheme("paper", true)).toBe("paper");
    expect(resolveTheme("midnight", false)).toBe("midnight");
  });

  it("only accepts known choices", () => {
    expect(isThemeChoice("dusk")).toBe(true);
    expect(isThemeChoice("neon")).toBe(false);
    expect(isThemeChoice(null)).toBe(false);
  });

  it("falls back to the default when storage is empty, invalid or blocked", () => {
    expect(readStoredChoice(() => ({ getItem: () => null }))).toBe(defaultThemeChoice);
    expect(readStoredChoice(() => ({ getItem: () => "neon" }))).toBe(defaultThemeChoice);
    expect(
      readStoredChoice(() => ({
        getItem: () => {
          throw new Error("blocked");
        },
      })),
    ).toBe(defaultThemeChoice);
  });

  it("survives storage whose getter itself throws (blocked site data)", () => {
    const blocked = (): never => {
      throw new Error("blocked");
    };
    expect(readStoredChoice(blocked)).toBe(defaultThemeChoice);
    expect(() => {
      storeChoice(blocked, "paper");
    }).not.toThrow();
  });

  it("remembers a choice and survives blocked storage", () => {
    const saved = new Map<string, string>();
    storeChoice(() => ({ setItem: (key, value) => void saved.set(key, value) }), "paper");
    expect([...saved.values()]).toEqual(["paper"]);
    expect(() => {
      storeChoice(
        () => ({
          setItem: () => {
            throw new Error("blocked");
          },
        }),
        "paper",
      );
    }).not.toThrow();
  });
});
