import { describe, expect, it } from "vitest";

import { hasSettingsMenu, showsSettingsMenu } from "./use-settings-menu.ts";

describe("ux_5 when the settings of a project show their menu", () => {
  it("is for a phone that asks for it, where there are two pages or more", () => {
    expect(showsSettingsMenu(true, 2, "?menu")).toBe(true);
    expect(showsSettingsMenu(true, 5, "?menu")).toBe(true);
  });
  it("is not for a person with one page only: there is nothing to choose", () => {
    expect(hasSettingsMenu(1)).toBe(false);
    expect(showsSettingsMenu(true, 1, "?menu")).toBe(false);
    expect(showsSettingsMenu(true, 0, "?menu")).toBe(false);
  });
  it("is not for a computer, and not for a phone that did not ask", () => {
    expect(showsSettingsMenu(false, 3, "?menu")).toBe(false);
    expect(showsSettingsMenu(true, 3, "")).toBe(false);
  });
});
