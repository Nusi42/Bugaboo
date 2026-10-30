import { describe, expect, it } from "vitest";

import { projectPages } from "../../shared/project-pages.ts";
import { hasSettingsMenu, settingsStartAddress, showsSettingsMenu } from "./use-settings-menu.ts";

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

describe("ux_5 where the settings of a project start", () => {
  it("is the menu where a phone has one", () => {
    expect(settingsStartAddress("WEB", true)).toBe("/p/WEB/settings?menu");
    expect(settingsStartAddress("WEB", true, projectPages.labels)).toBe("/p/WEB/settings?menu");
  });
  it("is the first page the person may open where there is no menu", () => {
    expect(settingsStartAddress("WEB", false)).toBe("/p/WEB/settings");
    expect(settingsStartAddress("WEB", false, projectPages.labels)).toBe("/p/WEB/settings/labels");
  });
});
