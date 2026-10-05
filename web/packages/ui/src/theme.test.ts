import { describe, expect, it } from "vitest";

import { isThemeChoice, resolveTheme } from "./theme.ts";

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
});

describe("thm_02 one light and one dark choice", () => {
  it("uses the light choice on a light device and the dark choice on a dark device", () => {
    const choices = { light: "paper", dark: "midnight" } as const;
    expect(resolveTheme("device", false, choices)).toBe("paper");
    expect(resolveTheme("device", true, choices)).toBe("midnight");
  });
});
